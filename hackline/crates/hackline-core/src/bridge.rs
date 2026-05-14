//! Bidirectional byte bridge between a TCP stream and a pair of Zenoh
//! pub/sub channels. The connect handshake uses a one-shot query/reply;
//! after the ack, data flows on `…/stream/<request_id>/gw` (gateway→agent)
//! and `…/stream/<request_id>/dev` (agent→gateway) until close.

use std::time::Duration;

use hackline_proto::connect::{ConnectAck, ConnectRequest};
use hackline_proto::keyexpr;
use hackline_proto::Zid;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, warn};
use uuid::Uuid;
use zenoh::bytes::ZBytes;
use zenoh::Session;

use crate::error::BridgeError;

const READ_BUF: usize = 32 * 1024;
const ACK_TIMEOUT: Duration = Duration::from_secs(10);
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

/// Agent side: accept a connect query, open a local TCP socket, and
/// run the byte bridge until either side closes.
pub async fn accept_bridge(
    session: &Session,
    zid: &Zid,
    port: u16,
    query: zenoh::query::Query,
) -> Result<(), BridgeError> {
    let payload = query
        .payload()
        .map(|p| p.to_bytes().to_vec())
        .unwrap_or_default();
    let req: ConnectRequest = serde_json::from_slice(&payload)
        .map_err(hackline_proto::error::ProtoError::Json)?;
    let request_id = req.request_id;

    debug!(%request_id, port, "accepting bridge");

    let tcp = match TcpStream::connect(format!("127.0.0.1:{port}")).await {
        Ok(s) => s,
        Err(e) => {
            let ack = ConnectAck {
                request_id,
                ok: false,
                message: Some(format!("tcp connect failed: {e}")),
            };
            let _ = query.reply(
                keyexpr::connect(zid, port),
                serde_json::to_vec(&ack).unwrap(),
            ).await;
            return Err(BridgeError::Io(e));
        }
    };

    let ack = ConnectAck {
        request_id,
        ok: true,
        message: None,
    };
    query.reply(
        keyexpr::connect(zid, port),
        serde_json::to_vec(&ack).unwrap(),
    ).await.map_err(BridgeError::Zenoh)?;

    // Drop the query so Zenoh sends the "final reply" frame — without this
    // the gateway's get() hangs until its internal timeout fires.
    drop(query);

    let ke_from_gw = keyexpr::stream_gw(zid, &request_id);
    let ke_to_gw = keyexpr::stream_dev(zid, &request_id);

    run_bridge(session, tcp, &ke_from_gw, &ke_to_gw).await
}

/// Gateway side: issue a connect query, wait for the ack, and run
/// the byte bridge on the paired pub/sub channels.
pub async fn initiate_bridge(
    session: &Session,
    zid: &Zid,
    port: u16,
    tcp: TcpStream,
    peer_addr: Option<String>,
) -> Result<(), BridgeError> {
    let request_id = Uuid::new_v4();
    let req = ConnectRequest {
        request_id,
        peer: peer_addr,
    };

    debug!(%request_id, %zid, port, "initiating bridge");

    let ke = keyexpr::connect(zid, port);
    let replies = session
        .get(&ke)
        .payload(ZBytes::from(serde_json::to_vec(&req).unwrap()))
        .timeout(QUERY_TIMEOUT)
        .await
        .map_err(BridgeError::Zenoh)?;

    let reply = tokio::time::timeout(ACK_TIMEOUT, replies.recv_async())
        .await
        .map_err(|_| BridgeError::AckTimeout)?
        .map_err(BridgeError::Zenoh)?;

    drop(replies);

    let ack_bytes = reply.result()
        .map_err(|e| BridgeError::Rejected(format!("{e:?}")))?
        .payload()
        .to_bytes()
        .to_vec();
    let ack: ConnectAck = serde_json::from_slice(&ack_bytes)
        .map_err(hackline_proto::error::ProtoError::Json)?;

    if !ack.ok {
        return Err(BridgeError::Rejected(
            ack.message.unwrap_or_default(),
        ));
    }

    let ke_to_agent = keyexpr::stream_gw(zid, &request_id);
    let ke_from_agent = keyexpr::stream_dev(zid, &request_id);

    run_bridge(session, tcp, &ke_from_agent, &ke_to_agent).await
}

/// Pump bytes between a TCP socket and a Zenoh pub/sub pair until
/// either side closes.
async fn run_bridge(
    session: &Session,
    tcp: TcpStream,
    subscribe_ke: &str,
    publish_ke: &str,
) -> Result<(), BridgeError> {
    let (mut tcp_read, mut tcp_write) = tcp.into_split();

    let publisher = session
        .declare_publisher(publish_ke.to_owned())
        .await
        .map_err(BridgeError::Zenoh)?;

    let subscriber = session
        .declare_subscriber(subscribe_ke.to_owned())
        .await
        .map_err(BridgeError::Zenoh)?;

    let pub_ke = publish_ke.to_owned();

    // TCP → Zenoh
    let tcp_to_zenoh = tokio::spawn(async move {
        let mut buf = vec![0u8; READ_BUF];
        loop {
            match tcp_read.read(&mut buf).await {
                Ok(0) => {
                    debug!(ke = %pub_ke, "tcp read EOF, publishing close sentinel");
                    let _ = publisher.put(ZBytes::from(Vec::<u8>::new())).await;
                    break;
                }
                Ok(n) => {
                    if publisher.put(ZBytes::from(buf[..n].to_vec())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!("tcp read error: {e}");
                    let _ = publisher.put(ZBytes::from(Vec::<u8>::new())).await;
                    break;
                }
            }
        }
    });

    let sub_ke = subscribe_ke.to_owned();

    // Zenoh → TCP
    let zenoh_to_tcp = tokio::spawn(async move {
        loop {
            match subscriber.recv_async().await {
                Ok(sample) => {
                    let bytes = sample.payload().to_bytes();
                    if bytes.is_empty() {
                        debug!(ke = %sub_ke, "received close sentinel");
                        break;
                    }
                    if tcp_write.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::try_join!(tcp_to_zenoh, zenoh_to_tcp);
    Ok(())
}
