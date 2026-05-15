# Assistant — Scope

Status: draft
Owner: ap@nube-io.com
Created: 2026-05-15

## Summary

The **Assistant** is an in-app conversational surface that lets a user
view, manage and create Codeless jobs by talking to the same runner
that powers headless tick execution. It is *not* a second runner; it
is a thin UI + RPC client over `codeless-runtime`. The assistant
reuses the existing job chat UI by extracting it into a shared
`CommonChat` component used by both the in-job chat and the top-level
assistant page.

The job chat today lives as the `JobChat` export in
[`codeless/ui/codeless-ui/src/modules/jobs/RunPane.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/RunPane.tsx)
(rendered both inline from `RunPane` and full-page from
[`JobChatPage.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/JobChatPage.tsx)).
A separate surface, [`modules/ai/components/AiChat.tsx`](../codeless/ui/codeless-ui/src/modules/ai/components/AiChat.tsx)
with [`modules/ai/store/chatStore.ts`](../codeless/ui/codeless-ui/src/modules/ai/store/chatStore.ts),
predates the job chat and is used by the in-editor AI panel. The
extraction below folds **both** into `CommonChat`; see §6.

Out of scope for this doc: the agent itself (planner, tools, prompts),
auth/multi-tenant, mobile-specific behaviour beyond what R1–R5 in
[`CLAUDE.md`](../CLAUDE.md) already require.

## Goals

1. One place a user can:
   - List, filter and inspect jobs.
   - Start / stop / pause / resume / restart jobs.
   - Edit a job's scope (the markdown brief the runner consumes).
   - Create new jobs from a conversation (intent → draft → confirm).
2. Reuse the job chat as a `CommonChat` component — same look, same
   attachments, same image upload, same streaming behaviour — in:
   - The assistant page.
   - The in-job chat (`JobChat` exported from
     [`RunPane.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/RunPane.tsx),
     rendered inline by `RunPane` and full-page by
     [`JobChatPage.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/JobChatPage.tsx)).
   - The in-editor AI panel
     ([`AiChat.tsx`](../codeless/ui/codeless-ui/src/modules/ai/components/AiChat.tsx)).
3. All transport flows through `RpcClient` (R2) — no Tauri or `fetch`
   leaks into the UI.
4. SQLite remains the source of truth (R4) — assistant conversations,
   attachments and any drafts are persisted server-side and replayed
   via `RpcClient.subscribe()`.

## Non-goals

- A new agent runtime. The assistant calls into the existing runner
  (planner + tools) via `RpcClient`.
- Per-user permissions. Single-tenant trust boundary (R5) holds.
- A second chat UI. If `CommonChat` can't express something a caller
  needs, the caller extends via props/slots — it does not fork.

## Surfaces

### 1. Assistant page

Route: `/assistant` (sibling of `/jobs`).

Layout:

- Left rail: thread list (recent assistant conversations).
- Main: `CommonChat` instance bound to the selected assistant thread.
- Right rail (collapsible): "context panel" showing what the
  assistant is currently looking at — selected job, draft scope, etc.
  — so the user can see what state actions will mutate.

The assistant page is a *conversation* first; structured actions
(start/stop/edit scope) appear inline as message-attached action cards
that the user confirms before the runner executes them. No silent
side-effects from chat.

### 2. CommonChat (extracted shared component)

Lives at `codeless/ui/codeless-ui/src/modules/chat/` (new module).

Responsibilities:

- Render message list (user / assistant / tool / system roles).
- Streaming token rendering with the existing event format
  ([`eventFormat.ts`](../codeless/ui/codeless-ui/src/modules/jobs/eventFormat.ts)).
- Composer with:
  - Multiline text input.
  - **Attachments**: drag-drop + file picker, same set as job chat
    (images: png/jpg/jpeg/gif/webp; text/code; pdf). Attachments
    upload via `RpcClient.uploadAttachment(threadId, file)` and are
    referenced by id in the outgoing message.
  - **Image paste** from clipboard.
  - Send / stop-stream button.
- Tool-call cards (approval, diff review, todo strip) — reused from
  `modules/ai/components/`.
