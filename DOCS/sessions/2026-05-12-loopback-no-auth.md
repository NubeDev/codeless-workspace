# Build status — loopback = no auth, zero-paste first-run

> ⛔ **AGENT REMINDER — READ BEFORE TOUCHING THIS FILE**
>
> 1. JOB-LOOP spec: `DOCS/JOB-LOOP.md`. Project scope: `DOCS/SCOPE.md`.
>    Code-style rules: `CLAUDE.md`.
> 2. One logical batch per tick; verify + commit + push per stage.
> 3. ⛔ Schedule the next tick before exiting; report DONE if all `[x]`.
> 4. ⛔ Commit AND push every stage via mani; never --force, never --no-verify.
> 5. ⛔ Comments: why, not what. No emojis, no task-status, no banners.
> 6. ⛔ Cross-platform reach: UI imports `RpcClient` only.

File: DOCS/sessions/2026-05-12-loopback-no-auth.md
Goal: A user runs `codeless serve --fs-root .` + `pnpm dev`, opens the
      browser, and the demo works. No DevTools, no localStorage paste,
      no token. Non-loopback binds keep the bearer-token requirement
      (or refuse to boot).
Started: 2026-05-12
Last tick: 2026-05-12 18:55
Current stage: 1 / 5

Repo:        codeless
Branch:      master
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

## Stages

- [ ] 1. [M] Server: when bind address is loopback, allow                  ← next
       unauthenticated requests. Behaviour:
       - `bind.is_loopback() && !args.require_token` → bearer
         middleware short-circuits to allow, and `serve --init-token`
         is unnecessary (skipped silently if no token is configured).
       - `bind.is_loopback() && args.require_token` → bearer required
         as today.
       - `!bind.is_loopback() && !args.require_token` → refuse to
         boot with an explanatory error pointing at the flag. This
         is the footgun guard.
       Plumbed via `AppState` carrying an `auth_mode: Required | Open`
       and the existing `bearer_layer` checking it before the header
       compare. Existing 401 tests stay green by passing
       `--require-token` (or pinning the AppState constructor).

- [ ] 2. [S] UI: skip the auth header when no token is configured.
       `HttpRpcClient` already takes `token: Option<string>`; the
       header line only fires when `Some`. The change is upstream
       at `readToken()` / `buildClient()`: when the URL is loopback
       and no localStorage token is set, return `None` instead of
       reading env var fallback. Same for `subscribe` SSE.

- [ ] 3. [S] UI: default `baseUrl` to `http://127.0.0.1:7777` when
       the Vite dev shell sees no `codeless-rpc-base-url` and is
       not on the special mock-mode shortcut. Today the browser
       shell defaults to `MockRpcClient` on the Vite dev port; flip
       that so the default is "talk to the local server, fall back
       to mock if it 404s on /healthz at startup". User-visible:
       opening `http://127.0.0.1:5173` against a running server
       Just Works. If the server is not running, the existing mock
       banner stays.

- [ ] 4. [S] DEMO-UI.md + codeless/README.md: drop the
       localStorage paste step from the happy path. Mention the
       --require-token flow as a one-liner for non-loopback binds.

- [ ] 5. [S] scripts/smoke-demo.sh: keep using a token to validate
       the bearer path still works. Add a parallel `smoke-demo-loopback.sh`
       (or a `--no-auth` flag) that asserts the zero-token path
       returns 200 on `/rpc/list_repos` against a loopback bind.

## Notes
- R5 (Single-tenant trust boundary) explicitly allows this: same
  trust boundary as the user, loopback already restricts to
  same-host same-user processes. The token only made sense once
  the server might be reached from elsewhere.
- The bearer-token machinery is not removed — operators who bind
  to anything other than loopback still need it. The token is just
  optional on loopback now.

## Blockers
(none)

## Tick log
