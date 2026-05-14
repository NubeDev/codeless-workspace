## Done

- Implemented Goal 5 (Phase 2): cmd_outbox + delivery, synchronous api, HTTP host-routing.
- V003 migration adds cmd_outbox with per-device cap, TTL, partial pending index.
- hackline-proto: CmdEnvelope, CmdAck, CmdResult, ApiRequest, ApiReply; keyexpr builders msg_cmd / msg_cmd_ack / msg_api / MSG_CMD_ACK_FANIN / parse_msg_cmd_ack_keyexpr.
- hackline-client SDK: subscribe_cmd → CmdStream/CmdHandle with at-least-once ack semantics (SDK owns the ack keyexpr); serve_api wrapping a per-topic queryable.
- Gateway cmd_delivery: cmd-ack wildcard fan-in + push-on-enqueue/fallback-sweep pusher driven by CmdNotifier in AppState.
- REST: POST/GET /v1/devices/{id}/cmd[/{topic}], DELETE /v1/cmd/{cmd_id}, POST /v1/devices/{id}/api/{topic} (Zenoh `get`, returns 503/504 on unreachable/timeout).
- auth::scope::check_device enforces customer-role per-device scope at every new entry point.
- tunnel::http_router runs a single shared listener that peeks Host: off the first request, looks up the matching http tunnel, and bridges bytes (incl. WebSocket upgrades) through Zenoh. Wired through optional `http_listen` config.
- CLI: `hackline cmd send|list|cancel` and `hackline api call`.
- Integration tests `cmd_round_trip` + `api_round_trip` against two in-process Zenoh peers; both pass.
- Session note DOCS/sessions/2026-05-14-goal5-cmd-api-host-routing.md with plan, outcome, design.
- cargo check --workspace clean (only the two pre-existing hackline-agent warnings); cargo test --workspace green.
- Committed as `goal5: commands, api, HTTP host-routing` (e88aca7).

## Next

- (none) — job's two stages are complete. Phase 3 (audit completeness + admin UI) is the next planned goal but is out of scope for this job.

## What you need to know

- All work landed under hackline/. Top-level CLAUDE.md from codeless-workspace and runs/ artifacts were intentionally not committed (gitignored at parent / runtime scratch).
- The HTTP host-router intentionally does not parse HTTP framing — it peeks Host: once and then bridges raw bytes, so WebSocket Upgrade works for free. Keep-alive across different Host: values on a single TCP connection is not supported (documented in module comment, matches Phase 2 scope; HTTP/2 is Phase 3).
- The cmd pusher uses push-on-enqueue (via CmdNotifier::notify from the REST handler) plus a 30 s fallback sweep — picks the Push-on-enqueue strawman in SCOPE.md §14 Q2 and explains it in the session-note Design section. No SCOPE.md change was needed.
- Test harness reuses the loopback-ephemeral-port pattern from goal4's message_plane.rs so concurrent `cargo test` runs don't collide.

## Open questions

- (none introduced this stage. SCOPE.md §14 Q2 (cmd-delivery shape) is effectively answered by the implementation; if the operator wants pull-from-device instead, revisit then.)
