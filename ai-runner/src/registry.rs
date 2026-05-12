use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::runner::Runner;
use crate::types::Provider;

/// Thread-safe registry mapping [`Provider`] → `Arc<dyn Runner>`.
///
/// Initialise with all four built-in runners via [`Registry::with_defaults`].
/// Clone the `Arc<Registry>` to share it across tasks.
#[derive(Default)]
pub struct Registry {
    runners: RwLock<HashMap<Provider, Arc<dyn Runner>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry pre-loaded with:
    /// - **CLI**: [`ClaudeRunner`] (claude-wrapper), [`CodexRunner`] (tokio process)
    /// - **REST**: [`AnthropicRunner`] (anthropic-ai-sdk), [`OpenAiRunner`] (async-openai)
    pub fn with_defaults() -> Self {
        use crate::runners::{
            anthropic::AnthropicRunner, claude::ClaudeRunner, codex::CodexRunner,
            openai::OpenAiRunner,
        };
        let r = Self::new();
        r.register(Arc::new(ClaudeRunner));
        r.register(Arc::new(CodexRunner));
        r.register(Arc::new(AnthropicRunner));
        r.register(Arc::new(OpenAiRunner));
        r
    }

    /// Register (or replace) a runner.
    pub fn register(&self, runner: Arc<dyn Runner>) {
        let key = runner.provider().clone();
        self.runners
            .write()
            .expect("registry lock")
            .insert(key, runner);
    }

    /// Look up a runner by provider.
    pub fn get(&self, provider: &Provider) -> Option<Arc<dyn Runner>> {
        self.runners
            .read()
            .expect("registry lock")
            .get(provider)
            .cloned()
    }

    /// List all registered providers with their readiness.
    ///
    /// Async because `Runner::ready` is async; readiness is computed
    /// fresh on each call and may touch the filesystem (CLI runners) or
    /// process env (REST runners). The lock is released before any
    /// `await` to keep readiness probes from blocking registration.
    pub async fn list(&self) -> Vec<ProviderStatus> {
        let runners: Vec<std::sync::Arc<dyn Runner>> = self
            .runners
            .read()
            .expect("registry lock")
            .values()
            .cloned()
            .collect();
        let mut out = Vec::with_capacity(runners.len());
        for r in runners {
            let ready = r.ready().await;
            out.push(ProviderStatus {
                provider: r.provider().clone(),
                available: ready,
            });
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub provider: Provider,
    pub available: bool,
}
