//! Per-tunnel TCP listener. One task per `kind = 'tcp'` row; accepts
//! connections and hands each one to `tunnel::bridge`.

use std::sync::Arc;

use hackline_proto::Zid;
use tokio::net::TcpListener;
use tracing::{info, warn};
use zenoh::Session;

use crate::error::GatewayError;

/// Listen on `listen_port` and bridge every accepted connection to
/// `device_port` on the device identified by `zid`.
pub async fn run_tcp_listener(
    session: Arc<Session>,
    zid: Zid,
    device_port: u16,
    listen_port: u16,
) -> Result<(), GatewayError> {
    let listener = TcpListener::bind(format!("0.0.0.0:{listen_port}")).await?;
    info!(listen_port, %zid, device_port, "tcp tunnel listener ready");

    loop {
        let (tcp, addr) = listener.accept().await?;
        let s = session.clone();
        let z = zid.clone();
        tokio::spawn(async move {
            let peer = addr.to_string();
            if let Err(e) = hackline_core::bridge::initiate_bridge(
                &s,
                &z,
                device_port,
                tcp,
                Some(peer.clone()),
            )
            .await
            {
                warn!(%addr, "bridge error: {e}");
            }
            info!(%addr, "connection closed");
        });
    }
}
