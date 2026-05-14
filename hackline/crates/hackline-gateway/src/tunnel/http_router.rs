//! Single shared HTTP listener that routes by `Host:` to the right
//! `kind = 'http'` tunnel and bridges the connection through the
//! same Zenoh byte tunnel used for raw TCP. WebSocket upgrades pass
//! through unchanged — we are not parsing HTTP framing, just peeking
//! the first request's `Host:` header off the wire to pick a route.
//!
//! Routing rule: the listener accepts a TCP connection, reads
//! request bytes until it has seen the first `Host:` header, then
//! matches the host against the `tunnels` table (`kind = 'http'`,
//! `public_hostname = <host>`). Bytes already read are forwarded
//! into the bridge before the socket halves are pumped freely.
//!
//! Keep-alive across different hostnames on a single TCP connection
//! is not supported — the matching `tunnels` row is selected once
//! per connection. HTTP/2 host-routing belongs to Phase 3.

use std::sync::Arc;
use std::time::Duration;

use hackline_proto::Zid;
use rusqlite::OptionalExtension;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{info, warn};
use zenoh::bytes::ZBytes;
use zenoh::Session;

use crate::db::pool::DbPool;
use crate::error::GatewayError;

const HEADER_LIMIT: usize = 8 * 1024;
const READ_BUF: usize = 8 * 1024;

/// Run a single HTTP host-routing listener on `listen_addr`. Each
/// accepted connection is host-routed to the matching `http` tunnel.
pub async fn run(
    db: DbPool,
    session: Arc<Session>,
    listen_addr: &str,
) -> Result<(), GatewayError> {
    let listener = TcpListener::bind(listen_addr).await?;
    info!(addr = listen_addr, "HTTP host-router listening");

    loop {
        let (tcp, peer) = listener.accept().await?;
        let db = db.clone();
        let session = session.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(db, session, tcp).await {
                warn!(%peer, "http host-router: {e}");
            }
        });
    }
}

