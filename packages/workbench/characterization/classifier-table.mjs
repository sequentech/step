// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The tally classifier's OWN decision table.
//
// This is a different artifact from the `tally` column in the rule tables,
// and the difference is the point. The rule tables sample the classifier
// *incidentally*: each cell's decoded ballot comes out of the real decode,
// so the classifier is only ever probed at input combinations the rules
// happen to produce (the over-vote runner never makes a declined ballot;
// no rule runner makes decline+flag). Here the classifier is probed
// *deliberately and exhaustively* as what it is — a pure function of the
// decoded ballot — over the full cross-product of the inputs it reads:
//
//   is_decline_to_vote (flag) × is_explicit_invalid (flag) ×
//   invalid_errors non-empty × selection {none, regular, marker, mixed}
//   = 2 × 2 × 2 × 4 = 32 cells.
//
// Inputs are therefore HAND-SHAPED decoded ballots, deliberately NOT
// booth-produced: the subject is the classifier, not the pipeline. Some
// cells are unreachable through parts of the real pipeline (noted in the
// generated legend); they are characterized anyway — prevention-guarded
// or channel-limited states must still have defined behaviour (see the
// pruning caution in VALIDATION_LOGIC_DISTILLATION.md §2).
//
// The class is recorded through velvet-wasm's real tally (tallyClass): one
// ballot in, one counter increments, that counter is the class. This runs
// the actual production classifier, not a reimplementation.
//
// Run:  node characterization/classifier-table.mjs  (from packages/workbench)

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {loadWasm, loadVelvetWasm, tallyClass, loadMarkerFixture} from "./harness.mjs"
import {specClassify} from "./rust-spec.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const DECLINE = [false, true]
const EXPLICIT_INVALID = [false, true]
const HAS_ERRORS = [false, true]
// Selection over the Referendum contest (Yes / No / explicit-blank marker):
//   none    — nothing selected
//   regular — one regular candidate (Yes)
//   marker  — the explicit-blank marker alone
//   mixed   — marker + a regular candidate
const SELECTIONS = ["none", "regular", "marker", "mixed"]

await loadWasm()
await loadVelvetWasm()
const snap = loadMarkerFixture()
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const markerId = contest.candidates.find((x) => x.presentation?.is_explicit_blank).id
const regularId = contest.candidates.find((x) => !x.presentation?.is_explicit_blank).id

const SYNTHETIC_ERROR = {
    error_type: "Implicit",
    candidate_id: null,
    message: "synthetic.characterization",
    message_map: {},
}

function makeDecoded(decline, flag, errors, selection) {
    const picked =
        selection === "regular"
            ? [regularId]
            : selection === "marker"
              ? [markerId]
              : selection === "mixed"
                ? [markerId, regularId]
                : []
    return {
        contest_id: contest.id,
        is_explicit_invalid: flag,
        is_decline_to_vote: decline,
        invalid_errors: errors ? [SYNTHETIC_ERROR] : [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected: picked.includes(c.id) ? 0 : -1,
            write_in_text: null,
        })),
    }
}

// ---------------------------------------------------------------------------
// PREDICTION — the documented precedence (VOTE_VALIDATION.md,
// "Tally-Time Classification"), transcribed independently of the code:
//
//   1. decline:  blank → Declined ; anything else → ImplicitInvalid
//      (blank = !invalid && nothing selected; invalid = flag || errors)
//   2. invalid (flag || errors): flag → ExplicitInvalid ; else ImplicitInvalid
//   3. marker + regular selected → ImplicitInvalid (mix rule)
//   4. marker alone → ExplicitBlank
//   5. nothing selected → ImplicitBlank
//   6. otherwise → Valid
// ---------------------------------------------------------------------------
// This runner's prediction IS the shared classifier — the classifier table
// validates the Rust spec's `classify` directly against velvet-wasm's real
// tally. One batched call up front, then a lookup per row.
const predictions = new Map()
const pkey = (decline, flag, errors, selection) =>
    `${decline}|${flag}|${errors}|${selection}`
{
    const cells = []
    for (const decline of [false, true])
        for (const flag of [false, true])
            for (const errors of [false, true])
                for (const selection of ["none", "regular", "marker", "mixed"])
                    cells.push({decline, flag, hasErrors: errors, selection})
    const out = specClassify(cells)
    cells.forEach((c, i) =>
        predictions.set(pkey(c.decline, c.flag, c.hasErrors, c.selection), out[i])
    )
}
function predict(decline, flag, errors, selection) {
    return predictions.get(pkey(decline, flag, errors, selection))
}

// ---------------------------------------------------------------------------
const rows = []
for (const decline of DECLINE) {
    for (const flag of EXPLICIT_INVALID) {
        for (const errors of HAS_ERRORS) {
            for (const selection of SELECTIONS) {
                const decoded = makeDecoded(decline, flag, errors, selection)
                const observed = tallyClass(contest, decoded)
                const predicted = predict(decline, flag, errors, selection)
                rows.push({
                    is_decline_to_vote: decline,
                    is_explicit_invalid: flag,
                    has_errors: errors,
                    selection,
                    observed_class: observed,
                    predicted_class: predicted,
                    match: observed === predicted,
                })
            }
        }
    }
}

