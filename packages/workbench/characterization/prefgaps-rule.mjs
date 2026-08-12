// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the PREFERENCE-GAPS rule (preferential only), layers
// 1+2 + recorded tally class.
//
// Reuses the existing `instant-runoff-3cand.json` fixture (Apple / Banana /
// Cherry, IRV, max_votes 3). In a preferential contest `selected` encodes
// the rank: 0 = rank 1, 1 = rank 2, …, -1 = unranked. A gap in the ranks
// (two candidates sharing a rank value) makes validate_preferencial_order
// return PreferenceOrderWithGaps, and the decode calls check_preference_gaps_policy.
//
// Structural point under test: `EPreferenceGapsPolicy` has only two
// variants and BOTH end in _WARN_AND_DIALOG — there is no silent "allowed"
// variant. So whenever a gap is present a gate always fires
// (dismissible under ALLOWED_WARN_AND_DIALOG, blocking under
// NOT_ALLOWED_WARN_AND_DIALOG), regardless of invalid_vote_policy. The
// prediction is therefore ZERO silent-discount cells — the opposite of the
// over-vote/min-vote families, whose rules can be configured into silence.
//
// Run:  node characterization/prefgaps-rule.mjs   (from packages/workbench)

import {writeFileSync} from "node:fs"
import {readFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {
    loadWasm,
    loadVelvetWasm,
    runChecker,
    runGates,
    tallyClass,
    isSilentDiscount,
    extractErrors,
} from "./harness.mjs"
import {hardGate, softGate, classify, inlineVisible, MSG} from "./spec.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const GAP_POLICIES = ["allowed-warn-and-dialog", "not-allowed-warn-and-dialog"]
const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// ranked selections (selected = rank; -1 unranked):
//   valid_full — 0,1,2 : a complete, well-ordered ranking (no order error)
//   gap        — 0,2,-1: ranks 0 and 2, skipping rank 1 (PreferenceOrderWithGaps)
const STATES = ["valid_full", "gap"]

await loadWasm()
await loadVelvetWasm()
const snap = JSON.parse(
    readFileSync(
        path.resolve(here, "../app/src/fixtures/snapshots/instant-runoff-3cand.json"),
        "utf8"
    )
)
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests[0]
const ids = contest.candidates.map((c) => c.id) // [Apple, Banana, Cherry]

function makeSelection(state) {
    const ranks =
        state === "valid_full"
            ? [0, 1, 2]
            : [0, 2, -1] // rank 0 then rank 2, skipping rank 1 (a gap)
    return {
        contest_id: contest.id,
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: ids.map((id, i) => ({id, selected: ranks[i], write_in_text: null})),
    }
}

function makeEml(gapPolicy, invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.presentation = {
        ...(c.presentation ?? {}),
        preference_gaps_policy: gapPolicy,
        invalid_vote_policy: invalidPolicy,
        // duplicated_rank at default; a pure gap does not trip duplicates
    }
    return clone
}

// PREDICTION — rule-specific emissions composed with the shared spec.
// check_preference_gaps_policy pushes a `preferenceOrderWithGaps` error on a
// gapped ranking; gates/classifier/filter are the shared spec.
function predict(gap, invalid, state) {
    const errors = []
    if (state === "gap") errors.push(MSG.preferenceOrderWithGaps)
    const alerts = []
    const selections = makeSelection(state).choices.filter(
        (c) => c.selected === 0
    ).length
    const facts = {
        errors,
        alerts,
        explicitInvalid: false,
        selections,
        min: contest.min_votes ?? 0,
        max: contest.max_votes,
        selection: "regular",
        policies: {gap, invalid},
    }
    return {
        errors,
        alerts,
        hard: hardGate(facts),
        soft: softGate(facts),
        tally: classify({hasErrors: errors.length > 0, selection: "regular"}),
    }
}

function derivedInlineVisible(observed, invalid) {
    return inlineVisible({
        errors: observed.errors,
        alerts: observed.alerts,
        policies: {invalid},
    })
}

const rows = []
for (const gap of GAP_POLICIES) {
    for (const invalid of INVALID_POLICIES) {
        const cellEml = makeEml(gap, invalid)
        const cellContest = cellEml.contests.find((x) => x.id === contest.id)
        for (const state of STATES) {
            const decoded = runChecker(makeSelection(state), cellEml)
            const {errors, alerts} = extractErrors(decoded)
            const gates = runGates([cellContest], {[contest.id]: decoded})
            const tally = tallyClass(cellContest, decoded)
            const observed = {errors, alerts, ...gates, tally}
            const p = predict(gap, invalid, state)
            const match =
                JSON.stringify([errors, alerts, gates.hard, gates.soft, tally]) ===
                JSON.stringify([p.errors, p.alerts, p.hard, p.soft, p.tally])
            rows.push({
                preference_gaps_policy: gap,
                invalid_vote_policy: invalid,
                state,
                observed,
                derived_inline_visible: derivedInlineVisible(observed, invalid),
                predicted: p,
                match,
            })
        }
    }
}

const mismatches = rows.filter((r) => !r.match)
console.log(`cells: ${rows.length}, prediction mismatches: ${mismatches.length}`)
for (const m of mismatches) {
    console.log(`\nMISMATCH gap=${m.preference_gaps_policy} invalid=${m.invalid_vote_policy} state=${m.state}`)
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}
const flagged = rows.filter(isSilentDiscount)
console.log(`silent-discount cells: ${flagged.length} (expected 0 — the policy has no silent variant)`)

writeFileSync(
    path.join(here, "prefgaps-rule.recorded.json"),
    JSON.stringify({contest: contest.name, rows}, null, 2) + "\n"
)

const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")
const fmt = (r) =>
    `| ${isSilentDiscount(r) ? "**⚠** " : ""}${r.preference_gaps_policy} | ${r.invalid_vote_policy} | ${r.state} | ` +
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
    "# Preference-gaps rule (preferential) — layers 1+2 + recorded tally class",
    "",
    "Generated by `characterization/prefgaps-rule.mjs`; do not edit by hand.",
    "",
    "**Experiment:** every row is one ranked selection on the IRV *Favourite",
    "fruit* contest (Apple / Banana / Cherry; `selected` = rank, 0-based)",
    "under one (`preference_gaps_policy` × `invalid_vote_policy`) config.",
    "States: *valid_full* = ranks 0,1,2 (well-ordered); *gap* = ranks 0,2",
    "(skipping rank 1 → PreferenceOrderWithGaps). `duplicated_rank` at default.",
    "",
    "Columns as in the other rule tables; *tally* is the **recorded** class.",
    "A row prefixed **⚠** is a derived silent-discount marker.",
    "",
    "**Result: zero silent-discount cells** — and that is the finding.",
    "`EPreferenceGapsPolicy` has only `*_WARN_AND_DIALOG` variants (no silent",
    "`allowed`), so whenever a gap is present a gate always fires:",
    "dismissible under `allowed-warn-and-dialog`, blocking under",
    "`not-allowed-warn-and-dialog`, independent of `invalid_vote_policy`. The",
    "over-vote and min-vote families are silent-discount-prone precisely",
    "because their rules *can* be configured to not gate (over=`allowed`,",
    "min-vote has no policy) while `invalid=allowed` removes the generic",
    "gate; a preferential rule cannot be put in that state.",
    "",
    "| gap_policy | invalid_policy | state | errors | alerts | inline (shown) | hard gate | soft gate | tally | pred? |",
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
writeFileSync(path.join(here, "prefgaps-rule.md"), md)
console.log(`\nwrote prefgaps-rule.recorded.json and prefgaps-rule.md (${rows.length} rows)`)
