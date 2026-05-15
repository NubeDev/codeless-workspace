# `@hackline/client`

TypeScript client for the [hackline](../../) gateway. Speaks the REST
+ SSE control-plane surface documented in `hackline/SCOPE.md` §5.

Two import surfaces:

- `@hackline/client` — `ApiClient` interface and `HttpApiClient`
  implementation against the gateway's `/v1/*` REST + `/v1/events/stream`
  SSE endpoints, plus the hand-written `types.ts` covering the REST
  request/response shapes.
- `@hackline/client/wire` — Rust-generated wire types from
  `hackline-proto` (connection lifecycle, `Event`, `MsgEnvelope`,
  `CmdEnvelope`, `ApiRequest`, `ApiReply`). Used by direct Zenoh
  consumers; regenerated from the Rust source by
  `cargo run -p hackline-proto --features specta --example wire_ts`.

The two surfaces overlap conceptually but not in shape today — the
gateway's SSE event JSON is hand-written, the Zenoh-side `Event` enum
comes from Rust. They will be reconciled when the gateway grows
specta-derived types.

## No mocks

This package ships only real-transport clients. There is no
`MockApiClient`, no in-memory fixture mode, and no `?mock=1`
short-circuit in any consumer. Tests — in this package and in
downstream consumers — run against a real gateway (loopback Zenoh
router for E2E). See `DOCS/CODEBASE-ANALYSIS.md` and
`DOCS/DEVELOPMENT.md`: "if it's wrong, do not paper over it with
mocks."

## Status

v0.1, workspace-linked only. No npm publish yet — consumers
inside this repo depend via pnpm `workspace:*`.
