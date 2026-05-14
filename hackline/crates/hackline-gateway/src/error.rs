//! Gateway error type. Maps cleanly onto HTTP status codes via the
//! `IntoResponse` impl so handlers can `?` freely.
