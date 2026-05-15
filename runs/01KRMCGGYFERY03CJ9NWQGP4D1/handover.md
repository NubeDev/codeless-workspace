## Done

- Created `src/modules/jobs/StageChat.tsx` — self-contained chat panel for a single stage tab:
- Subscribes to `{ scope: "job", job_id: stageId }` (agent_chat session) for ai-token/ai-message-complete.
- Subscribes to `{ scope: "job", job_id: jobId }` for `session-archived-then-resumed` lifecycle events, rendered as an inline divider.
- Persists chat to `STAGE-CHAT-<stageId>.md` via `write_job_file`.
- Calls `onChatActive(bool)` when streaming state changes so the parent can drive the tab indicator.
- Stop button calls `cancel_chat_task`; orphaned `waitForCompletion` promise is abandoned via `abortedRef`.
- Modified `StageDetail.tsx`:
- Split layout: existing detail capped at 50% height (ScrollArea), StageChat takes remaining flex space.
- Added `onChatActive?: (active: boolean) => void` prop forwarded to StageChat.
- `ActionBar` gains `hasWarmSession: boolean` — when true, clicking "new session + handover" shows a one-time inline confirm before proceeding.
- Modified `JobTabs.tsx`:
- Added optional `chatStreamingStages?: ReadonlySet<string>` prop.
- Stage tab indicators merge event-derived `TabIndicator` with the streaming signal — shows `●` (`"running"`) when the stage chat is streaming and the tab is not active.
- Modified `JobPage.tsx`:
- Tracks `chatStreamingStages: ReadonlySet<string>` state.
- Passes `handleStageChatActive(stageId, active)` as `onChatActive` to StageDetail.
- Passes `chatStreamingStages` to JobTabs.
- All four files pass `pnpm run typecheck` with zero errors.
- Committed and pushed via mani (commit `3aa1ece`).

## Next

- (none) — this was the final stage (6 of 6) of the job-page UX job.

## What you need to know

- `agent_chat` is stateless per v1 (no `--continue` on the wire). The stage's `capturedSessionId` is accepted as a prop in StageChat but the full transcript is included in each prompt instead. The wire comment in `AgentChatArgs` says a future `previous_session_id` field will enable `--continue`; when that lands the `buildStageChatPrompt` function in StageChat.tsx should use it.
- The `session-archived-then-resumed` event arrives on the JOB event bus (tagged with `job_id`), not on the stage chat session bus (tagged with `stageId`). StageChat therefore subscribes to both streams: one for chat events, one for job lifecycle.
- The tab indicator ● is driven by the `onChatActive` callback chain rather than a second event subscription in JobTabs. This was chosen to avoid per-stage SSE subscriptions in a component that otherwise only needs the one job-scoped stream.
- No test runner is configured yet (`pnpm run test` exits 0 trivially). The stage spec mentioned an e2e test requirement; that remains open.

## Open questions

- End-to-end test for warm/cold path: the spec's verify commands reference `pnpm run test:e2e -- stage-chat` but no test runner or e2e framework exists yet.
- Should the stage chat persist across job re-runs (i.e., the STAGE-CHAT-<stageId>.md filename embeds the stageId UUID, so a new run creates a new stageId and a fresh file)? Current behaviour: each stage has its own file; a rerun creates a new stage and a new file, so old chat is not carried over. This seems correct per the spec's single-tenant model but was not explicitly verified.
