//! Per-tunnel TCP listener. One task per `kind = 'tcp'` row; accepts
//! connections and hands each one to `tunnel::bridge`.
