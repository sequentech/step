// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The fix diff — ANALYSIS: the rationalized implementation (`f_fixed`, the
// query-provider) against the frozen oracle (`f`, production's bug-compatible
// behaviour), over the certified domain (distillation step 5, phase 2).
//
// The oracle stays byte-identical to production (that is what headless-sweep
// certifies). The fixed implementation is written "as if the bug had never
// existed", so it is MEANT to diverge — and this runner is the artifact that
// says exactly where, and attributes every difference to one intended fix.
// It is the acceptance check for the fork and the review artifact upstream
// reads: a difference that no intended fix explains is either a regression in
// the rewrite or an uncatalogued quirk, and either way this must fail.
//
// The fixes, and their signatures on a cell:
//
//   S6 — one selection count for gates and checker (was: gates counted first
//        preferences). Bites where the two counts differ: firstPreferences ≠
//        regulars (a ranked ballot). Can change only the GATE and the DIALOG.
//   S4 — one under-vote predicate, excluding the empty ballot (was: the
//        checker's zone included n = 0 when min = 0). Bites where n = 0,
//        min = 0, under ≠ allowed. Can change only the EMISSIONS (the
//        under-vote alert) and, if it rendered, the INLINE view.
//   D3 — the selectedMax alert deduped against the error copy, not itself.
//        Latent: the error copy is always present when the alert is, so this
//        changes NO cell — it appears here as a zero-cell fix, proof the
//        rewrite cannot reproduce the bug without changing behaviour.
//
// The two live signatures are disjoint (S4 needs n = 0, i.e. regulars = 0, so
// firstPreferences = 0 = regulars, which is not S6). This runner does not
// assume the diff is only these — it derives the diff and REQUIRES every
// differing cell to match one signature, both in the cell predicate and in
// which fields moved.
//
// Headless; needs cargo only. Writes fix-diff.md + .report.json; exits
// nonzero on any unexplained difference, or if a known fix stops biting.
//
// Run:  node characterization/fix-diff.mjs   (from packages/workbench)