async fn handle_connection(
    db: DbPool,
    session: Arc<Session>,
    mut tcp: tokio::net::TcpStream,
) -> Result<(), GatewayError> {
    let mut prefix = Vec::with_capacity(2048);
    let mut buf = [0u8; READ_BUF];
    let host = loop {
        if prefix.len() > HEADER_LIMIT {
            return Err(GatewayError::BadRequest(
                "HTTP header section exceeded 8 KiB".into(),
            ));
        }
        let n = tcp.read(&mut buf).await?;
        if n == 0 {
            return Err(GatewayError::BadRequest("HTTP preamble closed early".into()));
        }
        prefix.extend_from_slice(&buf[..n]);
        if let Some(h) = parse_host_header(&prefix) {
            break h;
        }
        if find_double_crlf(&prefix).is_some() {
            return Err(GatewayError::BadRequest(
                "HTTP request missing Host header".into(),
            ));
        }
    };

    let host_lookup = host.clone();
    let row = tokio::task::spawn_blocking(move || -> Result<Option<(String, i64)>, GatewayError> {
        let conn = db.get()?;
        let r = conn
            .query_row(
                "SELECT d.zid, t.local_port
                   FROM tunnels t
                   JOIN devices d ON d.id = t.device_id
                  WHERE t.kind = 'http'
                    AND t.public_hostname = ?1
                    AND t.enabled = 1",
                rusqlite::params![host_lookup],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(GatewayError::Db)?;
        Ok(r)
    })
    .await
    .map_err(|e| GatewayError::Config(format!("blocking task join: {e}")))??;

    let (zid_str, local_port) = row.ok_or_else(|| {
        GatewayError::BadRequest(format!("no http tunnel for host `{host}`"))
    })?;
    let zid = Zid::new(&zid_str).map_err(|e| GatewayError::BadRequest(e.to_string()))?;

    bridge_with_prefix(&session, &zid, local_port as u16, tcp, prefix).await
}

/// Open a bridge to the device's local HTTP port and stream bytes
/// in both directions. The captured `prefix` is forwarded first so
/// the device's local HTTP server sees a complete request.
async fn bridge_with_prefix(
    session: &Session,
    zid: &Zid,
    port: u16,
    tcp: tokio::net::TcpStream,
    prefix: Vec<u8>,
) -> Result<(), GatewayError> {
    let (mut tcp_read, mut tcp_write) = tcp.into_split();

    let request_id = uuid::Uuid::new_v4();
    let req = hackline_proto::connect::ConnectRequest {
        request_id,
        peer: None,
    };
    let ke_connect = hackline_proto::keyexpr::connect(zid, port);
    let replies = session
        .get(&ke_connect)
        .payload(ZBytes::from(serde_json::to_vec(&req).unwrap()))
        .timeout(Duration::from_secs(2))
        .await
        .map_err(GatewayError::Zenoh)?;
    let reply = tokio::time::timeout(Duration::from_secs(10), replies.recv_async())
        .await
        .map_err(|_| GatewayError::BadRequest("device ack timeout".into()))?
        .map_err(GatewayError::Zenoh)?;
    drop(replies);

    let ack_bytes = reply
        .result()
        .map_err(|e| GatewayError::BadRequest(format!("device ack: {e:?}")))?
        .payload()
        .to_bytes()
        .to_vec();
    let ack: hackline_proto::connect::ConnectAck = serde_json::from_slice(&ack_bytes)
        .map_err(|e| GatewayError::BadRequest(format!("ack decode: {e}")))?;
    if !ack.ok {
        return Err(GatewayError::BadRequest(
            ack.message.unwrap_or_else(|| "device rejected".into()),
        ));
    }

    let ke_to_dev = hackline_proto::keyexpr::stream_gw(zid, &request_id);
    let ke_from_dev = hackline_proto::keyexpr::stream_dev(zid, &request_id);

    let publisher = session
        .declare_publisher(ke_to_dev)
        .await
        .map_err(GatewayError::Zenoh)?;
    let subscriber = session
        .declare_subscriber(ke_from_dev)
        .await
        .map_err(GatewayError::Zenoh)?;

    if !prefix.is_empty() {
        publisher
            .put(ZBytes::from(prefix))
            .await
            .map_err(GatewayError::Zenoh)?;
    }

    let tcp_to_zenoh = tokio::spawn(async move {
        let mut b = vec![0u8; 32 * 1024];
        loop {
            match tcp_read.read(&mut b).await {
                Ok(0) => {
                    let _ = publisher.put(ZBytes::from(Vec::<u8>::new())).await;
                    break;
                }
                Ok(n) => {
                    if publisher.put(ZBytes::from(b[..n].to_vec())).await.is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = publisher.put(ZBytes::from(Vec::<u8>::new())).await;
                    break;
                }
            }
        }
    });

    let zenoh_to_tcp = tokio::spawn(async move {
        loop {
            let sample = match subscriber.recv_async().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let bytes = sample.payload().to_bytes().to_vec();
            if bytes.is_empty() {
                break;
            }
            if tcp_write.write_all(&bytes).await.is_err() {
                break;
            }
        }
        let _ = tcp_write.shutdown().await;
    });

    let _ = tokio::join!(tcp_to_zenoh, zenoh_to_tcp);
    Ok(())
}

fn parse_host_header(buf: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(buf).ok()?;
    for line in text.split("\r\n") {
        if line.is_empty() {
            return None;
        }
        for prefix in ["Host:", "host:", "HOST:"] {
            if let Some(rest) = line.strip_prefix(prefix) {
                return Some(rest.trim().to_string());
            }
        }
    }
    None
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_header_parsing() {
        let req = b"GET / HTTP/1.1\r\nHost: device-1.cloud.example.com\r\nUser-Agent: x\r\n\r\n";
        assert_eq!(
            parse_host_header(req).as_deref(),
            Some("device-1.cloud.example.com")
        );
    }

    #[test]
    fn host_header_missing() {
        let req = b"GET / HTTP/1.1\r\nUser-Agent: x\r\n\r\n";
        assert!(parse_host_header(req).is_none());
    }
}
