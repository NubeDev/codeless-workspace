# Codeless browser demo — quickstart

Two terminals, one browser. End state: the Terax-derived UI lists a
repo, browses files from disk, opens one in the editor, and shows a
mock job running end-to-end via SSE — all against a real
`codeless-server`, no mocks.

## Prereqs

- `cargo` (Rust toolchain matching `rust-toolchain.toml`)
- `pnpm` (the UI uses pnpm; `npm install` produces a stale lockfile)
- A directory you don't mind the demo reading from — anything works.
  The instructions below use the workspace root itself.

## One-time token init

```sh
cargo run -p codeless-cli --bin codeless -- \
    --db /tmp/codeless-demo.db \
    serve --init-token
```

Copy the 32-char hex string it prints. Re-run with `--force` to
rotate.

## Seed a demo repo + queued job

```sh
cargo run -p codeless-cli --bin codeless -- \
    --db /tmp/codeless-demo.db \
    demo bootstrap
```

Idempotent — running it twice does not duplicate the seed. The repo
row is named `demo`; the queued job uses the `mock` runner so it
completes without needing an AI provider configured.

## Terminal A — core

```sh
cargo run -p codeless-cli --bin codeless -- \
    --db /tmp/codeless-demo.db \
    serve \
    --bind 127.0.0.1:7777 \
    --fs-root "$PWD"
```

`--fs-root` is what makes the file explorer light up against the
real backend; without it, the `fs.*` RPC surface returns `Internal`
and the explorer stays empty. Leave the server running.

## Terminal B — UI

```sh
pnpm -C codeless/ui/codeless-ui install   # first time only
pnpm -C codeless/ui/codeless-ui dev
```

Vite serves at `http://localhost:5173`. (Vite may pick `5174` if
`5173` is busy — read its startup line.)

## Browser

Open the Vite URL. The browser shell reads two keys from
`localStorage`:

| key                     | value                       |
|-------------------------|-----------------------------|
| `codeless-rpc-base-url` | `http://127.0.0.1:7777`     |
| `codeless-rpc-token`    | the 32-char hex from above  |

Set them once in DevTools:

```js
localStorage.setItem("codeless-rpc-base-url", "http://127.0.0.1:7777");
localStorage.setItem("codeless-rpc-token", "<paste token>");
location.reload();
```

The mock-mode banner in the corner disappears once you're on the
real transport.

## What to expect

- File explorer panel shows the contents of whatever `--fs-root`
  pointed at. Click a file to open it in the editor.
- Edit a file and save (`Ctrl/Cmd-S`); the change lands on disk via
  `fs_write_file`.
- Jobs tab shows the `demo` repo with one completed mock job. Click
  the job to see its timeline: `task-started` → a few `ai-token`
  deltas (the prompt echoed back) → `task-completed` → `job-completed`.
- `SubmitJobDialog` on the dashboard submits more jobs against the
  same repo. With the default factory only the `mock` runner is
  wired; `--enable-claude` / `--enable-anthropic` on `codeless serve`
  light up the real ones.

## Smoke test

A scripted version of the above for catching regressions in CI or
during a refactor. Runs everything in one process tree against
ephemeral resources and asserts the demo path works.

```sh
codeless-workspace/scripts/smoke-demo.sh
```

The script boots `codeless serve` on `127.0.0.1:7799`, seeds the
demo data, polls `list_jobs` until the mock job hits `completed`,
and exits 0 on success.

## Troubleshooting

- **"401 Unauthorized" in the browser console.** Token mismatch —
  re-run `serve --init-token --force` and update localStorage.
- **Explorer pane shows "No current directory".** The server is
  missing `--fs-root`. Restart with `--fs-root <some-dir>`.
- **JobsDashboard shows "No repos yet".** Seed it: `codeless demo
  bootstrap` against the same `--db`.
- **Vite dev URL works but mock banner stays.** The browser shell
  defaults to mock when it can't see a configured base URL. Set
  both localStorage keys above and reload.
- **EventSource disconnects on tab background.** Expected; it
  auto-reconnects with `Last-Event-ID`.
