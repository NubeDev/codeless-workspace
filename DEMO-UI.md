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

Auth: loopback binds (127.0.0.1 / ::1) skip the bearer-token check
by default — the trust boundary is already same-user same-host
(SCOPE.md R5). To enforce a token locally for testing, add
`--require-token` and `codeless serve --init-token` first.

## Terminal B — UI

```sh
pnpm -C codeless/ui/codeless-ui install   # first time only
pnpm -C codeless/ui/codeless-ui dev
```

Vite serves at `http://127.0.0.1:5173`.

## Browser

Open `http://127.0.0.1:5173`. The browser shell probes the server
at `http://127.0.0.1:7777/healthz` once — if it responds, the UI
uses it. No localStorage to set, no token to paste.

If the server is not running, the UI falls back to its in-memory
`MockRpcClient` and shows a yellow "mock mode" badge in the corner.

## Non-loopback binds

`codeless serve --bind 0.0.0.0:7777` refuses to start unless you
pass `--require-token` — the footgun guard so an unauthenticated
core never ends up reachable from outside the host. With
`--require-token`:

```sh
cargo run -p codeless-cli -- --db /tmp/codeless-demo.db serve --init-token
# copy the token

cargo run -p codeless-cli -- --db /tmp/codeless-demo.db \
    serve --bind 0.0.0.0:7777 --fs-root "$PWD" --require-token
```

Then in the browser DevTools:

```js
localStorage.setItem("codeless-rpc-base-url", "http://your-host:7777");
localStorage.setItem("codeless-rpc-token", "<paste token>");
location.reload();
```

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

## Real runner: Claude Code

The mock runner is enough to drive the UI end-to-end without external
dependencies. To watch a real coding agent edit files, swap it for
Claude Code.

### Prereqs

- The `claude` binary on `PATH`, or its path in the `CLAUDE_BINARY`
  env var. Codeless also discovers it under the usual install
  locations (`~/.local/bin`, `~/.bun/bin`, the VS Code / Cursor /
  Windsurf extension dirs, `/opt/homebrew/bin`, `/usr/local/bin`).
- `claude auth login` run once on this host. The wrapper has its own
  credential cache; Codeless never sees the token.

