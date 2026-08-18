// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Exhaustive headless production sweep — stage 1 of the dependency-driven
// validation pipeline (VALIDATION_LOGIC_DISTILLATION.md §2, the effect-first
// decomposition; plan settled with the operator 2026-08-17).
//
// Runs PRODUCTION (the real WASM checker → gates → tally, exactly what the
// rule runners record) against `spec.f` on EVERY cell of the representable
// headless subdomain:
//
//   all 6 policies × all values   (incl. dup/gap — their claimed inertness
//                                  on plurality states gets production-
//                                  confirmed rather than assumed)
//   bounds                        min ∈ 0..3, max ∈ 1..3, min ≤ max
//                                  (max = 0 stays out: the config-sanity
//                                  scope boundary)
//   plurality vote states         regulars 0..2 × blank marker × invalid
//                                  flag, on the Referendum fixture
//   preferential vote states      every reachable (regulars, duplicate
//                                  ranks, rank gaps) triple × invalid flag,
//                                  on the IRV fixture (no marker candidate,
//                                  so no blank marker there)
//                                  No decline: the single-contest decode
//                                  hardcodes it false
//
// After a clean run, every headless effect claim the spec makes on this
// subdomain — dependence AND independence — is production-verified by
// coverage, not by argument: a transcription hole of either polarity
// (commission or omission) would surface as a disagreement here. Outside
// the subdomain the labels of effect-dependencies.md still apply.
//
// The sweep also emits stage 3's input: the QUOTIENT INVENTORY — every
// reachable (emissions, consulted-policies) class with one representative
// cell. The booth's message filter reads the inputs only through the
// checker record, the four policies it consults (invalid, blank, over,
// under — not dup/gap), and the observation point (sufficiency / mediated
// CI; the production-side license is the filter's props boundary,
// re-verified at the browser stage). One booth run per class therefore
// covers every inline claim on this subdomain — the reduced-scope browser
// test of the pipeline.
//
// Headless; needs the sequent-core wasm pkg. Writes headless-sweep.md +
// .recorded.json; exits nonzero on any disagreement. Takes ~1 min;
// progress is printed per policy block.
//
// Run:  node characterization/headless-sweep.mjs   (from packages/workbench)

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {performance} from "node:perf_hooks"
import {loadWasm, loadVelvetWasm} from "./harness.mjs"
import {representable, observeHeadless, shortKey, rankedTriples} from "./cell.mjs"
import {f as specF} from "./spec.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const INVALID = ["allowed", "warn", "warn-invalid-implicit-and-explicit", "not-allowed"]
const BLANK = ["allowed", "warn", "warn-only-in-review", "not-allowed"]
const OVER = [
    "allowed",
    "allowed-with-msg",
    "allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-disable",
]
const UNDER = ["allowed", "warn", "warn-only-in-review", "warn-and-alert"]
const RANKP = ["allowed-warn-and-dialog", "not-allowed-warn-and-dialog"]
const BOUNDS = []
for (let min = 0; min <= 3; min++)
    for (let max = 1; max <= 3; max++) if (min <= max) BOUNDS.push([min, max])
const STATES = []
// plurality states — the Referendum fixture's two regulars, the blank
// marker, the explicit-invalid flag
for (let regulars = 0; regulars <= 2; regulars++)
    for (const blankMarker of [false, true])
        for (const explicitInvalid of [false, true])
            STATES.push({
                regulars,
                blankMarker,
                explicitInvalid,
                duplicateRanks: false,
                rankGaps: false,
                // every plurality selection sits at rank 0
                firstPreferences: regulars,
            })
// preferential states — the IRV fixture. Its contest carries no marker
// candidate, so blankMarker is false throughout; the reachable
// (regulars, dup, gap) triples come from `cell.mjs`, which derives them by
// the same rule production uses (`plaintext.rs::validate_preferencial_order`)
for (const triple of rankedTriples())
    for (const explicitInvalid of [false, true])
        STATES.push({...triple, blankMarker: false, explicitInvalid})

