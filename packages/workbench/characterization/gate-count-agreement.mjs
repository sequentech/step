// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The gate/checker count-agreement property — ANALYSIS over the certified
// spec (EVIDENCE_RESTRUCTURE.md step 7).
//
//   count-agreement :=
//     ∀ (config, vote_state) reachable through the booth,
//       the count the GATES decide from
//         = the count the CHECKER decides from
//
// Both counts are fields of the spec, so the property is decided by
// evaluating it. Nothing here observes production.
//
// WHY A COUNTERFACTUAL, not a list of violations. Knowing the counts differ
// says nothing about whether a voter would notice. So for every violating
// cell this evaluates the mapping TWICE — once as production behaves, once
// with the gates given the checker's count — and reports the difference in
// what the voter meets. That difference IS the consequence; it is derived,
// not asserted alongside.
//
// The mechanism behind the violations is quirk
// S6_GATES_COUNT_FIRST_PREFERENCES_ONLY (`../validation-spec/src/lib.rs`):
// the gates count `choice.selected == 0` where the checker counts
// `choice.selected > -1`. On a plurality ballot those coincide; on a ranked
// ballot the gates are counting first preferences. This runner does not
// assume that — it asks the spec where the counts differ and what follows.
//
// Headless; needs cargo only. Writes gate-count-agreement.md + .report.json;
// exits nonzero if the property holds NOWHERE it used to fail (i.e. if a
// known consequence silently disappears).
//
// Run:  node characterization/gate-count-agreement.mjs   (from packages/workbench)

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {specF} from "./rust-spec.mjs"
import {representable, rankedTriples} from "./cell.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

// The domain headless-sweep certifies, enumerated the same way.
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
for (let regulars = 0; regulars <= 2; regulars++)
    for (const blankMarker of [false, true])
        for (const explicitInvalid of [false, true])
            STATES.push({
                regulars,
                blankMarker,
                explicitInvalid,
                duplicateRanks: false,
                rankGaps: false,
                firstPreferences: regulars,
            })
for (const triple of rankedTriples())
    for (const explicitInvalid of [false, true])
        STATES.push({...triple, blankMarker: false, explicitInvalid})

/** Do the two counts differ for this vote state? The checker counts every
 *  ranked selection (`regulars`); the gates count first preferences. */
const countsDiffer = (vs) =>
    vs.firstPreferences !== undefined && vs.firstPreferences !== vs.regulars

const cells = []
for (const invalid of INVALID)
    for (const blank of BLANK)
        for (const over of OVER)
            for (const under of UNDER)
                for (const dup of RANKP)
                    for (const gap of RANKP) {
                        const policies = {invalid, blank, over, under, dup, gap}
                        for (const [min, max] of BOUNDS)
                            for (const state of STATES) {
                                const cell = {
                                    config: {min, max, policies},
                                    voteState: {...state},
                                }
                                if (!representable(cell)) cells.push(cell)
                            }
                    }

const violating = cells.filter((c) => countsDiffer(c.voteState))
console.log(
    `${cells.length} certified cells; the two counts disagree on ${violating.length}`
)

// Evaluate each violating cell twice: as production behaves, and with the
// gates handed the checker's count.
const actual = specF(violating)
const repaired = specF(
    violating.map((c) => ({
        config: c.config,
        voteState: {...c.voteState, firstPreferences: c.voteState.regulars},
    }))
)

const consequences = new Map()
const examples = new Map()
let noConsequence = 0
for (let i = 0; i < violating.length; i++) {
    const a = actual[i]
    const r = repaired[i]
    if (a.dialog === r.dialog) {
        noConsequence++
        continue
    }
    // What the voter meets, versus what they would meet if the counts agreed.
    const silentInline = a.inline.voting.length === 0 && a.inline.review.length === 0
    const kind =
        r.dialog === "none"
            ? silentInline
                ? "dialog with NOTHING rendered inline (should be no dialog)"
                : "spurious dialog (should be none)"
            : a.dialog === "none"
              ? "MISSING dialog the policy promises"
              : `dialog kind changed: ${r.dialog} → ${a.dialog}`
    consequences.set(kind, (consequences.get(kind) ?? 0) + 1)
    if (!examples.has(kind)) examples.set(kind, {cell: violating[i], actual: a, repaired: r})
}

