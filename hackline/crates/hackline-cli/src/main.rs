//! `hackline` CLI entry point. Parses argv, dispatches to `cmd/`.

mod client;
mod cmd;
mod config;
mod error;
mod output;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hackline", about = "hackline CLI")]
struct Cli {
    /// Gateway base URL (overrides cached value)
    #[arg(long, global = true, env = "HACKLINE_SERVER")]
    server: Option<String>,

    /// Bearer token (overrides cached value)
    #[arg(long, global = true, env = "HACKLINE_TOKEN")]
    token: Option<String>,

    /// Output as JSON instead of human-readable tables
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Claim a fresh gateway and cache credentials
    Login {
        /// Gateway URL
        #[arg(long)]
        server: String,
        /// Claim token printed by the gateway at first boot
        #[arg(long)]
        token: String,
        /// Owner name
        #[arg(long, default_value = "owner")]
        name: String,
    },
    /// Print cached identity
    Whoami,
    /// Device management
    #[command(subcommand)]
    Device(DeviceCmd),
    /// Tunnel management
    #[command(subcommand)]
    Tunnel(TunnelCmd),
    /// User management
    #[command(subcommand)]
    User(UserCmd),
    /// Message-plane events (live tail + history)
    #[command(subcommand)]
    Events(EventsCmd),
    /// Message-plane logs (live tail + history)
    #[command(subcommand)]
    Log(LogCmd),
}

#[derive(Subcommand)]
enum EventsCmd {
    /// Follow the SSE stream and print one JSON line per delivery.
    Tail {
        #[arg(long)]
        device: Option<i64>,
        #[arg(long)]
        topic: Option<String>,
    },
    /// Page the cursor API for historical entries.
    History {
        #[arg(long)]
        device: Option<i64>,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
}

#[derive(Subcommand)]
enum LogCmd {
    /// Follow the SSE log stream.
    Tail {
        #[arg(long)]
        device: Option<i64>,
        #[arg(long)]
        topic: Option<String>,
    },
    /// Page the cursor API for historical log entries.
    History {
        #[arg(long)]
        device: Option<i64>,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
}

#[derive(Subcommand)]
enum DeviceCmd {
    /// Register a device
    Add {
        #[arg(long)]
        zid: String,
        #[arg(long)]
        label: String,
    },
    /// List all devices
    List,
    /// Show a single device
    Show {
        #[arg(long)]
        id: i64,
    },
    /// Remove a device
    Remove {
        #[arg(long)]
        id: i64,
    },
}

#[derive(Subcommand)]
enum TunnelCmd {
    /// Create a tunnel
    Add {
        #[arg(long)]
        device_id: i64,
        #[arg(long, default_value = "tcp")]
        kind: String,
        #[arg(long)]
        local_port: i64,
        #[arg(long)]
        public_port: Option<i64>,
        #[arg(long)]
        public_hostname: Option<String>,
    },
    /// List all tunnels
    List,
    /// Remove a tunnel
    Remove {
        #[arg(long)]
        id: i64,
    },
}

#[derive(Subcommand)]
enum UserCmd {
    /// Create a user
    Add {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "operator")]
        role: String,
    },
    /// List all users
    List,
    /// Remove a user
    Remove {
        #[arg(long)]
        id: i64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let json = cli.json;

    match cli.command {
        Command::Login { server, token, name } => {
            cmd::login::run(&server, &token, &name).await?;
        }
        Command::Whoami => {
            cmd::whoami::run(json)?;
        }
        Command::Device(sub) => {
            let c = client::Client::from_args_or_cache(cli.server, cli.token)?;
            match sub {
                DeviceCmd::Add { zid, label } => cmd::device::add::run(&c, &zid, &label, json).await?,
                DeviceCmd::List => cmd::device::list::run(&c, json).await?,
                DeviceCmd::Show { id } => cmd::device::show::run(&c, id, json).await?,
                DeviceCmd::Remove { id } => cmd::device::remove::run(&c, id).await?,
            }
        }
        Command::Tunnel(sub) => {
            let c = client::Client::from_args_or_cache(cli.server, cli.token)?;
            match sub {
                TunnelCmd::Add { device_id, kind, local_port, public_port, public_hostname } => {
                    cmd::tunnel::add::run(&c, device_id, &kind, local_port, public_port, public_hostname.as_deref(), json).await?
                }
                TunnelCmd::List => cmd::tunnel::list::run(&c, json).await?,
                TunnelCmd::Remove { id } => cmd::tunnel::remove::run(&c, id).await?,
            }
        }
        Command::User(sub) => {
            let c = client::Client::from_args_or_cache(cli.server, cli.token)?;
            match sub {
                UserCmd::Add { name, role } => cmd::user::add::run(&c, &name, &role, json).await?,
                UserCmd::List => cmd::user::list::run(&c, json).await?,
                UserCmd::Remove { id } => cmd::user::remove::run(&c, id).await?,
            }
        }
        Command::Events(sub) => {
            let c = client::Client::from_args_or_cache(cli.server, cli.token)?;
            match sub {
                EventsCmd::Tail { device, topic } => {
                    cmd::events::tail(&c, device, topic.as_deref(), cmd::events::StreamKind::Events).await?
                }
                EventsCmd::History { device, topic, limit } => {
                    cmd::events::history(&c, device, topic.as_deref(), limit, json, cmd::events::StreamKind::Events).await?
                }
            }
        }
        Command::Log(sub) => {
            let c = client::Client::from_args_or_cache(cli.server, cli.token)?;
            match sub {
                LogCmd::Tail { device, topic } => {
                    cmd::events::tail(&c, device, topic.as_deref(), cmd::events::StreamKind::Logs).await?
                }
                LogCmd::History { device, topic, limit } => {
                    cmd::events::history(&c, device, topic.as_deref(), limit, json, cmd::events::StreamKind::Logs).await?
                }
            }
        }
    }

    Ok(())
}
