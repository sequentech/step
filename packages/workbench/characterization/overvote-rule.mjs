// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the OVER-VOTE rule, layers 1+2 (checker + gates)
// PLUS the recorded tally class per cell — the third column the
// no-silent-discount query needs.
//
// Contest: "Council seat" from the `explicit-blank-invalid` fixture —
// Ada / Bruno / explicit-invalid marker, min_votes=0, max_votes=1. Chosen
// because over-voting it needs only regular candidates (Ada+Bruno), so the
// over-vote condition is not conflated with marker semantics. Blank and
// under policies stay at defaults so the over-vote rule is isolated.
//
// Run:  node characterization/overvote-rule.mjs   (from packages/workbench)
// Output: overvote-rule.recorded.json + overvote-rule.md next to this file.

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {
    loadWasm,
    loadVelvetWasm,
    runChecker,
    runGates,
    tallyClass,
    isSilentDiscount,
    loadMarkerFixture,
    extractErrors,
} from "./harness.mjs"
import {hardGate, softGate, classify, inlineVisible, MSG} from "./spec.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const OVER_POLICIES = [
    "allowed",
    "allowed-with-msg",
    "allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-disable",
]
const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// Vote states bracketing the over-vote condition (max_votes = 1):
//   empty    — 0 selections (blank condition; default blank policy)
//   at_max   — Ada: exactly max — the DISABLE variant's alert case
//   over_max — Ada + Bruno: max + 1 — the over-vote condition proper
const STATES = ["empty", "at_max", "over_max"]

await loadWasm()
await loadVelvetWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_invalid)
)
const regulars = contest.candidates
    .filter((x) => !x.presentation?.is_explicit_invalid)
    .map((x) => x.id)

function makeSelection(state) {
    const picked =
        state === "at_max" ? [regulars[0]] : state === "over_max" ? regulars : []
    return {
        contest_id: contest.id,
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected: picked.includes(c.id) ? 0 : -1,
            write_in_text: null,
        })),
    }
}

function makeEml(overPolicy, invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.presentation = {
        ...(c.presentation ?? {}),
        over_vote_policy: overPolicy,
        invalid_vote_policy: invalidPolicy,
    }
    return clone
}

// ---------------------------------------------------------------------------
// PREDICTION — transcription of the documented rules (VOTE_VALIDATION.md:
// over-vote checker, both gate tables, tally classifier). Independent of
// the implementation by construction; mismatches are findings.
// ---------------------------------------------------------------------------
function predict(over, invalid, state) {
    // rule-specific emissions (the over-vote checker)
    const errors = []
    const alerts = []
    if (state === "over_max") {
        errors.push(MSG.selectedMax) // pushed regardless of policy
        if (over !== "allowed") alerts.push(MSG.selectedMax)
    }
    if (state === "at_max" && over === "not-allowed-with-msg-and-disable") {
        alerts.push(MSG.overVoteDisabled)
    }
    // shared spec composes everything downstream. Council seat: max 1, min 0.
    const selections = state === "empty" ? 0 : state === "at_max" ? 1 : 2
    const selection = state === "empty" ? "none" : "regular"
    const facts = {
        errors,
        alerts,
        explicitInvalid: false,
        selections,
        min: 0,
        max: 1,
        selection,
        policies: {over, invalid},
    }
    return {
        errors,
        alerts,
        hard: hardGate(facts),
        soft: softGate(facts),
        tally: classify({hasErrors: errors.length > 0, selection}),
    }
}

function derivedInlineVisible(observed, over, invalid) {
    return inlineVisible({
        errors: observed.errors,
        alerts: observed.alerts,
        policies: {over, invalid},
    })
}

// ---------------------------------------------------------------------------
const rows = []
for (const over of OVER_POLICIES) {
    for (const invalid of INVALID_POLICIES) {
        const cellEml = makeEml(over, invalid)
        const cellContest = cellEml.contests.find((x) => x.id === contest.id)
        for (const state of STATES) {
            const decoded = runChecker(makeSelection(state), cellEml)
            const {errors, alerts} = extractErrors(decoded)
            const gates = runGates([cellContest], {[contest.id]: decoded})
            const tally = tallyClass(cellContest, decoded)
            const observed = {errors, alerts, ...gates, tally}
            const p = predict(over, invalid, state)
            const match =
                JSON.stringify([errors, alerts, gates.hard, gates.soft, tally]) ===
                JSON.stringify([p.errors, p.alerts, p.hard, p.soft, p.tally])
            rows.push({
                over_vote_policy: over,
                invalid_vote_policy: invalid,
                state,
                observed,
                derived_inline_visible: derivedInlineVisible(observed, over, invalid),
                predicted: p,
                match,
            })
        }
    }
}

