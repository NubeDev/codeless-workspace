//! Zenoh-session open / close helpers. Centralised so that the
//! gateway and the agent use the same shutdown semantics — important
//! because a half-closed session leaks subscribers.
