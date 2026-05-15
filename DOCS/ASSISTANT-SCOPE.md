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
`CommonChat` component used by both the per-stage job chat and the
top-level assistant page.

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
   - The existing per-stage chat ([`codeless/ui/codeless-ui/src/modules/jobs/StageChat.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/StageChat.tsx)).
   - The full-page job chat ([`codeless/ui/codeless-ui/src/modules/jobs/JobChatPage.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/JobChatPage.tsx)).
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
  kind: "assistant" | "job-stage" | "job-overview";
  // Optional slots: header, right-rail context, empty state.
  slots?: { header?: ReactNode; contextPanel?: ReactNode };
}
```

`CommonChat` owns no business logic about jobs. Callers feed it a
`threadId` and a `kind`; the runner-side handler for that thread kind
decides what tools/actions are available. This keeps R3 (one UI) and
R4 (SQLite as truth) honest.

### 3. Existing job chat call sites

- [`StageChat.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/StageChat.tsx)
  becomes a thin wrapper that resolves the stage's `threadId` and
  renders `<CommonChat kind="job-stage" .../>` with a stage-aware
  header slot.
- [`JobChatPage.tsx`](../codeless/ui/codeless-ui/src/modules/jobs/JobChatPage.tsx)
  becomes a thin wrapper for `kind="job-overview"`.

No behaviour change for existing users — same attachments, same
streaming, same tool cards. The extraction is mechanical; net LOC
goes down.

## Capabilities (what the assistant can do)

The assistant exposes these as runner-side tools. Each tool call
surfaces in the chat as an action card the user confirms before the
runner mutates state.

### View / manage

| Capability | RPC method                                  | Confirmation |
|------------|---------------------------------------------|--------------|
| List jobs  | `jobs.list({ filter })`                     | none (read) |
| Inspect    | `jobs.get(jobId)`                           | none (read) |
| Start      | `jobs.start(jobId)`                         | required    |
| Stop       | `jobs.stop(jobId, { reason })`              | required    |
| Pause      | `jobs.pause(jobId)`                         | required    |
| Resume     | `jobs.resume(jobId)`                        | required    |
| Restart    | `jobs.restart(jobId, { fromStage? })`       | required    |
| Update     | `jobs.update(jobId, { fields })`            | required    |

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

Editing scope on a *running* job follows existing job-state rules
(probably: must be paused; the runner re-reads scope on resume). The
assistant does not bypass those rules.

## Data model additions

- `assistant_threads` table: `id`, `created_at`, `title`, `context`
  (json: selected job id, draft id, etc.).
- `assistant_messages` table: `id`, `thread_id`, `role`, `content`,
  `tool_calls` (json), `attachments` (json: array of attachment ids),
  `created_at`.
- `attachments` table (shared with job chat — likely already exists;
  audit before adding): `id`, `thread_id`, `mime`, `bytes`, `sha256`,
  `created_at`, `display_name`. Stored on disk under the job/thread
  dir; row holds the path.

If `attachments` already exists for job chat, **reuse it**. The whole
point is one upload path.

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
- **R5**: bearer token only. No per-job-author permissions on
  assistant actions.

## Open questions

- Should the assistant be allowed to *create* a job without a draft
  confirmation step if the user explicitly says "just do it"? Bias:
  no — confirmation is cheap and prevents irreversible spend on the
  wrong scope.
- Where do assistant threads live in the job tree? Probably
  workspace-scoped, not per-job, with optional job pinning.
- Image rendering parity: job chat currently renders images inline
  in messages. Confirm the same path works for assistant threads
  before treating it as "free" via extraction.

## Milestones

1. Extract `CommonChat` from `StageChat` + `JobChatPage`; rewire
   both call sites; visual + behavioural parity verified.
2. Add `assistant.*` RPCs + tables; assistant page lists / creates
   threads and chats with a no-op responder.
3. Wire view/manage tools (list/inspect/start/stop/pause/resume/restart/update).
4. Wire draft-from-conversation + create-job confirmation flow.
5. Wire scope edit (inline diff + open-in-editor).

Each milestone ships behind the same UI route; partial completion is
visible but gated by feature presence, not feature flags.
