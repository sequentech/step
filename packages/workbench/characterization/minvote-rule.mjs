// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the MIN-VOTE rule, layers 1+2 + recorded tally class.
//
// Min-vote is NOT a policy enum: check_min_vote_policy always pushes a
// `selectedMin` error to invalid_errors when the marker-inclusive count is
// below min_votes. So the input dimension is the min_votes *value*, not a
// policy. We vary min_votes ∈ {1, 2} × invalid_vote_policy × selection count
// on the Referendum contest (Yes / No, max forced to 3 so we can be under
// min without being blank), and — because a selected explicit-blank marker
// counts toward the min — one marker-only state to record that interplay.
//
// Hypothesis under test (VOTE_VALIDATION.md master filter): `selectedMin`
// is NOT in the filter's keep-list, so under invalid_vote_policy=allowed a
// min-violation is suppressed from display AND neither gate fires (both need
// invalid != allowed), yet is_invalid() is true → ImplicitInvalid. That is
// a SECOND silent-discount family, distinct from the over-vote one.
//
// Run:  node characterization/minvote-rule.mjs   (from packages/workbench)

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

const here = path.dirname(fileURLToPath(import.meta.url))

const MIN_VALUES = [1, 2]
const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// Selection counts (regular candidates), plus a marker-only state:
//   none        — 0 regular selections
//   one         — 1 regular selection
//   marker_only — the explicit-blank marker alone (counts toward min)
const STATES = ["none", "one", "marker_only"]

await loadWasm()
await loadVelvetWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const markerId = contest.candidates.find((x) => x.presentation?.is_explicit_blank).id
const regulars = contest.candidates
    .filter((x) => !x.presentation?.is_explicit_blank)
    .map((x) => x.id)

function makeSelection(state) {
    const picked =
        state === "one"
            ? [regulars[0]]
            : state === "marker_only"
              ? [markerId]
              : []
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

function makeEml(min, invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.min_votes = min
    c.max_votes = 3 // room to be under-min without hitting over/at-max
    c.presentation = {
        ...(c.presentation ?? {}),
        // blank/under at defaults; over irrelevant here
        invalid_vote_policy: invalidPolicy,
    }
    return clone
}

// marker-inclusive count (explicit-blank marker counts toward min_votes)
function markerInclusiveCount(state) {
    return state === "none" ? 0 : 1
}

// PREDICTION — documented rules.
function predict(min, invalid, state) {
    const errors = []
    const alerts = []
    const count = markerInclusiveCount(state)
    if (count < min) errors.push("errors.implicit.selectedMin")
    // blank checker: count==0 && blank policy != allowed → default allowed,
    // so no blank output. (state=none, count 0, but blank allowed.)

    const hard = errors.length > 0 && invalid === "not-allowed"
    const soft = errors.length > 0 && invalid !== "allowed"

    // classifier
    let tally
    if (errors.length > 0) tally = "ImplicitInvalid"
    else if (state === "marker_only") tally = "ExplicitBlank"
    else if (count === 0) tally = "ImplicitBlank"
    else tally = "Valid"
    return {errors, alerts, hard, soft, tally}
}

// selectedMin is NOT in the master filter keep-list, so under
// invalid=allowed it is suppressed; otherwise shown. Alerts always shown.
function derivedInlineVisible(observed, invalid) {
    const keptErrors = observed.errors.filter(() => invalid !== "allowed")
    return [...keptErrors, ...observed.alerts]
}

const rows = []
for (const min of MIN_VALUES) {
    for (const invalid of INVALID_POLICIES) {
        const cellEml = makeEml(min, invalid)
        const cellContest = cellEml.contests.find((x) => x.id === contest.id)
        for (const state of STATES) {
            const decoded = runChecker(makeSelection(state), cellEml)
            const {errors, alerts} = extractErrors(decoded)
            const gates = runGates([cellContest], {[contest.id]: decoded})
            const tally = tallyClass(cellContest, decoded)
            const observed = {errors, alerts, ...gates, tally}
            const p = predict(min, invalid, state)
            const match =
                JSON.stringify([errors, alerts, gates.hard, gates.soft, tally]) ===
                JSON.stringify([p.errors, p.alerts, p.hard, p.soft, p.tally])
            rows.push({
                min_votes: min,
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
    console.log(`\nMISMATCH min=${m.min_votes} invalid=${m.invalid_vote_policy} state=${m.state}`)
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}
const flagged = rows.filter(isSilentDiscount)
console.log(`silent-discount cells: ${flagged.length}`)
for (const f of flagged) {
    console.log(`  ⚠ min=${f.min_votes} invalid=${f.invalid_vote_policy} state=${f.state} → ${f.observed.tally}`)
}

writeFileSync(
    path.join(here, "minvote-rule.recorded.json"),
    JSON.stringify({contest: contest.name, rows}, null, 2) + "\n"
)

const short = (xs) =>
    xs.length === 0 ? "—" : xs.map((m) => m.replace(/^errors\.\w+\./, "")).join("<br>")
const fmt = (r) =>
    `| ${isSilentDiscount(r) ? "**⚠** " : ""}${r.min_votes} | ${r.invalid_vote_policy} | ${r.state} | ` +
    `${short(r.observed.errors)} | ${short(r.observed.alerts)} | ` +
    `${r.observed.hard ? "**block**" : "—"} | ${r.observed.soft ? "dialog" : "—"} | ` +
    `${r.observed.tally} | ${r.match ? "✓" : "**✗**"} |`
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Min-vote rule characterization — layers 1+2 + recorded tally class",
    "",
    "Generated by `characterization/minvote-rule.mjs`; do not edit by hand.",
    "",
    "**Experiment:** min-vote is not a policy — it is the fixed rule",
    "`count < min_votes → selectedMin error`. Rows vary `min_votes` and the",
    "invalid policy on the *Referendum* contest (`max_votes` forced to 3).",
    "States: *none* = 0 selections; *one* = 1 regular candidate;",
    "*marker_only* = the explicit-blank marker alone (which **counts toward**",
    "min_votes — the marker-inclusive count).",
    "",
    "Columns as in the other rule tables. A row prefixed **⚠** is a derived",
    "silent-discount marker. **This rule is expected to produce them**: the",
    "`selectedMin` error is not in the booth filter's keep-list, so under",
    "`invalid_vote_policy = allowed` a min-violation is suppressed and neither",
    "gate fires, yet the tally classifies the ballot `ImplicitInvalid`.",
    "",
    "| min_votes | invalid_policy | state | errors | alerts | hard gate | soft gate | tally | pred? |",
    "|---|---|---|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
].join("\n")
writeFileSync(path.join(here, "minvote-rule.md"), md)
console.log(`\nwrote minvote-rule.recorded.json and minvote-rule.md (${rows.length} rows)`)
