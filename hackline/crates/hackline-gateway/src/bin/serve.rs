//! `hackline-gateway serve` — boot the gateway and bind every listener.

use std::path::PathBuf;
use std::sync::Arc;

use hackline_gateway::api;
use hackline_gateway::config::GatewayConfig;
use hackline_gateway::db::{claim, migrations, pool};
use hackline_gateway::cmd_delivery::{self, CmdNotifier};
use hackline_gateway::events_bus::MsgBus;
use hackline_gateway::msg_fanin;
use hackline_gateway::state::AppState;
use hackline_gateway::tunnel::{http_router, manager};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gateway.toml"));

    let cfg = GatewayConfig::load(&config_path)?;

    init_tracing(&cfg.log.level, &cfg.log.format);

    info!(config = ?config_path, "starting hackline-gateway");

    let db_path = cfg.database.as_deref().unwrap_or("gateway.db");
    let db = pool::open(std::path::Path::new(db_path))?;

    {
        let conn = db.get()?;
        migrations::run(&conn)?;
    }
    info!(db = db_path, "database ready");

    {
        let conn = db.get()?;
        match claim::ensure_pending(&conn)? {
            Some(token) => {
                info!("gateway unclaimed — claim token printed below");
                println!("\n  CLAIM TOKEN: {token}\n");
                println!("  Use: hackline login --server http://{listen_addr} --token {token}\n",
                    listen_addr = cfg.listen.as_deref().unwrap_or("127.0.0.1:8080"));
            }
            None => {
                info!("gateway already claimed (or claim pending from previous boot)");
            }
        }
    }

    let zenoh_cfg = cfg.to_zenoh_config()?;
    let session = Arc::new(hackline_core::session::open(zenoh_cfg).await?);
    info!(zid = %session.zid(), "zenoh session open");

    let (tunnel_tx, tunnel_rx) = tokio::sync::mpsc::channel(64);

    let msg_bus = MsgBus::new();
    let _fanin_handles =
        msg_fanin::spawn(session.clone(), db.clone(), msg_bus.clone()).await?;

    let cmd_notifier = CmdNotifier::new();
    let _cmd_handles =
        cmd_delivery::spawn(session.clone(), db.clone(), cmd_notifier.clone()).await?;

    let state = AppState {
        db: db.clone(),
        zenoh: session.clone(),
        tunnel_tx,
        msg_bus,
        cmd_notifier,
    };

    let listen_addr = cfg.listen.as_deref().unwrap_or("127.0.0.1:8080");
    let app = api::router::build(state);
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    info!(addr = listen_addr, "REST API listening");

    // Run axum and tunnel manager concurrently. The fan-in subscriber
    // tasks own their own loops and don't need to be in the select —
    // dropping their handles when the process exits is enough.
    // Optional HTTP host-routing listener — proxies
    // `device-<id>.<base>` HTTPS-fronted requests into the matching
    // `http` tunnel. Off unless the operator configured `http_listen`.
    let http_router_fut = {
        let db = db.clone();
        let session = session.clone();
        let addr = cfg.http_listen.clone();
        async move {
            match addr {
                Some(a) => http_router::run(db, session, &a).await,
                None => std::future::pending::<Result<(), _>>().await,
            }
        }
    };

    tokio::select! {
        result = axum::serve(listener, app) => {
            result?;
        }
        result = manager::run(db, session, tunnel_rx) => {
            result?;
        }
        result = http_router_fut => {
            result?;
        }
    }

    Ok(())
}

fn init_tracing(level: &str, format: &str) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    match format {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .init();
        }
    }
}