import {writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {specF, specFixed} from "./rust-spec.mjs"
import {certifiedCells} from "./domain.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const cells = certifiedCells()
const oracle = specF(cells)
const fixed = specFixed(cells)

const FIELDS = ["emissions", "inline", "gate", "dialog", "reachability", "tally"]
const diffFields = (a, b) => FIELDS.filter((k) => JSON.stringify(a[k]) !== JSON.stringify(b[k]))

const n = (vs) => vs.regulars + (vs.blankMarker ? 1 : 0) + (vs.explicitInvalid ? 1 : 0)
// S6 bites where the gate count (first preferences) and the checker count
// (all ranked) differ.
const isS6 = (c) =>
    c.voteState.firstPreferences !== undefined &&
    c.voteState.firstPreferences !== c.voteState.regulars
// S4 bites on the empty ballot in the zero-inclusive zone.
const isS4 = (c) => n(c.voteState) === 0 && c.config.min === 0 && c.config.policies.under !== "allowed"

// Which fields each fix is allowed to move — the second half of attribution.
const S6_FIELDS = new Set(["gate", "dialog"])
const S4_FIELDS = new Set(["emissions", "inline"])
const subset = (fields, allowed) => fields.every((f) => allowed.has(f))

const buckets = {S6: [], S4: [], both: [], unexplained: []}
for (let i = 0; i < cells.length; i++) {
    const fields = diffFields(oracle[i], fixed[i])
    if (fields.length === 0) continue
    const rec = {cell: cells[i], fields, oracle: oracle[i], fixed: fixed[i]}
    const s6 = isS6(cells[i])
    const s4 = isS4(cells[i])
    if (s6 && s4) buckets.both.push(rec)
    else if (s6 && subset(fields, S6_FIELDS)) buckets.S6.push(rec)
    else if (s4 && subset(fields, S4_FIELDS)) buckets.S4.push(rec)
    else buckets.unexplained.push(rec)
}

const totalDiff = buckets.S6.length + buckets.S4.length + buckets.both.length + buckets.unexplained.length

console.log(`${cells.length} certified cells; ${totalDiff} differ between oracle and fixed`)
console.log(`  S6 (gate/dialog on a ranked ballot) : ${buckets.S6.length}`)
console.log(`  S4 (under-vote alert on empty, min=0): ${buckets.S4.length}`)
console.log(`  both signatures                     : ${buckets.both.length}`)
console.log(`  UNEXPLAINED                         : ${buckets.unexplained.length}`)
for (const r of buckets.unexplained.slice(0, 10))
    console.log("    ✗ " + JSON.stringify({cell: r.cell, fields: r.fields}))

const fmtCell = (c) =>
    `min=${c.config.min} max=${c.config.max} ` +
    `regulars=${c.voteState.regulars} firstPreferences=${c.voteState.firstPreferences} ` +
    `blankMarker=${c.voteState.blankMarker} explicitInvalid=${c.voteState.explicitInvalid} ` +
    `dup=${c.voteState.duplicateRanks} gap=${c.voteState.rankGaps} | ` +
    Object.entries(c.config.policies)
        .map(([k, v]) => `${k}=${v}`)
        .join(" ")

const example = (rec) =>
    rec
        ? [
              "```",
              fmtCell(rec.cell),
              `  changed: ${rec.fields.join(", ")}`,
              ...rec.fields.map(
                  (f) =>
                      `    ${f}: oracle=${JSON.stringify(rec.oracle[f])}  fixed=${JSON.stringify(
                          rec.fixed[f]
                      )}`
              ),
              "```",
              "",
          ]
        : ["(no cell in this bucket)", ""]

// ACCEPTANCE. No unexplained difference; and each live fix must still bite
// (a fix that stops producing any cell means the rewrite regressed or the
// domain narrowed).
const ok =
    buckets.unexplained.length === 0 && buckets.S6.length > 0 && buckets.S4.length > 0
if (buckets.unexplained.length) console.log("\n! unexplained differences — the diff is not accounted for")
if (!buckets.S6.length) console.log("\n! S6 no longer bites — domain narrowed or rewrite regressed")
if (!buckets.S4.length) console.log("\n! S4 no longer bites — domain narrowed or rewrite regressed")

const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Fix diff — rationalized implementation vs the frozen oracle",
    "",
    "Generated by `characterization/fix-diff.mjs`; do not edit by hand.",
    "",
    "**What this is.** The rationalized implementation (`f_fixed`, the",
    "query-provider in `../validation-spec/src/queries.rs`) against the frozen",
    "oracle (`f`, production's bug-compatible behaviour — what",
    "`headless-sweep.md` certifies production equals). The oracle is the",
    "\"before\"; the fixed implementation is written as if the bugs had never",
    "existed, so it is meant to diverge. This is the review artifact: it names",
    "every difference and attributes it to one intended fix. **A difference no",
    "fix explains fails the run** — it is a regression in the rewrite or an",
    "uncatalogued quirk.",
    "",
    "**How attribution works.** A difference is charged to a fix only if the",
    "cell matches the fix's signature AND only the fields that fix can move",
    "have moved:",
    "",
    "| fix | cell signature | fields it may move |",
    "|---|---|---|",
    "| S6 | `firstPreferences ≠ regulars` (a ranked ballot) | gate, dialog |",
    "| S4 | `n = 0 ∧ min = 0 ∧ under ≠ allowed` (empty ballot, zero-zone) | emissions, inline |",
    "| D3 | — | — (latent: changes no cell) |",
    "",
    `**Result: of ${cells.length} certified cells, ${totalDiff} differ — ` +
        `${buckets.S6.length} S6, ${buckets.S4.length} S4, ${buckets.unexplained.length} unexplained.**`,
    "",
    "| fix | cells changed | what changes |",
    "|---|---|---|",
    `| S6 — one count for gate and checker | ${buckets.S6.length} | a ranked ballot the checker flags is now gated too (or a spurious gate on first-preferences is gone): the gate and the dialog |`,
    `| S4 — empty ballot is not an under-vote | ${buckets.S4.length} | the checker no longer emits an under-vote alert on the empty ballot at min=0; the emissions, and the inline view where it rendered |`,
    `| D3 — dedup against the error, not self | 0 | none — latent: the error copy is always present when the alert is, so the honest dedup drops the alert exactly as the buggy one did |`,
    `| **unexplained** | **${buckets.unexplained.length}** | must be zero |`,
    "",
    "## An S6 cell",
    "",
    ...example(buckets.S6[0]),
    "## An S4 cell",
    "",
    ...example(buckets.S4[0]),
    ...(buckets.unexplained.length
        ? ["## UNEXPLAINED cells (this run FAILS)", "", ...buckets.unexplained.slice(0, 20).flatMap(example)]
        : []),
    "**What is outside this analysis.** It decides what the two implementations",
    "say, over the cells `headless-sweep.md` has compared production against the",
    "oracle. It does not re-observe a booth; the fixed implementation is a",
    "workbench reference, not injected into production (that is a separate",
    "branch — see `README.md`).",
    "",
].join("\n")

writeFileSync(path.join(here, "fix-diff.md"), md)
writeFileSync(
    path.join(here, "fix-diff.report.json"),
    JSON.stringify(
        {
            cells_evaluated: cells.length,
            differ: totalDiff,
            by_fix: {
                S6: buckets.S6.length,
                S4: buckets.S4.length,
                both: buckets.both.length,
                D3_latent: 0,
                unexplained: buckets.unexplained.length,
            },
            examples: {
                S6: buckets.S6[0] ?? null,
                S4: buckets.S4[0] ?? null,
            },
            unexplained: buckets.unexplained.map((r) => ({cell: r.cell, fields: r.fields})),
            accepted: ok,
        },
        null,
        2
    ) + "\n"
)
console.log("\nwrote fix-diff.md and fix-diff.report.json")
if (!ok) process.exitCode = 1
