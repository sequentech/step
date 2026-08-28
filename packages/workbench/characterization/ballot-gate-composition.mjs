// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Ballot-gate composition — EVIDENCE that `BallotValidator`'s cross-contest
// gate OR matches production's whole-ballot gate.
//
// POST-INJECTION NOTE (gates injected into voting_screen.rs): production's
// per-contest gate now IS the query-provider, so this runner's independent
// leg is the ORACLE free functions' OR (emit-grid's `ballot` kind). Its
// combinations sit off the fix cells (fixed ≡ oracle there), so it still
// checks that production composes per-contest gates by OR — but it no
// longer checks the per-contest predicate itself against anything
// independent; the sweep's per-component expectations own that.
//
// The per-contest gate predicate is already validated exhaustively (the
// sweep, headless; dom-validate, in the booth). What neither reaches is the
// COMPOSITION: production's gates iterate every contest and fire if ANY
// blocks (`voting_screen.rs`; the "gate composition across contests" scope
// boundary in characterization/README.md). That whole-ballot gate is what
// the review-entry transition will query on `BallotValidator`, so it must be
// checked against production.
//
// It is a headless check, not a browser one, on purpose: the composition
// lives entirely in the wasm gate function (`check_voting_not_allowed_next` /
// `_error_dialog`, which iterate contests) — the per-contest predicate's UI
// manifestation is already dom-validated, and driving it in the booth would
// only add navigation that tests navigation, not the OR (and hits the
// Next-disabled-when-a-contest-gates wall). So we call the real gate wasm
// over a genuine multi-contest record, exactly as the booth's
// `disableNextButton` does, and compare to `BallotValidator`.
//
// The gate OR is contest-shape-agnostic (it folds a per-contest predicate
// that reads only that contest's own record — verified: the gates touch no
// ballot-level flag), so two Referendum-shaped contests in every combination
// of {no-gate, hard, soft} exercise the composition fully. The gates read
// only per-contest data, so nothing shared is lost by using one shape.
//
// Headless; needs the sequent-core wasm pkg and cargo. Writes
// ballot-gate-composition.md + .recorded.json; exits nonzero on any
// disagreement.
//
// Run:  node characterization/ballot-gate-composition.mjs   (from packages/workbench)

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {loadWasm, loadVelvetWasm, runChecker, runGates} from "./harness.mjs"
import {makeEml, makeSelection, contest} from "./cell.mjs"
import {specBallot} from "./rust-spec.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const BASE = {
    invalid: "allowed",
    blank: "allowed",
    over: "allowed",
    under: "allowed",
    dup: "allowed-warn-and-dialog",
    gap: "allowed-warn-and-dialog",
}
const vs = (regulars) => ({
    regulars,
    blankMarker: false,
    explicitInvalid: false,
    duplicateRanks: false,
    rankGaps: false,
    firstPreferences: regulars,
})

// Three per-contest gate outcomes on the Referendum contest:
//   none — one regular selected, permissive policies → no gate
//   hard — empty ballot, blank = not-allowed          → blocking
//   soft — empty ballot, blank = warn                 → dismissible
const STATES = {
    none: {config: {min: 0, max: 2, policies: BASE}, voteState: vs(1)},
    hard: {config: {min: 0, max: 2, policies: {...BASE, blank: "not-allowed"}}, voteState: vs(0)},
    soft: {config: {min: 0, max: 2, policies: {...BASE, blank: "warn"}}, voteState: vs(0)},
}

await loadWasm()
await loadVelvetWasm()

// Silence the wasm gate debug line (UPSTREAM_FINDINGS Defect 2).
const consoleLog = console.log
console.log = () => {}

/** Build a contest object + its decoded record for one state, at a chosen
 *  contest id (so a ballot can carry two of them). */
function buildContest(state, contestId) {
    const eml = makeEml(state.config, state.voteState)
    const c = structuredClone(eml.contests.find((x) => x.id === contest.id))
    const decoded = structuredClone(runChecker(makeSelection(state.voteState), eml))
    c.id = contestId
    decoded.contest_id = contestId
    return {contest: c, decoded}
}

const NAMES = ["none", "hard", "soft"]
const rows = []
const disagreements = []

// Sanity: confirm each per-contest state alone gives the intended gate — a
// wrong composition result is meaningless if the components are wrong.
for (const n of NAMES) {
    const {contest: c, decoded} = buildContest(STATES[n], `solo-${n}`)
    const g = runGates([c], {[`solo-${n}`]: decoded})
    const want = {none: [false, false], hard: [true, false], soft: [false, true]}[n]
    if (g.hard !== want[0] || g.soft !== want[1])
        disagreements.push({kind: "component", state: n, got: g, want})
}

// The composition: every (A, B) combination, whole-ballot gate vs BallotValidator.
for (const a of NAMES) {
    for (const b of NAMES) {
        const A = buildContest(STATES[a], "contest-a")
        const B = buildContest(STATES[b], "contest-b")
        const observed = runGates([A.contest, B.contest], {
            "contest-a": A.decoded,
            "contest-b": B.decoded,
        })
        const [expected] = specBallot([[STATES[a], STATES[b]]])
        const ok = observed.hard === expected.hard && observed.soft === expected.soft
        rows.push({a, b, observed, expected, ok})
        if (!ok) disagreements.push({kind: "composition", a, b, observed, expected})
    }
}

console.log = consoleLog

const ok = disagreements.length === 0
console.log(
    `ballot-gate composition: ${rows.length} contest pairs; ` +
        `${disagreements.length} disagreement(s) with BallotValidator`
)
for (const d of disagreements) console.log("  ✗ " + JSON.stringify(d))

const cell = (v) => (v ? "**block**" : "—")
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Ballot-gate composition",
    "",
    "Generated by `characterization/ballot-gate-composition.mjs`; do not edit by hand.",
    "",
    "**What this is.** Production's submission gates fire if ANY contest on",
    "the ballot blocks (`voting_screen.rs`). The per-contest predicate is",
    "validated elsewhere (the sweep, `dom-validate.md`); this validates the",
    "OR *composition* — the whole-ballot gate the review-entry transition",
    "queries on `BallotValidator` — against the real wasm gate functions, over",
    "every combination of two contests in {no-gate, hard, soft} states. It",
    "closes the *gate composition across contests* scope boundary.",
    "",
    "Columns: *A* / *B* are the two contests' per-contest gate outcomes;",
    "*hard* / *soft* are the whole-ballot gate observed from the real wasm,",
    "which must equal `BallotValidator`'s OR (the `ok` column).",
    "",
    "| A | B | hard | soft | matches BallotValidator |",
    "|---|---|---|---|---|",
    ...rows.map(
        (r) => `| ${r.a} | ${r.b} | ${cell(r.observed.hard)} | ${cell(r.observed.soft)} | ${r.ok ? "✓" : "**✗**"} |`
    ),
    "",
    `**Result: ${rows.length} contest pairs, ${disagreements.length} disagreement(s).**`,
    "",
].join("\n")

writeFileSync(path.join(here, "ballot-gate-composition.md"), md)
writeFileSync(
    path.join(here, "ballot-gate-composition.recorded.json"),
    JSON.stringify({pairs: rows.length, disagreements, rows}, null, 2) + "\n"
)
console.log("\nwrote ballot-gate-composition.md and ballot-gate-composition.recorded.json")
if (!ok) process.exitCode = 1
