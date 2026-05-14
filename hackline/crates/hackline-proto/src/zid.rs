//! Zenoh device-id newtype.
//!
//! Canonical form: lowercase hex, no separators, length 2..=32.
//! Parsing and validation live here; no other crate is allowed to
//! roundtrip a raw `String` for a ZID.
