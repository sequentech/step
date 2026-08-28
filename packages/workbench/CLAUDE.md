<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# CLAUDE.md — packages/workbench

Guidance for agents working in this package. The project documents itself
thoroughly — this file only routes, and records what is **not** written
anywhere else (conventions and gotchas learned while building it). If
something you need is missing here, it is in the documents below, on
purpose.

## Read first (routing)

- [README.md](README.md) — what the workbench is, how to run/build it,
  the embedding strategy, known gaps, and the forward agenda ("What's
  next").
- [characterization/README.md](characterization/README.md) — the
  validation-characterization suite: intent, conventions, harness, how to
  run every tool and what each produces, coverage status.
- [docs/](docs/) — the vote-validation deep dives and the findings.
  `UPSTREAM_FINDINGS.md` is the escalation artifact; `REPRODUCE.md` the
  reviewer recipes; each doc states its own role at the top.
- [WORKBENCH.md](WORKBENCH.md) / [LIFTING.md](LIFTING.md) /
  [LIFTING-TALLY.md](LIFTING-TALLY.md) — workbench design and the lift
  procedure; **LIFTING.md wins over WORKBENCH.md on any lift fact**.
- The monorepo-level `../../CLAUDE.md` covers repo-wide build/test/PR
  conventions; it applies here too.

## Non-negotiable conventions (operator-enforced, not in the docs)

- **Never rename anything that originates in production code** — Rust/TS
  identifiers, message keys, and **JSON field names in fixtures** alike.
  If production calls it `X`, the workbench calls it `X`. Workbench-only
  code may be named freely.
- **Generated tables are views, never sources.** Do not hand-edit
  `characterization/*.md` tables or `*.recorded.json` — change the runner
  and regenerate. (The seven per-rule tables are rendered by
  `rule-tables.mjs`; they are documentation, not evidence.) A display-only runner change must leave the recorded
  JSONs byte-identical; check that after regenerating (an unchanged
  `git status` on the JSON is the proof the change was display-only).
- **Narrative meets the table standard.** Prose claims in findings and
  docs face the same falsification bar as table cells (counterfactual,
  identity, and definitional-fit tests) — the narrative is read first,
  and if it is wrong it does not matter that the tables are right.
- **Reference discipline.** Never cite a bare "row 46" or "§4.5" — name
  the artifact it lives in ("`overvote-rule.md` row 46",
  "VALIDATION_LOGIC_DISTILLATION.md §4.5"), in chat and in docs.
- **Terminology carries cognitive load — pay it down at first use.** A
  term that a reader cannot identify with a concrete piece of code or
  functionality ("surface", "observation context", "lane") must be
  defined where the document first uses it, or replaced by the concrete
  thing it names. Prefer concrete over abstract wherever concrete will
  do; a definition given three sections later does not count.
- **The `// FIXME` lines in UPSTREAM_FINDINGS.md are faithful quotes of
  production source** — they are the *subject* of a finding. Do not
  "fix", reword, or remove them.
- **Three kinds of tool, and never blur them.** Every file in
  `characterization/` is exactly one of: **evidence** (touches real
  production; a failure means fidelity is broken), **analysis** (consumes
  the certified spec, never touches production; a failure means a finding
  stopped being derived), or **documentation** (cannot fail). The test is
  mechanical — if it doesn't touch production, it is not evidence. So an
  analysis runner must never load the wasm or open a browser, and a
  documentation generator must never grow a comparison column: duplicating
  the sweep's check there makes the story look like it has independent
  checks it does not have. See characterization/README.md, "Three kinds of
  tool".
- **The certified domain is defined once**, in
  `characterization/domain.mjs` — the enumeration and the membership
  predicate together, the predicate derived from the enumeration's own
  lists. It was previously written six times and one copy silently
  disagreed, so a runner claimed certification over cells the sweep never
  visited. Never re-enumerate it locally; import it.
- **Adjudication is nobody's alone — but fix decisions are ours to make.**
  Two tracks, don't conflate them (clarified by the operator, 2026-08-28).
  *Characterization track:* surprising production behaviour is recorded as
  a *suspect*; whether it is intended is documented as an open question
  (the three-state model — characterization/README.md, Conventions).
  *Fix track:* what the rationalized reference (`validation-spec/`) does
  about each quirk is decided **autonomously** here and recorded in the
  fix ledger; the upstream pull-request review is where those judgments
  are finally adjudicated. A fix decision stands until overturned there —
  do not defer it to a consultation, and do not act on it again except
  reactively, if the review rejects it.

## Working practices that fit this project

- **Verify before asserting.** Ground every claim about code or behaviour
  in a fresh read of the source or a recorded artifact, not recollection.
  When the operator challenges a claim, re-derive it from source rather
  than defending it — and concede precisely when wrong. (This session's
  history: several plausible-sounding claims were exactly backwards until
  the code was read.)
- **One focused, checkpointed unit at a time.** Commit at each checkpoint
  with a narrative message (what, why, what was verified); confirm scope
  with the operator before rolling into the next item.
- **Docs rot same-day here — sweep them when tools change.** When a
  runner's outputs change (a column, a method, a total), grep the written
  docs for the old value in the same session: cell totals appear in
  multiple documents, and an evidence pointer must name an artifact that
  actually carries the cited column. (A cross-doc total went stale within
  one day during the 2026-08 work.)

## Gotchas

- **Policy-overrides panel state accumulates per contest.** A browser
  runner covering several rules on a shared contest inherits the previous
  rule's overrides unless it reloads between rules and sets every varied
  policy *and* the bounds explicitly — see the comment in
  `characterization/dom-validate.mjs`. Related: `page.goto` under Vite
  wipes the ephemeral overrides entirely (reviewer-path tools must
  navigate client-side; `characterization/browser-harness.mjs` header).
  Two more faces of the same trap (found building
  `characterization/browser-witnesses.mjs`): the review screen renders
  EVERY contest and `warnIds` reads the whole page, so a runner touching
  multiple contests must neutralize the non-target contest (defaults +
  fixture bounds) each cell or its warnings pollute the observation; and
  navigating contest-page → contest-page used to race `setPanelConfig`'s
  wait-for-select against the outgoing page's identical panel, misrouting
  the first select onto the wrong contest — fixed inside `setPanelConfig`
  (it hops through the inspector), so don't re-introduce a direct wait.
  Third face: inline warnings render from component-local decode state
  that lags the last click, so reading `warnIds` immediately races it —
  read at quiescence (two consecutive reads agreeing;
  `characterization/booth-cell.mjs` `stableWarnIds`), never after a fixed
  sleep, and never immediately.
- **PowerShell tool sessions on this machine may lack `node` on PATH**;
  refresh with
  `$env:Path = [Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [Environment]::GetEnvironmentVariable("Path","User")`
  or use the Bash tool.
- **A shell pipeline hides a runner's exit code.** `node x.mjs | tail`
  reports *tail's* status, so a failing runner looks green. Redirect to a
  file and check `$?` when the exit code matters (it cost a false "the
  runner failed to fail" diagnosis once).
- **`git add -A` stages from the repo root regardless of cwd**, which here
  sweeps in the untracked `packages/vscode_target/` build tree and times
  out, leaving a stale `.git/index.lock`. Stage explicit paths.
- **The byte-identical oracle.** When a change is meant to be
  behaviour-preserving, regenerate and require the recorded artifacts to
  come back byte-identical; that is the check, not a green run. It has
  caught what green runs could not — a single field on one row, during the
  step-3b refactor, on a cell whose columns are not compared.
- **Harmless commit noise:** every commit here prints CRLF conversion
  warnings and a "too many unreachable loose objects" gc warning; neither
  indicates a problem.
