//! Handles `hackline/<zid>/tcp/<port>/connect` queries. Validates the
//! requested port against the agent's whitelist before opening a
//! loopback TCP connection and handing off to `hackline-core::bridge`.

use std::sync::Arc;

use hackline_core::bridge;
use hackline_proto::keyexpr;
use hackline_proto::Zid;
use tracing::{info, warn};
use zenoh::Session;

use crate::error::AgentError;

/// Run one queryable per allowed port. Blocks until all queryables close.
pub async fn serve_connect(
    session: Arc<Session>,
    zid: &Zid,
    allowed_ports: &[u16],
) -> Result<(), AgentError> {
    let mut handles = Vec::with_capacity(allowed_ports.len());

    for &port in allowed_ports {
        let ke = keyexpr::connect(zid, port);
        let q = session.declare_queryable(&ke).await?;
        info!(ke = %ke, "queryable ready");

        let s = session.clone();
        let z = zid.clone();
        handles.push(tokio::spawn(async move {
            loop {
                match q.recv_async().await {
                    Ok(query) => {
                        let s2 = s.clone();
                        let z2 = z.clone();
                        tokio::spawn(async move {
                            if let Err(e) = bridge::accept_bridge(&s2, &z2, port, query).await {
                                warn!(port, "bridge error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        warn!(port, "queryable closed: {e}");
                        break;
                    }
                }
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }
    Ok(())
}
