//! `ai-runner` — unified AI provider runner.
//!
//! Supports two transport categories:
//!
//! | Transport | Providers | Auth |
//! |-----------|-----------|------|
//! | CLI subprocess | `claude` (claude-wrapper), `codex` | binary handles auth / env key |
//! | REST HTTP | Anthropic (anthropic-ai-sdk), OpenAI (async-openai) | API key |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use ai_runner::{CliCfg, Registry, RunnerInput, Provider};
//! use tokio::sync::mpsc;
//! use tokio_util::sync::CancellationToken;
//!
//! #[tokio::main]
//! async fn main() {
//!     let registry = Arc::new(Registry::with_defaults());
//!     let runner = registry.get(&Provider::Claude).unwrap();
//!
//!     let (tx, mut rx) = mpsc::channel(64);
//!     tokio::spawn(async move {
//!         while let Some(ev) = rx.recv().await {
//!             println!("{ev:?}");
//!         }
//!     });
//!
//!     let input = RunnerInput::Cli(CliCfg {
//!         prompt: "explain Rust lifetimes".into(),
//!         ..Default::default()
//!     });
//!     let cancel = CancellationToken::new();
//!     let result = runner
//!         .run(input, "session-1".into(), tx, cancel)
//!         .await
//!         .expect("runner accepts the input variant");
//!
//!     println!("{}", result.text);
//! }
//! ```

pub mod defaults;
pub mod registry;
pub mod runner;
pub mod runners;
pub mod types;

pub use defaults::AiDefaults;
pub use registry::{ProviderStatus, Registry};
pub use runner::{OnEvent, Runner};
pub use types::{
    CliCfg, Event, EventKind, HistoryMessage, PermissionMode, Provider, RestCfg, RunResult,
    RunnerError, RunnerInput, SessionId, ToolCallEntry, ToolChoice, ToolDef, ToolUse,
};