`codeless-server` probes the binary at boot and surfaces the result
on `GET /server/info` (and in the UI's settings → Models → "Coding
agents" block). If the probe reports "Not installed" or "Not signed
in", fix that before submitting a job.

### Serve with the runner enabled

```sh
cargo run -p codeless-cli --bin codeless -- \
    --db /tmp/codeless-demo.db \
    serve \
    --bind 127.0.0.1:7777 \
    --fs-root "$PWD" \
    --enable-claude
```

`--worktree-root` is not required: when `--fs-root` is set and
`--worktree-root` is not, the server defaults to
`<fs-root>/.codeless/worktrees`. Per-job worktrees live at
`<root>/job-<job_id>` on a fresh branch `codeless/job-<job_id>`. The
worktree directory is reaped on job completion; the durable record
is the branch on the source repo.

The `codeless` repo's `.gitignore` already excludes
`.codeless/worktrees/`. If you point `--fs-root` at a different
checkout, add the same line there.

### Expected timeline

Submit a job in the UI with `runner = claude`. Compared to a mock
run, the timeline grows two extra event kinds:

- `tool-call` events arrive whenever Claude reaches for a tool (file
  Read, Write, Edit, Bash, etc.) — one per call.
- `ai-token` deltas stream the assistant's reply chunks. The mock
  runner emits a few of these for visual parity; a real run emits
  them densely over the run's lifetime.

The terminal sequence on success is `task-started → tool-call*
→ ai-token* → ai-message-complete → task-completed → job-completed`,
with `tool-call` and `ai-token` events interleaved.

### Known limitation — headless tool permissions

`claude-wrapper` defaults to interactive permission mode: tool calls
that touch the filesystem or shell are blocked pending user approval,
and a headless server-side run has no one to approve them. In that
mode you will see the `tool-call` events fire, immediately followed by
an `ai-token` asking the user for permission, and the job completing
without any edits landing on the branch.

The fix needs `claude-wrapper`'s
`QueryCommand::dangerously_skip_permissions()` /
`PermissionMode::BypassPermissions` plumbed through
`ai-runner::CliCfg`. That work is upstream (see the workspace's
[`ai-runner/`](./ai-runner/) directory; per `CLAUDE.md` the inner
tree is treated as read-only and updates flow from the rubix-agent
workspace).

Until that lands, the real-Claude path works for surfaces that do
not require tool execution — e.g. read-only repo Q&A — and the
`scripts/smoke-claude-demo.sh` regression net asserts every step up
to the permission gate. The mock runner remains the canonical way to
drive the UI end-to-end.

### Smoke-test the Claude path

```sh
codeless-workspace/scripts/smoke-claude-demo.sh
```

Bootstraps a fresh `/tmp` repo, starts `codeless serve --enable-claude`,
asserts `/server/info` reports the claude runner with the implicit
worktree-root default, submits a job, polls `get_job` to terminal,
and finally checks that `hello.txt` is committed on the job branch.
Today the final assertion fails because of the permission gate
documented above; the script's value is in catching regressions to
the *plumbing* surface that already works.

## What landed in the UX grind

The Phase 3 "make Codeless feel like a real tool to drive" pass —
11 small landings on master that turn the browser MVP from a wire
demo into something you'd actually leave running.

- **ux-1 — worktrees survive job completion.** A finished job's
  working tree stays on disk so you can `cd` into it and inspect.
  The branch was always durable; this makes the tree durable too.
  SCOPE-compliant default; explicit GC lives at ux-10.
- **ux-2 — job header reads like a debug card.** Real branch,
  preserved worktree path, and the repo's source checkout are
  surfaced together in the detail panel. No more guessing which
  branch a job actually ran on.
- **ux-3 — tool calls pretty-print their args.** The timeline shows
  `Bash(git status)`, `Read(src/main.rs)`, `Write(people.csv)`
  instead of empty parens. Hovering still reveals the full JSON.
- **ux-4 — "Files changed" tab on job detail.** A `job_diff` RPC
  computes the diff between the job branch and the repo's default
  branch, surfaced as a per-file list with adds/deletes counts.
- **ux-5 — `SubmitJobArgs.branch` is honoured.** The wizard's
  branch field is no longer a lie; an empty value falls back to the
  canonical `codeless/job-<id>`. The runtime writes the actually-
  created branch back to the row so the UI never shows a name
  `git` can't resolve. `JobDetail`'s `canonicalBranch()` helper
  went away.
- **ux-6 — `demo bootstrap` detects HEAD.** Probes the repo's
  current HEAD branch with `git symbolic-ref --short HEAD` before
  falling back to `main`. Bootstrapping in a `master`-init'd repo
  no longer silently breaks `job_diff` with `base ref 'main' not
  found`.
- **ux-7 — `rerun_job` RPC.** Clones a previous job's prompt,
  runner, caps, and repo with a fresh `JobId` and queues it; the
  source job is untouched. JobDetail grew a "re-run" button next
  to the cost cell that navigates to the new job on success.
- **ux-8 — timeline grouped by stage and task.** Events fall into
  per-stage cards; per-task blocks coalesce `ai-token` deltas into
  a single rendered "Assistant output" markdown bubble bound to
  its matching `ai-message-complete` cost line. Tool-calls become
  sub-rows of their owning task. A "raw events" toggle drops back
  to the flat ordered stream for debugging.
- **ux-9 — scannable dashboard rows.** Each row carries the
  status pill, a runner badge, the branch, a relative age
  ("3m ago"), the cost, and an italicised one-line activity chip
  projected live from the events stream ("Bash(git status)",
  "verify failed (exit 1)"). One 30s clock on the dashboard
  drives every row's age.
- **ux-10 — `gc_worktrees` RPC + dashboard sweep button.** After
  ux-1, worktrees pile up. The new RPC takes
  `{ older_than_ms, job_ids, dry_run }` and returns per-entry
  size + path + removal status. A "GC worktrees" button on the
  dashboard header opens a confirm modal that **always loads a
  dry-run preview first**: an accidental click can't delete
  anything until the user sees the size and path list. Default
  filter is 7 days.
- **ux-11 — "Open in terminal" affordance.** The worktree row
  on JobDetail offers a "cd" copy button next to "copy" that puts
  a shell-ready `cd '<path>' && git status` on the clipboard.
  Single-quoted so paths with spaces paste safely. No real
  terminal is opened — desktop / browser parity (Rule 3).

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
