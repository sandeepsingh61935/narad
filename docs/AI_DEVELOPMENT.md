# AI-driven development — Narad playbook

How humans and agents ship Narad quickly without hallucinated progress.

## 1. Why this exists

Agents fail in predictable ways: inventing APIs, widening scope, claiming green checks they never ran, and drowning context with whole-repo dumps. This playbook counters that with **small slices**, **hard sources of truth**, **verification gates**, and a **live code graph**.

## 2. Roles

| Role | Does | Does not |
|---|---|---|
| Human | Sets goal, picks phase/FR, accepts architecture forks, merges | Paste giant context every turn |
| Agent | Implements one scoped slice, verifies, updates docs/graph | Expand phase, invent FRs, skip checks |
| CI / local checks | Ground truth for “works” | Substitute for reading the spec |

## 3. Slice contract (every task)

Before coding, the agent (or you) writes a four-line card:

```
Goal:     <one sentence>
In:       <FR ids or file seams>
Out:      <observable exit: test name, CLI, or manual step>
Not:      <explicit non-goals>
```

Rules:

- Prefer **vertical slices** that match a ROADMAP phase exit over horizontal “plumbing everywhere”.
- One PR / one agent session ≈ one slice. If the card needs “and also…”, split.
- Exit criteria must be **checkable** (“`cargo test -p naradd hotkeys` green”), not vibes (“understanding reached”).

## 4. Anti-hallucination / anti-lie

Positive targets (state what to do):

1. **Read before write** — open the FR and the module seam; then edit.
2. **Quote the source** — when answering “what does X do?”, cite path or graph node; do not paraphrase from memory across sessions.
3. **Run, then claim** — status, tests, portal behaviour, latency: only after a command or measured spike.
4. **Label confidence** — `EXTRACTED` (from code/docs), `INFERRED` (reasonable guess), `UNKNOWN` (stop). Same honesty model as graphify edges.
5. **Fail loud** — if a check cannot run, say so and stop; do not simulate success.

Forbidden patterns agents drift into:

- “Should work” / “probably fine” without a run
- Inventing crate APIs or D-Bus method names not in tree or ARCHITECTURE
- Rewriting unrelated modules “while here”
- Treating chat history as fresher than the repo

## 5. Context management

### Always-loaded (cheap)

- `AGENTS.md` — **cross-tool contract** (Cursor, Claude Code, Grok, Codex, Copilot, …). Edit rules only there.
- Tool adapters are thin pointers to `AGENTS.md`: `CLAUDE.md`, `.cursor/rules/*.mdc` (`alwaysApply`). Do not put policy only in a tool-specific file.
- Active slice card
- Open files related to the seam

### On demand (pointers)

| Need | Open |
|---|---|
| Behaviour / acceptance | `docs/REQUIREMENTS.md` (FR id) |
| Where it lives | `docs/ARCHITECTURE.md` + `graphify query` |
| Sequencing | `docs/ROADMAP.md` phase exit |
| How we work | this file |

### Do not

- Dump entire REQUIREMENTS into the prompt every turn
- Ask the agent to “remember the whole architecture”
- Keep long failed-debug transcripts in the same thread; start a fresh chat with the slice card + failing command output

### Graph as map

After Phase 0+ code exists:

```bash
# First build (or after large moves)
graphify .

# Incremental after edits
graphify --update

# Structure questions
graphify query "How does hotkey activation reach ASR?"
graphify path "hotkeys" "insert"
```

Install once: `graphify hook install` (post-commit AST refresh for **code**). Doc/image-only changes: the agent runs `graphify --update` when those matter to structure answers. Prefer graph query over blind `find`/`grep` for “how does X connect to Y”.

Truth ranking: **code > EXTRACTED graph edges > GRAPH_REPORT god nodes > agent summary**. Graph `INFERRED`/`AMBIGUOUS` edges are hypotheses — verify in source. Descriptive prose in this playbook is HINT only (see `AGENTS.md` authority ladder).

## 6. Checks ladder

Match the check to the slice; climb only as needed.

| Level | When | Example |
|---|---|---|
| 0 Compile/type | Any code touch | `cargo check -p …` |
| 1 Unit | Pure logic | module tests |
| 2 Integration | IPC / store | D-Bus method + SQLite |
| 3 Spike / manual | Portals, GNOME, mic | Phase 0 scripts; note hardware |
| 4 Phase exit | End of ROADMAP phase | checklist in ROADMAP |

