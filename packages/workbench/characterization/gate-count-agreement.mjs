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
import {certifiedCells} from "./domain.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const cells = certifiedCells()

/** Do the two counts differ for this vote state? The checker counts every
 *  ranked selection (`regulars`); the gates count first preferences. */
const countsDiffer = (vs) =>
    vs.firstPreferences !== undefined && vs.firstPreferences !== vs.regulars

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

// WHAT SHAPE OF BALLOT PRODUCES WHICH CONSEQUENCE, derived rather than
// argued. A malformed ranking always carries a duplicate or a gap error, and
// both variants of those policies raise a dialog — so on those cells a dialog
// fires whatever the count is, and only its KIND can change. It is the
// WELL-FORMED rankings, carrying no such error, where a dialog can appear
// from nothing or vanish entirely. Those were unreachable until the certified
// domain learned to route them (`cell.mjs`, `domain.mjs`); before that this
// property could only ever derive the kind-changes, and the sharper
// consequences had to be shown by ad-hoc probe.
const PREF_ERRORS = ["duplicatedPosition", "preferenceOrderWithGaps"]
const wellFormed = violating.filter(
    (_, i) =>
        !actual[i].emissions.errors.some((m) => PREF_ERRORS.includes(m.split(".").pop()))
).length

// ACCEPTANCE — every consequence shape must keep falling out of the property.
// The last two were impossible to derive before the domain was extended; if
// they stop appearing, the domain has silently narrowed again.
const KNOWN = [
    "dialog kind changed: dismissible → blocking",
    "dialog kind changed: blocking → dismissible",
    "dialog with NOTHING rendered inline (should be no dialog)",
    "MISSING dialog the policy promises",
]
const missing = KNOWN.filter((k) => !consequences.has(k))
for (const m of missing) console.log(`\n! KNOWN consequence no longer derived: ${m}`)
console.log(
    `\nby ballot shape: ${violating.length - wellFormed} violating cells are malformed ` +
        `rankings (a duplicate or a gap), ${wellFormed} are well-formed.`
)
console.log(
    "  Malformed rankings always carry an error whose policy raises a dialog either\n" +
        "  way, so only the dialog KIND can change there. The well-formed ones are where\n" +
        "  a dialog appears from nothing or goes missing — the ordinary ranked ballot."
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
    "## Which ballots produce which consequence",
    "",
    `Of the ${violating.length} violating cells, ${violating.length - wellFormed} are **malformed**`,
    "rankings (carrying a duplicate or a gap) and",
    `**${wellFormed}** are **well-formed**.`,
    "",
    "That split explains the table above. A malformed ranking always carries an",
    "error whose policy raises a dialog either way, so on those cells only the",
    "dialog's *kind* can change. The well-formed rankings — the ordinary ranked",
    "ballot — are where a dialog appears on a ballot the checker is content",
    "with, or the dialog a policy promises never fires.",
    "",
    "**What is still outside this analysis.** It decides what the spec says,",
    "over cells where `headless-sweep.md` has compared production against that",
    "spec. It does not observe a booth: whether these dialogs and messages",
    "render as predicted on a ranked ballot is checked per cell for the seven",
    "rule grids (`dom-validate.md`) but is spec-only for the wider ranked",
    "region, pending a generic IRV booth recipe (`README.md`, Open work).",
    "",
    "The mechanism is quirk `S6_GATES_COUNT_FIRST_PREFERENCES_ONLY`: the gates",    "The mechanism is quirk `S6_GATES_COUNT_FIRST_PREFERENCES_ONLY`: the gates",
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
            by_ballot_shape: {
                malformed_rankings: violating.length - wellFormed,
                well_formed_rankings: wellFormed,
                note:
                    "malformed rankings always carry an error whose policy raises a dialog " +
                    "either way, so only the dialog kind can change there; the well-formed " +
                    "ones are where a dialog appears from nothing or goes missing.",
            },
        },
        null,
        2
    ) + "\n"
)
console.log("\nwrote gate-count-agreement.md and gate-count-agreement.report.json")
if (missing.length) process.exitCode = 1