await loadWasm()
await loadVelvetWasm()

// The wasm gates print a debug line per call (the stray `max={min:?}` log —
// UPSTREAM_FINDINGS.md Defect 2); at sweep scale that is hundreds of
// thousands of lines, so silence console.log for the duration and report
// progress via stdout directly.
const consoleLog = console.log
console.log = () => {}

const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b)
const sortedUniq = (xs) => [...new Set(xs)].sort()

const disagreements = []
const classes = new Map() // quotient: (emissions, consulted policies) → representative
let cells = 0
const t0 = performance.now()

for (const invalid of INVALID) {
    for (const blank of BLANK) {
        for (const over of OVER) {
            for (const under of UNDER) {
                for (const dup of RANKP) {
                    for (const gap of RANKP) {
                        const policies = {invalid, blank, over, under, dup, gap}
                        for (const [min, max] of BOUNDS) {
                            for (const state of STATES) {
                                const cell = {
                                    config: {min, max, policies},
                                    voteState: {...state},
                                }
                                if (representable(cell)) continue
                                cells++
                                const prod = observeHeadless(cell)
                                const spec = specF(cell.config, cell.voteState)
                                const specDialog = spec.gate.hard
                                    ? "blocking"
                                    : spec.gate.soft
                                      ? "dismissible"
                                      : "none"
                                const prodDialog = prod.hard
                                    ? "blocking"
                                    : prod.soft
                                      ? "dismissible"
                                      : "none"
                                const bad =
                                    !eq(sortedUniq(prod.errors), sortedUniq(spec.emissions.errors.map(shortKey))) ||
                                    !eq(sortedUniq(prod.alerts), sortedUniq(spec.emissions.alerts.map(shortKey))) ||
                                    prod.hard !== spec.gate.hard ||
                                    prod.soft !== spec.gate.soft ||
                                    prodDialog !== specDialog ||
                                    prod.tally !== spec.tally
                                if (bad) {
                                    disagreements.push({cell, production: prod, spec: {
                                        errors: spec.emissions.errors.map(shortKey),
                                        alerts: spec.emissions.alerts.map(shortKey),
                                        hard: spec.gate.hard,
                                        soft: spec.gate.soft,
                                        tally: spec.tally,
                                    }})
                                }
                                // Quotient class: keyed by the PRODUCTION-observed
                                // checker record + the policies the filter consults.
                                // The spec's inline views are recorded per class; a
                                // conflict would break the spec-side factorization
                                // (structurally impossible while renderedKeys reads
                                // only these inputs — kept as a cheap safety net).
                                const key = JSON.stringify([
                                    sortedUniq(prod.errors),
                                    sortedUniq(prod.alerts),
                                    invalid,
                                    blank,
                                    over,
                                    under,
                                ])
                                const specInline = {
                                    voting: sortedUniq(spec.inline.voting.map(shortKey)),
                                    review: sortedUniq(spec.inline.review.map(shortKey)),
                                }
                                const existing = classes.get(key)
                                if (!existing) {
                                    classes.set(key, {representative: cell, spec_inline: specInline, cells: 1})
                                } else {
                                    existing.cells++
                                    if (!eq(existing.spec_inline, specInline)) {
                                        disagreements.push({
                                            factorization_conflict: key,
                                            cell,
                                            spec_inline: specInline,
                                            class_inline: existing.spec_inline,
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        process.stdout.write(
            `  swept invalid=${invalid} blank=${blank} — ${cells} cells, ` +
                `${disagreements.length} disagreements, ${classes.size} classes, ` +
                `${Math.round((performance.now() - t0) / 1000)}s\n`
        )
    }
}
console.log = consoleLog

const totalS = Math.round((performance.now() - t0) / 1000)
console.log(
    `\n${cells} cells: production ≡ spec on every headless effect: ${disagreements.length === 0}` +
        ` (${disagreements.length} disagreements); ${classes.size} quotient classes; ${totalS}s`
)

const classList = [...classes.entries()].map(([key, v]) => ({
    key: JSON.parse(key),
    representative: v.representative,
    spec_inline: v.spec_inline,
    cells: v.cells,
}))

writeFileSync(
    path.join(here, "headless-sweep.recorded.json"),
    JSON.stringify(
        {
            domain: {
                policies: {INVALID, BLANK, OVER, UNDER, dup: RANKP, gap: RANKP},
                bounds: "min 0..3 × max 1..3, min ≤ max",
                states: "regulars 0..2 × blank marker × explicit-invalid flag; no decline, no preferential state",
                cells,
            },
            disagreements,
            quotient: {
                consulted_policies: ["invalid", "blank", "over", "under"],
                classes: classList,
            },
        },
        null,
        2
    ) + "\n"
)

const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Exhaustive headless sweep — production ≡ spec on the representable subdomain",
    "",
    "Generated by `characterization/headless-sweep.mjs`; do not edit by hand.",
    "",
    "**Experiment.** Every cell — one concrete (contest-configuration ×",
    "vote-state) combination — of the representable headless subdomain is",
    "driven through the REAL WASM (checker → gates → tally, the same entry",
    "points the rule runners record) and compared against `spec.f` on every",
    "headless effect: checker errors and alerts (as key sets), both gates,",
    "the dialog projection, and the tally class. The subdomain: all values",
    "of all six policies (dup/gap included — their inertness on plurality",
    "states is thereby production-confirmed, not assumed), bounds min 0..3 ×",
    "max 1..3 with min ≤ max (max = 0 stays out — the config-sanity scope",
    "boundary), and the plurality vote states (regulars 0–2 × blank marker ×",
    "explicit-invalid flag; no decline, no preferential state).",
    "",
    "**Why exhaustive.** The per-rule grids validate slices; the",
    "effect-dependency analysis (`effect-dependencies.md`) enumerates what",
    "the spec claims between the slices, but claims of *independence* are",
    "universal and cannot be discharged by witnesses. Sweeping the whole",
    "subdomain discharges them by coverage: after a clean run, a",
    "transcription hole of either polarity — a wrong clause or a missing",
    "one — cannot exist on this subdomain for the headless effects, because",
    "every cell was compared. Outside the subdomain (preferential states,",
    "decline, max = 0, browser-only effects) the labels of",
    "`effect-dependencies.md` continue to apply.",
    "",
    `**Result: ${cells.toLocaleString("en-US")} cells, ${disagreements.length} disagreement(s).**`,
    "",
    "## Quotient inventory (the browser stage's input)",
    "",
    "The booth's message filter reads the inputs only through the checker",
    "record, the four policies it consults (`invalid`, `blank`, `over`,",
    "`under` — never dup/gap), and the observation point — *sufficiency*",
    "(conditional independence given a computed mediator). Its",
    "production-side license is the filter's props boundary, re-verified by",
    "source read at the browser stage. Under it, the inline behaviour of",
    "every cell of this subdomain is covered by one booth run per",
    "**reachable class** of (emissions × consulted policies):",
    "",
    `**${classes.size} reachable classes** (each with a representative cell and`,
    "the spec's predicted inline views, in `headless-sweep.recorded.json`) —",
    "versus " + cells.toLocaleString("en-US") + " cells: the browser cost collapse the",
    "quotient buys. Driving each representative through the real booth and",
    "comparing inline at both observation points is stage 3 of the pipeline;",
    "the spec-side factorization (inline constant within each class) was",
    "asserted during this sweep as a safety net.",
    "",
].join("\n")
writeFileSync(path.join(here, "headless-sweep.md"), md)
console.log("wrote headless-sweep.md and headless-sweep.recorded.json")
if (disagreements.length) process.exitCode = 1
