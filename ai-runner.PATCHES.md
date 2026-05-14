# `ai-runner/` patches applied by this workspace

The vendored copy of `ai-runner/` is upstream-forked: codeless makes
**local edits** here when an upstream gap blocks a product feature.
Every such edit is listed below. Each entry says what changed, why,
and what upstream needs to absorb so the patch eventually disappears.

This file is the source of truth for "what diverges from upstream."
If you patch `ai-runner/` without adding a row here, you have created
silent drift — the next upstream sync will either drop your fix or
fight it.

## Policy

- `ai-runner/` is **not read-only.** The earlier CLAUDE.md wording
  ("treat as read-only") was aspirational and broke as soon as the
  first product feature (`tool-call` args visibility, see PATCH-001)
  needed a one-line upstream change. The honest model:
  - Edit when blocked.
  - Each edit lands as a focused commit *plus* a row here.
  - Every patch ends with a `// codeless-patch-NNN:` comment so a
    future contributor reading the source knows where it came from.
  - The PR upstream is a follow-up, not a precondition.
- Patches must keep `ai-runner/`'s public API surface intact. If a
  patch needs a new field on a public type, that field is `Option<…>`
  with `#[serde(default)]` so existing callers compile unchanged.
- Sync from upstream: `mani fetch ai-runner` (TBD as a mani task) then
  rebase patches. The list below is the rebase guide.

## Patches

### PATCH-001 — `tool_use` blocks dropped their `input` payload

**File:** `src/runners/claude.rs` (the `"tool_use"` branch of the
`assistant`-event handler).

**Before:** `EventKind::ToolUse { input: None, … }` — the parser read
`block["name"]` but ignored `block["input"]`, so every tool-call event
landed in the codeless event bus with an empty `args_json`. UI rendered
`Bash()`, `Write()`, `Read()` with empty parens; users could see *that*
a tool was invoked but not *what* it was invoked with.

**After:** Forward `block["input"]` as `Some(JsonValue)` when present
on the block. Existing callers see the new payload through the
already-existing `input: Option<JsonValue>` field; no API break.

**Upstream:** straightforward bug fix; PR should land cleanly in
rubix-agent. Until then, codeless carries the patch.

**Marker:** `// codeless-patch-001`

### PATCH-002 — headless `permission_mode` not pluggable

**Files:** `src/types.rs` (new `PermissionMode` enum + `CliCfg`
field), `src/runners/claude.rs` (forward to `claude-wrapper`),
`src/lib.rs` (re-export).

**Before:** `claude-wrapper` defaults to interactive permission
mode — every Write / Bash / Edit pauses for user approval. The
headless codeless server has no TTY user, so every claude job
emitted `tool-call Write(…)` followed by an `ai-token "I need
permission"` and a `job-completed` with zero commits. UI showed
real-looking tool calls; worktrees stayed empty.

**After:** `CliCfg` gains
`permission_mode: Option<PermissionMode>`. The enum is
provider-agnostic (`Default | AcceptEdits | Plan | Bypass`),
mirroring `claude-wrapper::PermissionMode`. When `Some`, the claude
runner calls `cmd.permission_mode(...)` on the upstream
QueryCommand. `None` keeps the wrapper default (interactive),
preserving the pre-patch behaviour for terminal callers.
`codeless-runtime/src/claude_runner.rs` always sets
`Some(Bypass)` — the worktree is the blast radius.

**Upstream:** add the same provider-agnostic enum + field upstream;
straightforward.

**Marker:** `// codeless-patch-002`

### PATCH-004 — built-in tool restriction (`CliCfg::tools` → `--tools`)

**Files:** `src/types.rs` (new `tools: Option<String>` field on `CliCfg`),
`src/runners/claude.rs` (forward to `cmd.tools()`).

**Before:** Spec mode passed tool names through `CliCfg::allowed_tools`,
which the claude runner forwards as `--allowed-tools`. That flag gates
MCP server permissions, not the built-in tool set; Bash remained callable
even when omitted from the list.

**After:** `CliCfg` gains `tools: Option<String>`. When set, the claude
runner calls `cmd.tools(tool_list)`, which generates `--tools` on the
claude binary and actually restricts the available built-in tool set.
The codeless spec-mode chat turn sets this to
`"Read,Edit,Write,Glob,Grep,LS,TodoWrite"` to prevent the agent from
calling Bash, NetFetch, or any other execution tool while authoring a
job spec.

**Upstream:** add the same `tools` field and forward it via `cmd.tools()`.

**Marker:** `// codeless-patch-004`

### PATCH-003 — GitHub Copilot CLI runner

**Files:** `src/types.rs` (new `Provider::Copilot` variant + Display
arm), `src/defaults.rs` (extend the `api_key_for` no-key match arm),
`src/runners/copilot.rs` (new file), `src/runners/mod.rs` (export),
`src/registry.rs` (register in `with_defaults`).

**Before:** No runner for the GitHub Copilot CLI (`copilot`, installed
via `curl -fsSL https://gh.io/copilot-install | bash`). Users with a
Copilot subscription could not pick it as a backend.

**After:** `CopilotRunner` follows the `CodexRunner` shape — spawns
`copilot -p <prompt> --allow-all-tools --no-ask-user --no-auto-update
[-C <work_dir>] [--model <m>]`, streams stdout lines as
`EventKind::Text`. Auth is the binary's responsibility (GitHub device
flow, state in `~/.copilot/`). `--allow-all-tools` is mandatory for
non-interactive mode; `--no-ask-user` prevents the agent from stalling
on a missing TTY; `--no-auto-update` keeps the spawned process from
trying to mutate itself mid-job.

**Caveats:** The Copilot CLI emits plain text (with ANSI in some
modes) rather than a JSONL event stream like Claude Code, so this
runner does not surface structured tool-use events or per-run cost.
Upgrading to richer events would require either parsing
`--log-dir` JSON logs after the fact or wiring the
`--acp` (Agent Client Protocol) server mode — both larger jobs
deferred until there is product demand.

**Upstream:** new feature, not a patch over upstream behaviour. PR
should add the runner unchanged.

**Marker:** none required — entirely new file, no inline patch site.