- Scroll anchoring, "jump to latest", unread divider.

Props (sketch):

```ts
interface CommonChatProps {
  threadId: string;
  client: RpcClient;
  // What kind of thread this is — drives which action cards render,
  // which tools the runner is allowed to call, and the placeholder text.
  kind: "assistant" | "job-overview" | "ai-panel";
  // Optional slots: header, right-rail context, empty state.
  slots?: { header?: ReactNode; contextPanel?: ReactNode };
}
```

`CommonChat` owns no business logic about jobs. Callers feed it a
`threadId`; capabilities are derived **server-side** from the thread
row (`assistant_threads.kind` / equivalent on job threads), not
trusted from the client. The `kind` prop is UI-only — it picks
placeholder copy, header affordances, and which action-card
renderers to mount. A malicious client passing the wrong `kind`
must not be able to invoke tools the runner wouldn't otherwise
allow on that thread.

### 3. Existing chat call sites

- `JobChat` in [`RunPane.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/RunPane.tsx)
  becomes a thin wrapper that resolves the job's `threadId` and
  renders `<CommonChat kind="job-overview" .../>` with a job-aware
  header slot. The inline render from `RunPane` and the full-page
  render from [`JobChatPage.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/JobChatPage.tsx)
  both go through this wrapper.
- [`AiChat.tsx`](../codeless/ui/codeless-ui/src/modules/ai/components/AiChat.tsx)
  becomes a thin wrapper for `kind="ai-panel"`. Its current
  client-side state in
  [`chatStore.ts`](../codeless/ui/codeless-ui/src/modules/ai/store/chatStore.ts)
  must move server-side to satisfy R4; the store either disappears
  or shrinks to UI presentation state (scroll position, composer
  draft) only.

No behaviour change for existing users — same attachments, same
streaming, same tool cards. The extraction is mechanical.

## Capabilities (what the assistant can do)

The assistant exposes these as runner-side tools. Each tool call
surfaces in the chat as an action card the user confirms before the
runner mutates state.

### View / manage

| Capability | RPC method                                  | Confirmation |
|------------|---------------------------------------------|--------------|
| List jobs  | `jobs.list({ filter })`                     | none (read) |
| Inspect    | `jobs.get(jobId)`                           | none (read) |
| Start      | `jobs.start(jobId)`                         | confirm: job id + entry stage |
| Stop       | `jobs.stop(jobId, { reason })`              | confirm: reason text |
| Pause      | `jobs.pause(jobId)`                         | confirm: job id + current stage |
| Resume     | `jobs.resume(jobId)`                        | confirm: job id + resume point |
| Restart    | `jobs.restart(jobId, { fromStage? })`       | confirm: stages that will be discarded |
| Update     | `jobs.update(jobId, { fields })`            | confirm: per-field diff of changed fields |

All methods already exist or are planned in
[`DOCS/JOB-MODEL.md`](./JOB-MODEL.md); the assistant does not invent
its own job model.

### Create

`jobs.draftFromConversation(threadId) → DraftJob` returns the
proposed `JobSpec` (repo, branch, scope markdown, runner choice).
The user reviews the draft inline (a diff-style card) and confirms
via `jobs.create(draft)`. No job is created until the user clicks
Confirm.

### Edit scope

The scope is a markdown document
([`DOCS/JOB-DIR.md`](./JOB-DIR.md) defines the layout). The
assistant offers two edit modes:

1. **Inline edit** — the assistant proposes a unified diff against
   the current scope; the user accepts/rejects hunks in a
   `PlanDiffReview`-style card; accepted hunks are written via
   `jobs.updateScope(jobId, newMarkdown)`.
2. **Open in editor** — opens the scope file in the existing editor
   tab. No assistant mediation; same `RpcClient` write path.

Editing scope on a *running* job follows existing job-state rules:
the job must be paused before scope can be written, and the runner
re-reads scope on resume. The assistant surfaces the pause as an
action card if the user requests a scope edit on a running job; it
does not bypass the rule.

## Data model additions