const mismatches = rows.filter((r) => !r.match)
console.log(`cells: ${rows.length}, prediction mismatches: ${mismatches.length}`)
for (const m of mismatches) {
    console.log(
        `MISMATCH decline=${m.is_decline_to_vote} flag=${m.is_explicit_invalid} ` +
            `errors=${m.has_errors} selection=${m.selection}: ` +
            `observed=${m.observed_class} predicted=${m.predicted_class}`
    )
}

// class → cells view (the "six-class decision table" made visible)
const byClass = {}
for (const r of rows) {
    ;(byClass[r.observed_class] ??= []).push(
        `decline=${+r.is_decline_to_vote} flag=${+r.is_explicit_invalid} ` +
            `errors=${+r.has_errors} sel=${r.selection}`
    )
}
console.log("\nrecorded class distribution:")
for (const [cls, cells] of Object.entries(byClass)) {
    console.log(`  ${cls}: ${cells.length} cells`)
}

writeFileSync(
    path.join(here, "classifier-table.recorded.json"),
    JSON.stringify({contest: contest.name, rows, byClass}, null, 2) + "\n"
)

const b = (v) => (v ? "**T**" : "f")
const fmt = (r) =>
    `| ${b(r.is_decline_to_vote)} | ${b(r.is_explicit_invalid)} | ${b(r.has_errors)} | ` +
    `${r.selection} | ${r.observed_class} | ${r.match ? "✓" : "**✗**"} |`
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Tally classifier — six-class decision table",
    "",
    "Generated by `characterization/classifier-table.mjs`; do not edit by hand.",
    "",
    "**Subject and contrast.** This table characterizes `classify_ballot`",
    "directly, as the pure function it is, over the **full cross-product of",
    "the inputs it reads** — unlike the `tally` column in the rule tables,",
    "which samples the same classifier only at the decoded ballots each",
    "rule's cells happen to produce. Inputs here are **hand-shaped decoded",
    "ballots, not booth-produced**: `has_errors` is a synthetic",
    "`invalid_errors` entry (in the pipeline it would come from the decode's",
    "checkers), and every flag combination is exercised whether or not any",
    "single pipeline can reach it.",
    "",
    "**Dimensions** (columns 1–4; `T`/`f` = true/false):",
    "",
    "- *decline* — the `is_decline_to_vote` flag. Only multi-contest ballots",
    "  can carry it in production (`raw_ballot` decode hardcodes false), so",
    "  decline rows are unreachable via single-contest flows — characterized",
    "  anyway: channel-limited states must still have defined behaviour.",
    "- *flag* — the `is_explicit_invalid` flag.",
    "- *errors* — whether `invalid_errors` is non-empty. Note `is_invalid()`",
    "  = *flag* OR *errors*; the two dimensions let the table separate the",
    "  Explicit/Implicit split from mere invalidity.",
    "- *selection* — none / regular (one normal candidate) / marker (the",
    "  explicit-blank marker alone) / mixed (marker + regular).",
    "",
    "*class* is **recorded**: the ballot ran through velvet-wasm's real",
    "tally and the counter that incremented is the class. `pred?` compares",
    "against the documented precedence (decline → invalid → mix → marker →",
    "empty → valid); ✗ = code and docs disagree.",
    "",
    "**Precedence semantics: first matching guard wins.** A ballot often",
    "satisfies several class descriptions at once (e.g. blank marker +",
    "a `selectedMin` error); its class is decided by the *earliest* guard",
    "it trips, and every later description it also satisfies is never",
    "consulted. Two guards split internally: *decline* is purity-guarded",
    "(decline + otherwise-empty → Declined, decline + any content →",
    "ImplicitInvalid) and *invalid* splits on the flag (flag →",
    "ExplicitInvalid even when errors are also present; errors alone →",
    "ImplicitInvalid). This ordering is why a marker-only ballot with an",
    "error can never reach ExplicitBlank (S2's transmutation) and why",
    "ImplicitInvalid dominates the table — it is the sink every",
    "contradiction drains into.",
    "",
    "| decline | flag | errors | selection | class (recorded) | pred? |",
    "|---|---|---|---|---|---|",
    ...rows.map(fmt),
    "",
    "## Recorded decision structure (class → cells)",
    "",
    "Generated from the rows above — the six classes and exactly which",
    "input combinations produce each:",
    "",
    ...Object.entries(byClass).flatMap(([cls, cells]) => [
        `**${cls}** (${cells.length}):`,
        "",
        ...cells.map((c) => `- \`${c}\``),
        "",
    ]),
].join("\n")
writeFileSync(path.join(here, "classifier-table.md"), md)
console.log(`\nwrote classifier-table.recorded.json and classifier-table.md (${rows.length} rows)`)
