# `@hackline/client`

TypeScript client for the [hackline](../../) gateway. Speaks the REST
+ SSE control-plane surface documented in `hackline/SCOPE.md` §5.

Two import surfaces:

- `@hackline/client` — `ApiClient` interface, `HttpApiClient`
  implementation against the gateway's `/v1/*` REST + `/v1/events/stream`
  SSE endpoints, `MockApiClient` for tests, plus the hand-written
  `types.ts` covering the REST request/response shapes.
- `@hackline/client/wire` — Rust-generated wire types from
  `hackline-proto` (connection lifecycle, `Event`, `MsgEnvelope`,
  `CmdEnvelope`, `ApiRequest`, `ApiReply`). Used by direct Zenoh
  consumers; regenerated from the Rust source by
  `cargo run -p hackline-proto --features specta --example wire_ts`.

The two surfaces overlap conceptually but not in shape today — the
gateway's SSE event JSON is hand-written, the Zenoh-side `Event` enum
comes from Rust. They will be reconciled when the gateway grows
specta-derived types.

## Status

v0.1, workspace-linked only. No npm publish yet — consumers
inside this repo depend via pnpm `workspace:*`.
