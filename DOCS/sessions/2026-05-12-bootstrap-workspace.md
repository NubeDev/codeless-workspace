# Build status — Bootstrap: codeless-workspace + repos + mani

> ⛔ **AGENT REMINDER — READ BEFORE TOUCHING THIS FILE**
>
> 1. You are running JOB-LOOP. Spec: [`DOCS/JOB-LOOP.md`](../JOB-LOOP.md).
>    Project scope: [`DOCS/SCOPE.md`](../SCOPE.md). Multi-repo workflow:
>    [`DOCS/MANI.md`](../MANI.md). The rules file `CLAUDE.md` is created
>    by this loop (stage 7); until then, follow SCOPE.md directly.
> 2. **One logical batch per tick.** Read each stage's `[S|M|L]` tag and
>    batch per JOB-LOOP.md "Hard rules" #3: up to 4 contiguous S in one
>    area, OR 1 M (+ optional related S), OR the next sub-stage of an L.
>    Verify + commit + push **each stage** via the bundled mani before
>    moving to the next stage in the batch.
> 3. **You MUST schedule the next tick before exiting** — call
>    `CronCreate` with `recurring: false` for a single fire ~1 min from
>    now. If all stages are `[x]`, report `DONE` instead. If you cannot
>    schedule, **do NOT exit silently** — see JOB-LOOP.md "If you cannot
>    schedule".
> 4. Update this file in the **same commit** as the code change it goes
>    with. Stages that touch multiple repos commit per-repo; this file
>    lives in `codeless-workspace` and is committed alongside whichever
>    `codeless-workspace` change happens in that stage. If a stage only
>    touches `codeless` (the inner repo), commit the inner-repo change
>    AND update this status file in `codeless-workspace` as two commits
>    in the same tick — the status-file commit comes last so it
>    references real pushed work.
> 5. ⛔ **COMMIT _AND_ PUSH BEFORE THE TICK ENDS.** Use the bundled mani
>    at `./bin/mani` from the `codeless-workspace` root. Push targets
>    are per-stage and explicit (see each stage's notes). If push fails,
>    mark the stage `[!]` and halt. Never `--force`, never `--no-verify`.
> 6. ⛔ **CODE COMMENTS ARE LOAD-BEARING.** Explain *why*, never *what*.
>    No emojis, no task-status comments, no decorative banners, no
>    references to stages/ticks/milestones.
> 7. ⛔ **CROSS-PLATFORM REACH IS ENFORCEABLE.** This bootstrap loop
>    only stubs crates; it doesn't add real code yet. But Stage 8's
>    `cargo check --workspace` is the first gate that proves the crate
>    table's iOS-safe / Android-safe columns are buildable in principle.

File: codeless-workspace/DOCS/sessions/2026-05-12-bootstrap-workspace.md
Goal: Stand up the `codeless-workspace` multi-repo workspace as a public
      GitHub repo, move the inner `codeless` repo into it, vendor
      `ai-runner`, fill in `mani.yaml`, stub the Cargo workspace inside
      `codeless/`, and land `CLAUDE.md` — leaving Phase 1 ready to start.
Started: 2026-05-12
Last tick: 2026-05-12 09:32 (Tick B)
Current stage: 4 / 12

Workspace root: /home/user/code/rust/codeless-workspace
mani binary:    ./bin/mani  (bundled, statically linked — use this, not
                $PATH mani, so the workspace is self-contained)
mani config:    ./mani.yaml (currently empty — stage 6 fills it in)

Repos in scope:
  - codeless-workspace   → new repo this loop creates
                           (NubeDev/codeless-workspace, public)
  - codeless             → exists at NubeDev/codeless; currently at
                           /home/user/code/rust/codeless (1 commit:
                           README). Will be moved into the workspace
                           in stage 3.
  - ai-runner            → vendored at codeless-workspace/ai-runner/
                           as a plain directory, no .git. Source of
                           truth remains the rubix-agent crate at
                           /home/user/code/rubix-workspace/rubix-agent/
                           crates/ai-runner/ — re-vendoring is manual.

Branch policy:
  - codeless-workspace:  master (single-line history; this is a tooling
                                  repo, no PRs)
  - codeless:            feat/bootstrap-cargo-workspace
                         (PR-merged back to master at end of loop)

Scheduler:    CronCreate one-shot, ~1 min between ticks
Max ticks:    20

## Known context (before tick 1)

- `gh` is authenticated as `NubeDev` (scopes: repo, workflow, gist,
  read:org). `gh repo create` and `gh repo edit` are both available.
- `codeless-workspace/` already exists as a working directory but is
  **not yet a git repo**. Contents present today: `ai-runner/` (vendored
  source, no .git), `bin/mani`, `mani.yaml` (empty), `DOCS/SCOPE.md`,
  `DOCS/JOB-LOOP.md`, `DOCS/JOB-LOOP-KICKOFF.template.md`,
  `DOCS/MANI.md` (empty), `DOCS/sessions/` (this file).
- `codeless/` still lives at the **old** path `~/code/rust/codeless/`
  with just `README.md` committed to `master`. SCOPE.md and the other
  docs that were once in `codeless/DOCS/` have already been moved into
  `codeless-workspace/DOCS/`.
- `terax-ai` is **not** vendored into the workspace. It stays at
  `~/code/rust/terax-ai/` (a `crynta/terax-ai` clone) as a read-only
  reference for the UI port. This loop doesn't touch it.

## Stages

Format: `[ ] N. [S|M|L] title` — complexity tag is mandatory.
`L` stages must be split into S/M sub-stages before being worked.

- [x] 1. [S] In `codeless-workspace/`, write a top-level `.gitignore`
        covering `target/`, `node_modules/`, `*.db`, `worktrees/`,
        `.env`, `.DS_Store`. **Do not** ignore `bin/` — the bundled
        `mani` binary is intentionally tracked. **Do not** ignore
        `codeless/` either; the inner repo is moved here in stage 3
        and present as a tracked subdirectory (no submodule).
        Verify: `cat .gitignore` shows the expected lines.

- [x] 2. [S] Initialise `codeless-workspace` as a git repo:
        `git init`, set up the initial `master` branch, stage everything
        currently present (DOCS/, bin/, ai-runner/, mani.yaml empty,
        .gitignore from stage 1). Configure user/email if not already
        set. First commit message: `init: codeless-workspace seed
        (DOCS, vendored mani binary, vendored ai-runner)`. **Do not
        push yet** — origin doesn't exist until stage 4.

- [x] 3. [M] Move the inner `codeless` repo into the workspace:
        `mv /home/user/code/rust/codeless /home/user/code/rust/codeless-workspace/codeless`.
        Verified: inner repo still on `master`, remote is
        `NubeDev/codeless`, working tree clean, switched to
        `feat/bootstrap-cargo-workspace`.
        **Correction from original plan**: git treats a nested directory
        with a `.git/` as a submodule candidate (gitlink) by default,
        which is exactly what we don't want. Resolved by **adding
        `codeless/` and `terax-ai/` to the workspace `.gitignore`**.
        Workspace tracks shared tooling only (mani.yaml, DOCS, bin/,
        vendored ai-runner). Inner repos are colocated, not nested.
        Outer commit captures the `.gitignore` update only:
        `chore: ignore inner repos at workspace level (no submodules)`.

- [ ] 4. [S] Create the GitHub repo for the workspace:                ← next
        `gh repo create NubeDev/codeless-workspace --public --source=.
        --remote=origin --description "Codeless multi-repo workspace
        — SCOPE, JOB-LOOP, mani.yaml, vendored ai-runner"`. Push:
        `git push -u origin master`. Verify via
        `gh repo view NubeDev/codeless-workspace --json url` and a
        plain `git remote -v`.

- [ ] 5. [S] Confirm the bundled `mani` binary works:
        `./bin/mani --version` exits 0. If `mani.yaml` is empty, that's
        expected for this stage. Capture the version string in a Notes
        line below. (No commit unless the stage discovers something
        needing recording.)

- [ ] 6. [M] Fill in `codeless-workspace/mani.yaml`. Projects:
          - `codeless`   (path: `codeless`, tag: `rust`, `active`)
          - `ai-runner`  (path: `ai-runner`, tag: `rust`, `vendored`,
                          `reference`)
        Tasks: at minimum, the standard helpers from
        [`DOCS/MANI.md`](../MANI.md) — `status`, `fetch`, `commit`,
        `push`. (If `DOCS/MANI.md` is empty, write a minimal version
        first as a sub-step of this stage; lift the structure from the
        block-flutter-workspace MANI.md the user pointed at.)
        Verify: `./bin/mani --config mani.yaml run status --all` exits
        0 and lists both projects. Commit on the workspace repo:
        `chore(mani): wire up codeless + ai-runner projects`.

- [ ] 7. [M] Create `codeless-workspace/CLAUDE.md` at the workspace
        root (not inside `codeless/`). Capture the rules an agent must
        follow when touching code anywhere in this workspace, distilled
        from `DOCS/SCOPE.md`:
          - Crate dependency direction (mobile depends only on
            `types + client`; the iOS-safe / Android-safe columns in
            the crate table are enforceable by `cargo check`).
          - No `@tauri-apps/api/core` imports in UI modules — only
            `RpcClient`.
          - No `Foo.web.tsx` / `Foo.mobile.tsx` per-shell files
            (Rule 5: one UI framework, forever).
          - Single responsibility per file.
          - Comments: explain *why*, no emojis, no task-status
            comments, no decorative banners.
          - Mani-only for commit/push from JOB-LOOP ticks; use the
            bundled `./bin/mani`.
        Cross-link `DOCS/SCOPE.md`, `DOCS/JOB-LOOP.md`,
        `DOCS/MANI.md`. Commit on workspace repo.

- [ ] 8. [M] Inside `codeless/`, stand up the Cargo workspace at the
        repo root. Outer `Cargo.toml`:
        ```toml
        [workspace]
        resolver = "2"
        members = ["crates/*"]
        ```
        Create stub crates under `crates/`:
          - `codeless-types`      (lib)
          - `codeless-rpc`        (lib)
          - `codeless-runtime`    (lib)
          - `codeless-adapters-host`  (lib)
          - `codeless-server`     (bin: `src/main.rs` with empty `fn main(){}`)
          - `codeless-client`     (lib)
          - `codeless-cli`        (bin)
          - `codeless-tauri-desktop` (bin, but **do not** add the
                                     `tauri` dependency yet — Phase 5)
        Each crate's `lib.rs` / `main.rs` is the minimum that compiles,
        with **one comment** at the top pointing at the SCOPE.md crate
        table row that defines what the crate will contain. No real
        logic, no stub functions. `cargo check --workspace` must pass.
        Commit inside `codeless/`:
        `chore(workspace): cargo workspace + crate stubs per SCOPE`.
        Push the `codeless` branch:
        `./bin/mani --config mani.yaml run push --projects codeless`.
        (Do NOT merge to `master`; PR comes after this loop.)

- [ ] 9. [S] Inside `codeless/`, add a `.gitignore` covering `target/`,
        `*.db`, `worktrees/`, `.env`, OS junk. (Distinct from the
        workspace `.gitignore` — different repo, different concerns.)
        Commit inside `codeless/`, push via mani.

- [ ] 10. [S] Inside `codeless/`, create an empty-ish `CODELESS.md` at
        the repo root: short README-style preamble pointing at the
        workspace-level `DOCS/SCOPE.md`, `DOCS/JOB-LOOP.md`, `CLAUDE.md`
        (note the relative path: `../DOCS/SCOPE.md` from inside the
        inner repo). Commit + push.

- [ ] 11. [S] Patch up internal references in the moved DOCS:
        `DOCS/JOB-LOOP.md` and `DOCS/JOB-LOOP-KICKOFF.template.md`
        currently say things like "paste into a session pointed at
        `/home/user/code/rust/codeless`" — update to point at
        `/home/user/code/rust/codeless-workspace` (for the loop docs
        themselves) or at `codeless-workspace/codeless` (when the
        loop's actual cargo work happens inside the inner repo).
        Also fix the mani commands: `./bin/mani --config mani.yaml run
        commit --projects codeless` (bundled binary, explicit config).
        Commit on workspace repo.

- [ ] 12. [S] Final verify pass:
          - `git -C /home/user/code/rust/codeless-workspace status` clean,
            ahead-by-0 on `master`.
          - `git -C codeless status` clean, branch
            `feat/bootstrap-cargo-workspace` pushed and ahead-by-0.
          - `./bin/mani --config mani.yaml run status --all` shows
            both repos green.
          - `cd codeless && cargo check --workspace` exits 0.
          - `gh repo view NubeDev/codeless-workspace` confirms the
            public repo exists.
        Append a "DONE notes" Notes line summarising what's next
        (Phase 1 proper — the worked example in
        `DOCS/JOB-LOOP-KICKOFF.template.md`).

## Likely batching (planning hint, not a contract)

- **Tick A**: stages 1 + 2 (2×S). `.gitignore` + `git init` + seed
  commit. No remote yet.
- **Tick B**: stage 3 (M). The `mv` is mechanical but error-prone if
  the inner repo isn't clean — give it its own tick.
- **Tick C**: stages 4 + 5 (S + S). `gh repo create`, push, smoke-test
  the bundled mani binary.
- **Tick D**: stage 6 (M). `mani.yaml` (+ minimal `DOCS/MANI.md` if
  empty).
- **Tick E**: stage 7 (M). `CLAUDE.md` — real thinking.
- **Tick F**: stage 8 (M). Cargo workspace + crate stubs inside
  `codeless/`. The first commit on the inner repo's bootstrap branch.
- **Tick G**: stages 9 + 10 + 11 (3×S). Inner `.gitignore`,
  `CODELESS.md`, doc-path fixups.
- **Tick H**: stage 12 (S). DONE.

Expected total: ~8 ticks. If it stretches past 12, halt and reassess.

## Notes

- **Mani binary path**: every mani invocation in this loop uses
  `./bin/mani --config mani.yaml ...` from the workspace root, never
  bare `mani`. The bundled binary is the canonical version for this
  project; `which mani` (system-wide) may be a different build.
- **Two repos, two histories**: `codeless-workspace` and `codeless`
  are independent GitHub repos. The outer repo does not track the
  inner repo's commits. Pull requests against `codeless` are reviewed
  on `codeless`; commits on `codeless-workspace` go straight to
  `master` (tooling repo, no review needed).
- **`ai-runner` is vendored, not submoduled**. Updates require a
  manual re-copy from the rubix-agent workspace. This is the SCOPE.md
  open question "vendor / workspace-dep / fork" resolved (for now)
  in favour of vendoring. Revisit if upstream `ai-runner` diverges.
- **Stage 3's `mv` is destructive**. If `codeless/` has uncommitted
  work at the old path, halt before moving. The pre-flight check
  catches this. (Today, the old path only has `README.md` already
  committed — safe to move.)
- **No Phase 1 work in this loop**. The Cargo workspace is stubs only;
  every crate's body is a single comment pointing at SCOPE.md. The
  real Phase 1 build runs as a separate loop driven by the worked
  example in `DOCS/JOB-LOOP-KICKOFF.template.md`.
- **`codeless-tauri-mobile` is not created** (Phase 6 only).
- **`codeless-adapters-desktop` is not created** (SCOPE.md: "created
  when there is more than one thing to put in it", Phase 5).

## Tick log

- **Tick A (2026-05-12 09:25)** — stages 1 + 2 done. `.gitignore`
  written; `git init -b master` succeeded; user/email configured to
  `NubeDev / ap@nube-io.com`. Seed commit landed (`init: codeless-
  workspace seed (DOCS, vendored mani binary, vendored ai-runner,
  bootstrap session doc)`).
- **Tick B (2026-05-12 09:32)** — stage 3 done with a correction.
  Old codeless tree had a 391-line uncommitted README addition; it
  was committed and pushed as `docs(readme): rationale for forking
  terax-ai and Codeless scope notes` BEFORE the move so we didn't
  carry uncommitted work across paths. Then `mv` succeeded and the
  inner repo was switched to `feat/bootstrap-cargo-workspace`. Git's
  default behaviour staged `codeless/` as a gitlink which we reject;
  resolved by adding `codeless/` + `terax-ai/` to the workspace
  `.gitignore`. Workspace and inner repos are now properly
  independent.

## Blockers

(none)