Agent response after a slice should include: commands run, pass/fail, and leftover risk (especially unknown on Wayland/portals).

## 7. Session shapes (fast mode)

**Implement** — slice card → code → level-appropriate checks → commit. Graph: hook on commit for code; `graphify --update` if docs/structure changed or hook absent.

**Explore** — graph query / ARCHITECTURE only; no edits unless asked.

**Diagnose** — failing command first; bisect to seam; fix only the failing cause.

**Spike** — throwaway under an agreed path; measure; write numbers into docs; delete or quarantine spike code before MVP polish.

Use separate chats per shape when possible so context stays on-rails.

## 8. Human prompt habits that keep agents honest

- Name the FR: “Implement FR-ACT-1 hold/tap only; Not: wake word.”
- Demand evidence: “Show `cargo test` output.”
- Cap blast radius: “Touch only `crates/naradd/src/hotkeys/`.”
- Force unknown: “If the portal API is unclear, stop with options; do not invent.”
- Prefer Plan for architecture forks; Agent for a locked slice.

## 9. Bootstrap checklist (this repo)

- [x] Product docs: REQUIREMENTS, ARCHITECTURE, ROADMAP
- [x] `AGENTS.md` as single agent contract + this playbook
- [x] Tool adapters pointing at `AGENTS.md`: `CLAUDE.md`, `.cursor/rules/*.mdc`
- [x] Workspace scaffold (`crates/*`, `spikes/*`, `extension/`)
- [x] Phase 0 spike code landed → `graphify update .` (AST graph) + `graphify hook install`
- [x] `.gitignore` covers `.superpowers/` (SDD scratch)
- [ ] Per-crate `cargo`/`pnpm` scripts as packages appear — document in package READMEs, not here
- [ ] CI mirrors the checks ladder for the packages that exist

## 10. Phase discipline

Ship in ROADMAP order. Agents may propose jumping phases; humans approve. Phase 0 spikes stay disposable until numbers land in REQUIREMENTS / ARCHITECTURE open decisions.

## 11. Close rule (default slice)

A slice is closed only when all hold:

1. Scope restated in one sentence; extras refused
2. Observable exit named up front (test | command | FR id)
3. Change at existing seams only
4. Verify ran **this turn**; evidence = command + result
5. `DRIFT:` lines if doc ⊥ code
6. Conventional commit on a feature branch — **never push** unless asked
7. Map: post-commit hook for code; `graphify --update` if docs/structure changed or hook failing
8. Multi-task: ledger `Task N: complete (commits …)` before N+1

## 12. Git and worktrees

- Never start implementation on `main`/`master` without explicit human consent.
- Default: feature branch on the current clone for a single slice.
- Worktree per long-lived plan, or when parallel slices need isolation:

```bash
git worktree add ../narad-wt-<slug> -b feat/<slug>
```

Record non-overlapping file claims **before** two writers run. One writer per worktree. Agent commits per closed task; no `--no-verify` unless ordered. Stop and ask before push, merge, release, or destructive git.

## 13. Subagent-driven development (multi-task)

Use when the human says “plan” **or** the work has >1 independent tasks.

**If a subagent dispatch tool exists:** follow skill `subagent-driven-development` fully. Slim controller card:

- Isolated worktree; ledger at `.superpowers/sdd/<plan>/progress.md` (gitignored)
- Fresh implementer per task; **controller does not freestyle-fix**
- Review package as a **file**; spec compliance **and** quality both required
- Fix loop cap 5; every ruling in the ledger
- Final whole-branch review on the most capable model; then delete that plan’s workspace
- No pause for “should I continue?”

**If the host has no subagent tool:** execute the plan **serially inline** with the same close/review discipline. Do not theater-fake SDD.

## 14. Review expectations

| Gate | When | Looks at |
|---|---|---|
| Implementer self-review | Before reporting DONE | Own diff + tests just run |
| Per-task review | After each SDD task | Spec ✅/❌ **and** quality; package is a file |
| Whole-branch review | After all tasks (or before merge) | Diff vs merge-base; triage deferred minors |

Self-review never replaces the task gate. Residual load-bearing findings surface to the human; do not push or merge to settle them.