- `assistant_threads` table: `id`, `created_at`, `title`, `context`
  (json: selected job id, draft id, etc.).
- `assistant_messages` table: `id`, `thread_id`, `role`, `content`,
  `tool_calls` (json), `attachments` (json: array of attachment refs),
  `created_at`.

### Attachments — current state and required change

Job chat attachments are **filesystem-backed, not table-backed**.
The runner writes uploads under the job's worktree at
`.codeless/chat-attachments/` (see
[`crates/codeless-runtime/src/rpc/chat.rs`](../codeless/crates/codeless-runtime/src/rpc/chat.rs)
around line 182), and outgoing messages carry the attachment refs
inline. There is no `attachments` table to reuse.

Assistant threads are workspace-scoped, not job-scoped, so they
have no worktree to write into. Two options, pick one before
milestone 1:

1. **Add a workspace-scoped attachments dir** (e.g.
   `<codeless-data>/threads/<thread_id>/attachments/`) and keep
   the filesystem-backed model. Job chat keeps writing to the
   worktree; assistant chat writes to the workspace dir. The
   `RpcClient.uploadAttachment(threadId, file)` shape is the same;
   the runner picks the directory from the thread's kind.
2. **Promote attachments to a SQLite table** (`id`, `thread_id`,
   `mime`, `bytes`, `sha256`, `created_at`, `display_name`,
   `disk_path`) and migrate job chat to it. Larger blast radius,
   but means one path forever.

Bias: option 1. It's strictly additive and doesn't touch the
existing job-chat code. Resolve in the design review before
milestone 1 lands.

## RPC additions

Extend `RpcClient`:

```ts
interface RpcClient {
  // existing methods...
  assistant: {
    listThreads(): Promise<AssistantThread[]>;
    createThread(title?: string): Promise<AssistantThread>;
    deleteThread(id: string): Promise<void>;
  };
  uploadAttachment(threadId: string, file: File): Promise<Attachment>;
}
```

`messages` and `subscribe` are already generic over thread id; the
assistant uses the same channels as job chat. This is what makes
`CommonChat` actually shareable.

## Cross-cutting rules (must hold)

- **R1**: nothing the assistant does spawns processes from the UI.
  All `process::Command` use stays in `codeless-adapters-host`.
- **R2**: only `RpcClient`. The assistant page does not import
  `@tauri-apps/api` or call `fetch` directly.
- **R3**: no `Assistant.web.tsx` / `Assistant.mobile.tsx`. One
  responsive component.
- **R4**: assistant threads, messages, attachments and drafts live
  in SQLite. The UI subscribes; it does not cache authoritative
  state.
- **R5**: single-tenant trust boundary. The bearer token authorises
  the assistant identically to every other client; no per-job or
  per-action scopes are introduced.

## Open questions

- Should the assistant be allowed to *create* a job without a draft
  confirmation step if the user explicitly says "just do it"? Bias:
  no — confirmation is cheap and prevents irreversible spend on the
  wrong scope.
- Where do assistant threads live in the job tree? Probably
  workspace-scoped, not per-job, with optional job pinning. This
  decision drives the attachments directory choice in §Data model
  and the shape of `assistant_threads.context`.
- Image rendering parity: job chat currently renders images inline
  in messages. Confirm the same path works for assistant threads
  before treating it as "free" via extraction.

## Milestones

1. Resolve the attachments-directory question (§Data model) and the
   `AiChat` server-state migration (§Surfaces 3). Both are
   prerequisites for the extraction.
2. Extract `CommonChat` from `JobChat` (in `RunPane.tsx`) and
   `AiChat`; rewire all three call sites (`RunPane`, `JobChatPage`,
   AI panel); visual + behavioural parity verified.
3. Add `assistant.*` RPCs + tables; assistant page lists / creates
   threads and chats with a no-op responder.
4. Wire view/manage tools (list/inspect/start/stop/pause/resume/restart/update).
5. Wire draft-from-conversation + create-job confirmation flow.
6. Wire scope edit (inline diff + open-in-editor).

Each milestone ships behind the same UI route; partial completion is
visible but gated by feature presence, not feature flags.
