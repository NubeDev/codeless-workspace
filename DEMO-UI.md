# Codeless browser demo — quickstart

Boot the headless core in one terminal, the Vite dev server in
another, paste a bearer token into the browser, and the
JobsDashboard mounts against a live `codeless-server`.

## One-time token init

```sh
cargo run -p codeless-cli --bin codeless -- \
    --db /tmp/codeless-demo.db \
    serve --init-token
```

Copy the 32-char hex string it prints. Re-run with `--force` to
rotate; the previous token is invalidated.

## Terminal A — core

```sh
cargo run -p codeless-cli --bin codeless -- \
    --db /tmp/codeless-demo.db \
    serve --bind 127.0.0.1:7777
```

Leave it running. The server logs `listening on http://127.0.0.1:7777`
on stderr.

## Terminal B — UI

```sh
pnpm -C codeless/ui/codeless-ui install   # first time only
pnpm -C codeless/ui/codeless-ui dev
```

Vite serves at `http://localhost:5173`.

## Browser

Open `http://localhost:5173`. The browser shell reads two keys from
`localStorage` (see `codeless/ui/codeless-ui/src/lib/rpc/config.ts`):

| key                       | value                          |
|---------------------------|--------------------------------|
| `codeless-rpc-base-url`   | `http://127.0.0.1:7777`        |
| `codeless-rpc-token`      | the 32-char hex from above     |

Set them once in DevTools:

```js
localStorage.setItem("codeless-rpc-base-url", "http://127.0.0.1:7777");
localStorage.setItem("codeless-rpc-token", "<paste token>");
location.reload();
```

The JobsDashboard now drives a real core: repos, jobs, and the live
event tail all come from SQLite via `codeless-server` REST + SSE.

## Seeding a demo job (optional)

In a third terminal, hit the same DB:

```sh
cargo run -p codeless-cli --bin codeless -- --db /tmp/codeless-demo.db \
    run --repo /path/to/some/checkout --runner mock "hello world"
```

The browser's live tail reflects the new job within a tick.

## Troubleshooting

- "401 Unauthorized" in the browser console: token mismatch. Re-run
  `serve --init-token --force` and refresh localStorage.
- Browser at `http://localhost:5173` but core at `127.0.0.1:7777`:
  the dev server proxies nothing — the UI hits the configured
  `baseUrl` directly. CORS is opened workspace-wide by
  `codeless-server`'s middleware.
- EventSource disconnects on tab background: expected; it
  auto-reconnects with `Last-Event-ID`.
