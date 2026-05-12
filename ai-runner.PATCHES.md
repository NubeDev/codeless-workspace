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