const affected = violating.length - noConsequence
console.log(
    `\nof those, ${affected} change what the voter meets; ${noConsequence} are absorbed ` +
        `(another clause fires either way)\n`
)
for (const [kind, n] of [...consequences.entries()].sort((a, b) => b[1] - a[1]))
    console.log(`  ${String(n).padStart(5)}  ${kind}`)

// THE DOMAIN'S OWN LIMIT, derived rather than argued. Every ranked state the
// sweep can drive carries a duplicate or a gap — `cell.mjs` routes a cell to
// the IRV fixture only when one of those is set — and BOTH variants of the
// dup and gap policies raise a dialog. So on this domain a dialog fires
// whatever the count is, and only its KIND can change. The cells where a
// dialog could appear from nothing, or vanish, are the CLEAN rankings, which
// no bundled fixture can drive (`representable()` rejects regulars > 2: the
// plurality contest has two candidates and the routing never sends a
// gap-free, duplicate-free cell to the IRV one).
//
// That is why the spurious-dialog-on-a-complete-ballot case has to be shown
// by direct probe and booth run: it is real, but it lies OUTSIDE everything
// this suite certifies. The property must not be read as covering it.
const PREF_ERRORS = ["duplicatedPosition", "preferenceOrderWithGaps"]
const withoutPrefError = violating.filter(
    (_, i) =>
        !actual[i].emissions.errors.some((m) => PREF_ERRORS.includes(m.split(".").pop()))
).length

// ACCEPTANCE — the consequences this domain can express must keep falling out
// of the property. If one stops appearing the derivation has lost its subject
// and the run fails.
const KNOWN = [
    "dialog kind changed: dismissible → blocking",
    "dialog kind changed: blocking → dismissible",
]
const missing = KNOWN.filter((k) => !consequences.has(k))
for (const m of missing) console.log(`\n! KNOWN consequence no longer derived: ${m}`)
console.log(
    `\ndomain limit: ${withoutPrefError} of the ${violating.length} violating cells carry ` +
        `neither a duplicate nor a gap error.`
)
console.log(
    "  Every ranked cell this suite can drive has one, and both dup/gap policy variants\n" +
        "  raise a dialog — so a dialog fires whatever the count is, and only its KIND can\n" +
        "  change here. A dialog appearing from nothing needs a CLEAN ranking, which no\n" +
        "  bundled fixture can drive. That case is real but lies outside this domain."
)

const fmtCell = (c) =>
    `min=${c.config.min} max=${c.config.max} ` +
    `regulars=${c.voteState.regulars} firstPreferences=${c.voteState.firstPreferences} ` +
    `dup=${c.voteState.duplicateRanks} gap=${c.voteState.rankGaps} | ` +
    Object.entries(c.config.policies)
        .map(([k, v]) => `${k}=${v}`)
        .join(" ")

