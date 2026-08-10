// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Characterization of the BLANK-VOTE rule, layers 1+2 (checker + gates).
//
// Enumerates blank_vote_policy × invalid_vote_policy × vote-state over the
// Referendum contest of the `explicit-blank-invalid` fixture (Yes / No /
// explicit-blank marker, min_votes=0, max_votes=2; over/under policies left
// at their defaults so the blank rule is isolated). For every cell it
// records what checker.rs emits and what both gates answer, and compares
// each observation against a PREDICTION derived from the documented rules
// in docs/VOTE_VALIDATION.md. A mismatch means one of the two is wrong —
// which is the entire point of recording (distillation §5.3 step 2).
//
// Run:  node characterization/blank-rule.mjs   (from packages/workbench)
// Output: blank-rule.recorded.json + blank-rule.md next to this script.

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {
    loadWasm,
    runChecker,
    runGates,
    loadMarkerFixture,
    extractErrors,
} from "./harness.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const BLANK_POLICIES = ["allowed", "warn", "warn-only-in-review", "not-allowed"]
const INVALID_POLICIES = [
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
]
// Vote states, chosen to bracket the blank condition:
//   empty            — the blank condition proper (marker-inclusive count 0)
//   explicit_invalid — flag set, nothing selected: blank checker must skip
//   marker_only      — explicit-blank marker selected: count 1, NOT blank
//   one_regular      — control: a normal selection
const STATES = ["empty", "explicit_invalid", "marker_only", "one_regular"]

await loadWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const markerId = contest.candidates.find(
    (x) => x.presentation?.is_explicit_blank
).id
const regularId = contest.candidates.find(
    (x) => !x.presentation?.is_explicit_blank
).id

function makeSelection(state) {
    return {
        contest_id: contest.id,
        is_explicit_invalid: state === "explicit_invalid",
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected:
                (state === "marker_only" && c.id === markerId) ||
                (state === "one_regular" && c.id === regularId)
                    ? 0
                    : -1,
            write_in_text: null,
        })),
    }
}

function makeEml(blankPolicy, invalidPolicy) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
    c.presentation = {
        ...(c.presentation ?? {}),
        blank_vote_policy: blankPolicy,
        invalid_vote_policy: invalidPolicy,
    }
    return clone
}

// ---------------------------------------------------------------------------
// PREDICTION — a direct transcription of the documented rules
// (VOTE_VALIDATION.md: blank/invalid checker tables + both gate tables).
// This is deliberately independent of the implementation: it is the first
// draft of the declarative mapping, and disagreements with the recording
// are findings, not bugs in this file.
// ---------------------------------------------------------------------------
function predict(blank, invalid, state) {
    const errors = []
    const alerts = []
    // marker-inclusive count
    const count =
        state === "empty" ? 0 : 1 // explicit_invalid / marker_only / one_regular all count 1

    // blank checker: count == 0 && !is_explicit_invalid && policy != allowed
    if (count === 0 && state !== "explicit_invalid" && blank !== "allowed") {
        ;(blank === "not-allowed" ? errors : alerts).push(
            "errors.implicit.blankVote"
        )
    }
    // invalid checker: only fires on the explicit flag
    if (state === "explicit_invalid") {
        if (invalid === "not-allowed") errors.push("errors.explicit.notAllowed")
        if (invalid === "warn-invalid-implicit-and-explicit")
            alerts.push("errors.explicit.alert")
    }

    // hard gate
    const hasExplicitTypeError =
        state === "explicit_invalid" && invalid === "not-allowed"
    const hard =
        hasExplicitTypeError ||
        (errors.length > 0 && invalid === "not-allowed") ||
        (count === 0 && blank === "not-allowed")

    // soft gate
    const soft =
        (errors.length > 0 && invalid !== "allowed") ||
        (invalid === "warn-invalid-implicit-and-explicit" &&
            state === "explicit_invalid") ||
        (blank === "warn" && count === 0)

    return {errors, alerts, hard, soft}
}

// ---------------------------------------------------------------------------
const rows = []
for (const blank of BLANK_POLICIES) {
    for (const invalid of INVALID_POLICIES) {
        const cellEml = makeEml(blank, invalid)
        for (const state of STATES) {
            const decoded = runChecker(makeSelection(state), cellEml)
            const {errors, alerts} = extractErrors(decoded)
            const gates = runGates(
                cellEml.contests.filter((c) => c.id === contest.id),
                {[contest.id]: decoded}
            )
            const p = predict(blank, invalid, state)
            const match =
                JSON.stringify([errors, alerts, gates.hard, gates.soft]) ===
                JSON.stringify([p.errors, p.alerts, p.hard, p.soft])
            rows.push({
                blank_vote_policy: blank,
                invalid_vote_policy: invalid,
                state,
                observed: {errors, alerts, ...gates},
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
        `\nMISMATCH  blank=${m.blank_vote_policy} invalid=${m.invalid_vote_policy} state=${m.state}`
    )
    console.log("  observed :", JSON.stringify(m.observed))
    console.log("  predicted:", JSON.stringify(m.predicted))
}

writeFileSync(
    path.join(here, "blank-rule.recorded.json"),
    JSON.stringify({generated: "see git history", contest: contest.name, rows}, null, 2) + "\n"
)

// human-readable table
const fmt = (r) =>
    `| ${r.blank_vote_policy} | ${r.invalid_vote_policy} | ${r.state} | ` +
    `${r.observed.errors.join("<br>") || "—"} | ${r.observed.alerts.join("<br>") || "—"} | ` +
    `${r.observed.hard ? "**block**" : "—"} | ${r.observed.soft ? "dialog" : "—"} | ` +
    `${r.match ? "✓" : "**✗**"} |`
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Blank-rule characterization — layers 1+2 (checker + gates)",
    "",
    "Generated by `characterization/blank-rule.mjs`; do not edit by hand.",
    "Cells run through the real wasm codec (`test_contest_reencoding_js`) and",
    "both gates. `pred?` compares against the rules as documented in",
    "`docs/VOTE_VALIDATION.md` — a ✗ is a disagreement between code and doc.",
    "",
    "Contest: Referendum (Yes / No / explicit-blank marker), min=0, max=2,",
    "over/under policies at defaults. Layer 3 (the TypeScript filter) is",
    "characterized separately in the browser.",
    "",
    "| blank_policy | invalid_policy | state | errors | alerts | hard gate | soft gate | pred? |",
    "|---|---|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
].join("\n")
writeFileSync(path.join(here, "blank-rule.md"), md)
console.log(`\nwrote blank-rule.recorded.json and blank-rule.md (${rows.length} rows)`)