const mismatches = rows.filter((r) => !r.match)
console.log(`cells: ${rows.length}, prediction mismatches: ${mismatches.length}`)
for (const m of mismatches) {
    console.log(
        `\nMISMATCH  over=${m.over_vote_policy} invalid=${m.invalid_vote_policy} state=${m.state}`
    )
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}

writeFileSync(
    path.join(here, "overvote-rule.recorded.json"),
    JSON.stringify({contest: contest.name, rows}, null, 2) + "\n"
)

const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")
const fmt = (r) =>
    `| ${isSilentDiscount(r) ? "**⚠** " : ""}${r.over_vote_policy} | ${r.invalid_vote_policy} | ${r.state} | ` +
    `${short(r.observed.errors)} | ${short(r.observed.alerts)} | ${short(r.derived_inline_visible)} | ` +
    `${r.observed.hard ? "**block**" : "—"} | ${r.observed.soft ? "dialog" : "—"} | ` +
    `${r.observed.tally} | ${r.match ? "✓" : "**✗**"} |`
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Over-vote rule characterization — layers 1+2 + recorded tally class",
    "",
    "Generated by `characterization/overvote-rule.mjs`; do not edit by hand.",
    "",
    "**Experiment:** every row is one vote state on the *Council seat* contest",
    "(Ada / Bruno / explicit-invalid marker, `min_votes: 0`, `max_votes: 1`)",
    "under one (over-vote policy × invalid policy) configuration. States:",
    "*empty* = nothing selected; *at_max* = Ada (exactly `max_votes`);",
    "*over_max* = Ada + Bruno (one over). Blank/under policies at defaults.",
    "",
    "Columns: *errors* / *alerts* are the checker record (message keys,",
    "`errors.implicit.` prefix stripped); *hard/soft gate* are the",
    "review-transition gates — evaluated when the voter clicks Next on the",
    "*last* voting page (blocking dialog = review unreachable; dismissible =",
    "may continue); *tally* is the **recorded** class — the counter that incremented",
    "when this exact decoded ballot was run through velvet-wasm's real tally.",
    "`pred?` compares all five observables against the documented rules;",
    "✗ = code and docs disagree. A row whose first cell is prefixed **⚠** is",
    "a **derived** silent-discount marker (convention 3): no booth signal on",
    "any surface yet the tally discards the ballot — the property predicate,",
    "single-sourced from `harness.mjs::isSilentDiscount` and reported in",
    "`no-silent-discount.md`.",
    "",
    "| over_policy | invalid_policy | state | errors | alerts | inline (shown) | hard gate | soft gate | tally | pred? |",
    "|---|---|---|---|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
    "**inline (shown)** — what the voter actually sees inline, after the booth's",
    "master filter (`spec.inlineVisible`). It can differ from the raw *errors*",
    "column: under `invalid_vote_policy = allowed` the filter hides every error",
    "except its keep-list (`selectedMax` unless `over_vote_policy = allowed`;",
    "`blankVote` when `blank = not-allowed`). An error listed in *errors* but",
    "absent here, with no gate, is exactly what a silent **⚠** turns on.",
    "",
    "**Provenance.** *errors*, *alerts*, *hard/soft gate*, *tally* are",
    "WASM-observed and checked cell-by-cell by `pred?`. *inline (shown)* is a",
    "PREDICTION from the shared spec (`spec.mjs`) — `filterErrorList` is",
    "TypeScript, not callable headlessly — validated against the real DOM only",
    "for browser-covered cells (the silent-discount cells, via the e2e runners);",
    "prediction-only elsewhere. A per-cell DOM-validation lane is deferred (see",
    "the e2e-cost note in VALIDATION_LOGIC_DISTILLATION.md §5.3).",
    "",
].join("\n")
writeFileSync(path.join(here, "overvote-rule.md"), md)
console.log(`\nwrote overvote-rule.recorded.json and overvote-rule.md (${rows.length} rows)`)
