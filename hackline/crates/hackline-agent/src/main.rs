//! `hackline-agent` binary entry point. Argv parsing and the logging
//! subscriber install live here; everything else is a library function.

mod config;
mod connect;
mod error;
mod info;
mod liveliness;

use std::path::PathBuf;
use std::sync::Arc;

use config::AgentConfig;
use hackline_proto::Zid;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("agent.toml"));

    let cfg = AgentConfig::load(&config_path)?;

    init_tracing(&cfg.log.level, &cfg.log.format);

    info!(config = ?config_path, ports = ?cfg.allowed_ports, "starting hackline-agent");

    let zid = Zid::new(&cfg.zid)?;

    let zenoh_cfg = cfg.to_zenoh_config()?;
    let session = Arc::new(hackline_core::session::open(zenoh_cfg).await?);

    info!(%zid, zenoh_zid = %session.zid(), "zenoh session open");

    connect::serve_connect(session, &cfg.org, &zid, &cfg.allowed_ports).await?;
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
