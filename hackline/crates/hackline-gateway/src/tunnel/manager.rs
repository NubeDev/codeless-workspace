//! Watches the `tunnels` table and opens / closes listeners to match.
//! The single source of truth for "which listeners are live right now".

use std::sync::Arc;

use hackline_proto::Zid;
use tracing::{error, info};
use zenoh::Session;

use crate::db::pool::DbPool;
use crate::db::tunnels;
use crate::tunnel::tcp_listener;

/// Load active TCP tunnels from the DB and spawn a listener task for
/// each. Blocks until all listeners exit (i.e. forever, under normal
/// operation).
pub async fn run(db: DbPool, session: Arc<Session>) -> Result<(), crate::error::GatewayError> {
    let conn = db.get()?;
    let active = tunnels::list_active_tcp(&conn)?;
    drop(conn);

    if active.is_empty() {
        info!("no active TCP tunnels in DB");
        std::future::pending::<()>().await;
        return Ok(());
    }

    info!(count = active.len(), "starting tunnel listeners from DB");

    let mut handles = Vec::with_capacity(active.len());
    for t in active {
        let Ok(zid) = Zid::new(&t.zid) else {
            error!(zid = %t.zid, "invalid ZID in tunnels table, skipping");
            continue;
        };
        let s = session.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) =
                tcp_listener::run_tcp_listener(s, zid, t.local_port, t.public_port).await
            {
                error!(listen_port = t.public_port, "tunnel listener failed: {e}");
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }
    Ok(())
}
