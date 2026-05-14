## Done

- (none) — request references "hackline gateway", which does not exist in this workspace (`codeless-workspace`, `codeless/`, `ai-runner/`). No file written to avoid fabricating an API surface.

## Next

- Wait for user to clarify: real source location, a different (real) service in this workspace, or explicit go-ahead to invent a fictional spec.

## What you need to know

- Workspace contains: `codeless-workspace` (docs + mani), inner repo `codeless/` (Rust core + React UI), and vendored `ai-runner/`. No service named "hackline".
- Per CLAUDE.md, half-finished or invented work is disallowed; making up endpoints for a non-existent gateway would violate the "no half-finished implementations" and comment-honesty norms.
- AskUserQuestion is not usable in this headless run, so clarification has to come back as a new prompt.

## Open questions

- Which codebase is "hackline gateway"? Is it a typo for something in `codeless/` (e.g. the runtime's HTTP+SSE RPC surface)?
- If fictional: what scope (auth model, resources, versioning, format — OpenAPI 3.1 YAML vs Markdown)?
