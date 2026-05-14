//! TOML config loader. Schema documented in `DOCS/CONFIG.md`.
//! Unknown keys are an error so a typo doesn't silently disable
//! something.

use std::path::Path;

use serde::Deserialize;

use crate::error::GatewayError;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayConfig {
    #[serde(default)]
    pub listen: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
    pub zenoh: ZenohConfig,
    #[serde(default)]
    pub tunnels: Vec<TunnelEntry>,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZenohConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub listen: Vec<String>,
    #[serde(default)]
    pub connect: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TunnelEntry {
    pub zid: String,
    pub device_port: u16,
    pub listen_port: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

fn default_mode() -> String { "client".into() }
fn default_log_level() -> String { "info".into() }
fn default_log_format() -> String { "pretty".into() }

impl GatewayConfig {
    pub fn load(path: &Path) -> Result<Self, GatewayError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| GatewayError::Config(format!("{path:?}: {e}")))?;
        let cfg: Self = toml::from_str(&text)
            .map_err(|e| GatewayError::Config(format!("{path:?}: {e}")))?;
        Ok(cfg)
    }

    pub fn to_zenoh_config(&self) -> Result<zenoh::Config, GatewayError> {
        let mut config = zenoh::Config::default();
        config
            .insert_json5("mode", &format!(r#""{}""#, self.zenoh.mode))
            .map_err(|e| GatewayError::Config(format!("zenoh mode: {e}")))?;
        if !self.zenoh.listen.is_empty() {
            let json = serde_json::to_string(&self.zenoh.listen)
                .map_err(|e| GatewayError::Config(format!("zenoh listen: {e}")))?;
            config
                .insert_json5("listen/endpoints", &json)
                .map_err(|e| GatewayError::Config(format!("zenoh listen: {e}")))?;
        }
        if !self.zenoh.connect.is_empty() {
            let json = serde_json::to_string(&self.zenoh.connect)
                .map_err(|e| GatewayError::Config(format!("zenoh connect: {e}")))?;
            config
                .insert_json5("connect/endpoints", &json)
                .map_err(|e| GatewayError::Config(format!("zenoh connect: {e}")))?;
        }
        config
            .insert_json5("scouting/multicast/enabled", "false")
            .map_err(|e| GatewayError::Config(format!("zenoh scouting: {e}")))?;
        Ok(config)
    }
}