const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Gate/checker count agreement",
    "",
    "Generated by `characterization/gate-count-agreement.mjs`; do not edit by hand.",
    "",
    "**The property.** The count the submission gates decide from should equal",
    "the count the checker decides from. They are two readings of one ballot;",
    "if they disagree, the gates are responding to a ballot the voter did not",
    "cast.",
    "",
    "**How it is decided.** Both counts are fields of the specification, so",
    `this is an ANALYSIS over the ${cells.length} cells \`headless-sweep.md\``,
    "certifies. Nothing here observes production.",
    "",
    "**What the consequence column means.** A disagreement in counting is not",
    "yet a harm. Each violating cell is therefore evaluated twice — as",
    "production behaves, and with the gates handed the checker's count — and",
    "the difference is what a voter would actually meet. Cells where another",
    "clause fires either way are reported as absorbed, not as harms.",
    "",
    `**Result: the counts disagree on ${violating.length} cells; ${affected} of those`,
    `change what the voter meets, ${noConsequence} are absorbed.**`,
    "",
    "| consequence | cells |",
    "|---|---|",
    ...[...consequences.entries()]
        .sort((a, b) => b[1] - a[1])
        .map(([k, n]) => `| ${k} | ${n} |`),
    "",
    "## One example of each",
    "",
    ...[...examples.entries()].flatMap(([kind, ex]) => [
        `**${kind}**`,
        "",
        "```",
        fmtCell(ex.cell),
        `  as production behaves : dialog=${ex.actual.dialog} ` +
            `inline@voting=${JSON.stringify(ex.actual.inline.voting)} ` +
            `inline@review=${JSON.stringify(ex.actual.inline.review)}`,
        `  if the counts agreed  : dialog=${ex.repaired.dialog} ` +
            `inline@voting=${JSON.stringify(ex.repaired.inline.voting)}`,
        "```",
        "",
    ]),
    "## What this domain cannot show",
    "",
    `Of the ${violating.length} violating cells, **${withoutPrefError}** carry neither a`,
    "duplicate nor a gap error. That is not a coincidence: `cell.mjs` routes a",
    "cell to the IRV fixture only when one of those is set, and both variants of",
    "the dup and gap policies raise a dialog. So on this domain a dialog fires",
    "whatever the count is, and only its **kind** can change.",
    "",
    "The sharper consequence — a dialog appearing on a ballot the checker is",
    "entirely happy with, with nothing rendered to explain it — needs a CLEAN",
    "ranking (every candidate ranked, no duplicate, no gap). No bundled fixture",
    "can drive one: the plurality contest carries two candidates, and the",
    "routing never sends a gap-free, duplicate-free cell to the IRV contest. It",
    "is demonstrated by direct probe and confirmed in the booth, but it lies",
    "**outside** the domain this property covers, and this file should not be",
    "read as evidence for it.",
    "",
    "The mechanism is quirk `S6_GATES_COUNT_FIRST_PREFERENCES_ONLY`: the gates",
    "count `choice.selected == 0` (`voting_screen.rs`) where the checker counts",
    "`choice.selected > -1` (`raw_ballot.rs`). On a plurality ballot every",
    "selection sits at rank 0 and the two agree, which is why this is invisible",
    "there; on a ranked ballot the gates are counting first preferences.",
    "",
].join("\n")

writeFileSync(path.join(here, "gate-count-agreement.md"), md)
writeFileSync(
    path.join(here, "gate-count-agreement.report.json"),
    JSON.stringify(
        {
            cells_evaluated: cells.length,
            counts_disagree: violating.length,
            voter_visible: affected,
            absorbed: noConsequence,
            consequences: [...consequences.entries()].map(([kind, cells]) => ({kind, cells})),
            examples: [...examples.entries()].map(([kind, ex]) => ({
                kind,
                cell: ex.cell,
                actual: {dialog: ex.actual.dialog, inline: ex.actual.inline},
                if_counts_agreed: {dialog: ex.repaired.dialog, inline: ex.repaired.inline},
            })),
            known_still_derived: missing.length === 0,
            domain_limit: {
                violating_cells_without_a_preferential_error: withoutPrefError,
                note:
                    "every ranked cell this suite can drive carries a duplicate or a gap, " +
                    "and both policy variants raise a dialog; so only the dialog KIND can " +
                    "change here. A dialog appearing from nothing needs a clean ranking, " +
                    "which no bundled fixture can drive.",
            },
        },
        null,
        2
    ) + "\n"
)
console.log("\nwrote gate-count-agreement.md and gate-count-agreement.report.json")
if (missing.length) process.exitCode = 1
