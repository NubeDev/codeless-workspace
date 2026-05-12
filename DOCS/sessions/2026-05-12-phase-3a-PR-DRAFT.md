# Draft PR — `feat/phase-2a-persistence` (Phase 2a + 2b + 2c + 3a)

> Not opened automatically; landing the PR is a human action. This
> file is the draft body the human can paste into `gh pr create`.

## Title
`feat: persistence + runtime + reviews + browser demo (Phases 2a-3a)`

## Summary

This branch is the cumulative work of Phases 2a → 3a, stacked on
`master`. The four phases bundle because each leans on the previous
one's wire types and runtime surface; splitting them would either
duplicate migrations or leave half-formed RPC methods on `master`.

- **Phase 2a — persistence.** SQLite-backed `SqliteStore`,
  `EventBus` with broadcast + persisted-cursor replay, lease-based
  task queue with three-scope concurrency caps, durable migrations
  from Appendix A, startup lease reaper.
- **Phase 2b — real runners + worktree + cost.** `ClaudeRunnerAdapter`
  shelling out to the `claude` CLI; `AnthropicRunnerAdapter` over the
  REST API with cost rollup per stage; `WorktreeManager` for isolated
  `git worktree`-backed job execution; heartbeat / lease renewal
  loop driving the queue.
- **Phase 2c — reviews + tail + notifier.** `Review` row + the four
  `*_review` RPC methods (list/approve/comment/stop); `codeless tail
  <job-id>` CLI verb replaying persisted events and following live
  with a cursor; HMAC-SHA256-signed `WebhookNotifier` for `JobFailed`
  and `ReviewRequested`.
- **Phase 3a — browser demo loop.** `codeless-server` axum library
  exposing every `RpcServer` method as `POST /rpc/<method>` plus
  `GET /events` SSE; `codeless serve` CLI verb wiring the runtime,
  the shared bearer token from the secrets file (key
  `core_bearer_token`), and graceful Ctrl-C shutdown; permissive
  CORS for the single-tenant loopback MVP; `DEMO-UI.md` quickstart
  in the parent workspace.

## What's not in this PR

- The Tauri desktop shell (Phase 5).
- The mobile shells (Phase 6).
- Anything that touches `ui/codeless-ui/` from the JOB-LOOP side —
  in-flight UI work (shell-injection adapters, `fs.*`/`secrets.*`
  RPC wiring) lives on the same branch but is driven by a parallel
  session and is out of scope here.

## Verify

```sh
cargo test --workspace                                  # 118 passed
cargo clippy --workspace --all-targets -- -D warnings   # clean
cargo fmt --check                                       # clean
```

Manual dry run (Phase 3a stage 4) is captured in
`DOCS/sessions/2026-05-12-phase-3a-terax-demo.md` Notes.

## Test plan

- [ ] `cargo test --workspace` green on a clean checkout.
- [ ] `codeless serve --init-token` prints a 32-char hex string and
      persists it to `core_bearer_token`.
- [ ] `codeless serve` binds, accepts a `POST /rpc/list_repos` with
      the bearer, refuses without it (401).
- [ ] `GET /events?scope=all&since=0` replays existing events and
      surfaces live ones within ~1s.
- [ ] Follow `DEMO-UI.md` end-to-end against a fresh DB; the
      Terax-derived `JobsDashboard` mounts and shows repos / jobs
      backed by SQLite.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
