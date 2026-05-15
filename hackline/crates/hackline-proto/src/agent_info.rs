//! `AgentInfo` — payload returned by `hackline/<zid>/info`.

use serde::{Deserialize, Serialize};

/// Describes a running agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AgentInfo {
    pub label: Option<String>,
    pub allowed_ports: Vec<u16>,
}
