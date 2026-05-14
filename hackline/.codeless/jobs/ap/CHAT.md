# Chat for this job

## user @ 2026-05-14T07:06:39Z

whats the job scope

## assistant @ 2026-05-14T07:06:47Z

The job **ap** has two stages, both inside the hackline repo, each landing one goal from `SCOPE.md` §13:

**Stage 1 — Goal 4: message plane (Phase 1.5)**
- SDK `publish_event` / `publish_log`
- Gateway fan-in subscribers on `hackline/*/msg/event/**` and `hackline/*/msg/log/**`
- New `events` + `logs` tables with ring-buffer pruning (inside the insert txn)
- `GET /v1/events`, `GET /v1/log` + SSE streams
- `hackline events tail` CLI
- Session note `DOCS/sessions/2026-05-14-goal4-message-plane.md` in the goal3 shape

**Stage 2 — Goal 5: commands + api + HTTP host-routing (Phase 2)**
- `cmd_outbox` table + gateway delivery loop
- SDK `subscribe_cmd` with ack semantics
- `POST /v1/devices/:id/cmd/:topic`
- SDK `serve_api` + synchronous `POST /v1/devices/:id/api/:topic`
- axum HTTP host-routing for `device-<id>.cloud.example.com` (incl. WebSocket passthrough)
- customer-role + per-device scope enforced at the edge
- Session note `DOCS/sessions/2026-05-14-goal5-cmd-api-host-routing.md`

**Gates per stage:** plan table written first and ticked as work lands; `cargo check --workspace` + `cargo test --workspace` clean (no new warnings); Outcome + Design sections filled. In-repo mode, commit per stage, no pushing to origin, no edits to landed migrations or the auth layer beyond wiring middleware onto new routes.
