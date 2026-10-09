# Narad — Agent contract

Applies to **every** coding agent on this repo. Host shims (`CLAUDE.md`, `.cursor/rules/`) only point here.

Narad is a local voice-to-text stack (Rust daemon, GNOME extension, Tauri dashboard). Full playbook: [docs/AI_DEVELOPMENT.md](docs/AI_DEVELOPMENT.md).

## Authority ladder

1. Human instruction for this turn
2. Normative intent: `docs/REQUIREMENTS.md` (FR), `docs/ARCHITECTURE.md` (ADR), `docs/ROADMAP.md` (phase), explicit spike goals
3. Ground truth for *what is*: code + tests + git
4. Structural map: graphify EXTRACTED edges (verify load-bearing edges in source)
5. Descriptive prose (DESIGN tours, old plans, playbooks, READMEs): HINT only

If sources conflict: stop and ask. Do not invent requirements, APIs, or “done” claims.

## Stale resolve

Docs rot. Locate seams in **code** (prefer `graphify query` / `path` / `explain` → `source_location` → Read). If doc ⊥ code on a fact: **code wins**; emit `DRIFT: <doc> vs <file:line>`. Do not rewrite narrative docs unless the task is doc/FR work. Confidence: EXTRACTED | INFERRED | UNKNOWN. UNKNOWN → stop with options; no plausible fiction.

## Operating loop

1. **Scope** — restate the ask in one sentence before editing; refuse extras.
2. **Locate** — FR/ADR/module; if `graphify-out/graph.json` exists, query the graph before broad search.
3. **Plan slice** — smallest change with an observable exit (test, command, or FR id).
4. **Implement** — existing seams only; no drive-by refactors.
5. **Verify** — run the slice checks this turn; evidence = command + result.
6. **Update map** — code: rely on the post-commit graphify hook; run `graphify --update` if docs/structure changed or the hook is absent/failing.

## Close (default slice)

Done only when all hold: scope restated; exit named up front; change at existing seams; verify ran **this turn**; DRIFT recorded if any; conventional commit created (**never push** unless asked). Multi-task: ledger line `Task N: complete (commits …)` before starting N+1.

## Parallelism

Read-only work may run in parallel. **One writer per branch/worktree.** Parallel implementation only via separate worktrees with non-overlapping file claims recorded up front. Multi-task plans: skill `subagent-driven-development` (fresh implementer per task, review package as file, controller coordinates only). No subagent tool → serial inline; do not theater-fake SDD.

## Token doctrine

Ledger/files over chat paste. Graph-first locate. Smallest sufficient context per dispatch.

## Hard rules (Narad)

- **Evidence over assertion.** Runtime/test claims require a command you ran this turn.
- **Cite or mark unknown.** Prefer `file:line` / FR ids.
- **One job per turn** unless the user listed multiple. Finish the named job; leave TODOs for out-of-scope items.
- **No silent scope creep.** New deps, new crates, architecture changes, or phase jumps need an explicit ask.
- **Spec binding.** Behaviour changes map to an FR in `docs/REQUIREMENTS.md` or a stated spike goal. Respect phase boundaries in `docs/ROADMAP.md` unless the user overrides.
- **Deep modules.** Small interfaces; daemon owns state; clients are thin. See architecture §3.
- **Git.** Never start implementation on `main`/`master` without explicit consent. Feature branch for slices. No `git commit --no-verify` unless ordered. Stop and ask before push, merge, release, or destructive git.

## graphify

When `graphify-out/graph.json` exists and the question is structure, call graph, or data flow: `graphify query` / `path` / `explain` before reading the tree. Trust EXTRACTED > INFERRED > AMBIGUOUS; verify important edges in source. If the graph is missing and the tree already has substantial code: offer `/graphify .` once, then continue. `graphify hook install` if `graphify hook status` is not installed.
