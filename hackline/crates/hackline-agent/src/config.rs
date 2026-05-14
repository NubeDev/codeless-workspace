//! TOML config loader for the agent. Schema is documented in
//! `DOCS/CONFIG.md`. Validation rejects unknown keys so a typo in
//! `allowed_ports` doesn't silently expose nothing.
