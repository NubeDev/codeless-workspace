//! Handles `hackline/<zid>/tcp/<port>/connect` queries. Validates the
//! requested port against the agent's whitelist before opening a
//! loopback TCP connection and handing off to `hackline-core::bridge`.
