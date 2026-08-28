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
//   S1 — no master mute (phase-3 judgment: every emitted error renders
//        inline; the ledger entry in lib.rs carries the grounds). Bites
//        where the ORACLE muted something: invalid ∈ {allowed,
//        allowed-with-exclusive-explicit} and some emitted error is absent
//        from the oracle's review view. Can change only the INLINE views.
//   D3 — the selectedMax alert deduped against the error copy, not itself.
//        Latent: the error copy is always present when the alert is, so this
//        changes NO cell — it appears here as a zero-cell fix, proof the
//        rewrite cannot reproduce the bug without changing behaviour.
//
// Signatures may overlap (a ranked over-vote under a double-allowed config is
// S6 ∧ S1), so attribution is a COVER check: every differing cell must have
// each moved field covered by a fix whose signature the cell matches. This
// runner does not assume the diff is only these — it derives the diff and
// requires the cover, both in the cell predicates and in which fields moved.
//
// One more acceptance, the property the S1 fix exists to establish: over the
// whole certified domain, f_fixed has ZERO silent-discount cells (tally =
// ImplicitInvalid ∧ no dialog ∧ nothing inline at either casting point ∧
// reachable) — silent discounting is unrepresentable in the rewrite.
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
const ALLOWED_FAMILY = new Set(["allowed", "allowed-with-exclusive-explicit"])

// Each fix: its cell signature (S1's reads the ORACLE's own certified output
// — did the mute bite here? — the others read the cell alone) and the fields
// it is allowed to move.
const FIXES = [
    {
        id: "S6",
        // The gate count (first preferences) and the checker count differ.
        matches: (c) =>
            c.voteState.firstPreferences !== undefined &&
            c.voteState.firstPreferences !== c.voteState.regulars,
        fields: new Set(["gate", "dialog"]),
    },
    {
        id: "S4",
        // The empty ballot in the zero-inclusive under-vote zone.
        matches: (c) =>
            n(c.voteState) === 0 && c.config.min === 0 && c.config.policies.under !== "allowed",
        fields: new Set(["emissions", "inline"]),
    },
    {
        id: "S1",
        // The oracle muted an emitted error: allowed-family invalid policy,
        // and some error key is absent from the oracle's review view.
        matches: (c, o) =>
            ALLOWED_FAMILY.has(c.config.policies.invalid) &&
            o.emissions.errors.some((e) => !o.inline.review.includes(e)),
        fields: new Set(["inline"]),
    },
]

const buckets = Object.fromEntries(FIXES.map((f) => [f.id, []]))
const combos = {}
const unexplained = []
let totalDiff = 0
for (let i = 0; i < cells.length; i++) {
    const fields = diffFields(oracle[i], fixed[i])
    if (fields.length === 0) continue
    totalDiff++
    const rec = {cell: cells[i], fields, oracle: oracle[i], fixed: fixed[i]}
    const matched = FIXES.filter((f) => f.matches(cells[i], oracle[i]))
    // COVER: every moved field must be movable by some matching fix.
    const covered = fields.every((fld) => matched.some((f) => f.fields.has(fld)))
    if (!covered || matched.length === 0) {
        unexplained.push(rec)
        continue
    }
    // Charge each matching fix that actually moved one of its fields.
    const charged = matched.filter((f) => fields.some((fld) => f.fields.has(fld)))
    for (const f of charged) buckets[f.id].push(rec)
    if (charged.length > 1) {
        const key = charged.map((f) => f.id).join("+")
        ;(combos[key] ??= []).push(rec)
    }
}

// The property the S1 fix establishes: silent discounting is unrepresentable
// in f_fixed — no reachable cell is discarded with nothing on any casting
// surface.
const silentOnFixed = []
for (let i = 0; i < cells.length; i++) {
    const x = fixed[i]
    if (
        x.tally === "ImplicitInvalid" &&
        x.dialog === "none" &&
        x.inline.voting.length === 0 &&
        x.inline.review.length === 0 &&
        x.reachability === "yes"
    ) {
        silentOnFixed.push(cells[i])
    }
}

console.log(`${cells.length} certified cells; ${totalDiff} differ between oracle and fixed`)
for (const f of FIXES) console.log(`  ${f.id}: ${buckets[f.id].length} cells`)
for (const [k, v] of Object.entries(combos)) console.log(`  (${k} jointly: ${v.length})`)
console.log(`  UNEXPLAINED: ${unexplained.length}`)
console.log(`  silent-discount cells on f_fixed: ${silentOnFixed.length} (must be 0)`)
for (const r of unexplained.slice(0, 10))
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

// ACCEPTANCE. No unexplained difference; each live fix must still bite (a
// fix that stops producing any cell means the rewrite regressed or the
// domain narrowed); and f_fixed must have zero silent-discount cells.
const ok =
    unexplained.length === 0 &&
    FIXES.every((f) => buckets[f.id].length > 0) &&
    silentOnFixed.length === 0
if (unexplained.length) console.log("\n! unexplained differences — the diff is not accounted for")
for (const f of FIXES)
    if (!buckets[f.id].length)
        console.log(`\n! ${f.id} no longer bites — domain narrowed or rewrite regressed`)
if (silentOnFixed.length)
    console.log("\n! f_fixed still has silent-discount cells — the S1 fix is not doing its job")

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
    "| S1 | the oracle muted an emitted error: `invalid ∈ {allowed, allowed-with-exclusive-explicit}` ∧ some error key absent from the oracle's review view | inline |",
    "| D3 | — | — (latent: changes no cell) |",
    "",
    "Signatures may overlap (a ranked over-vote under a double-allowed config",
    "is S6 ∧ S1); attribution is a **cover**: every moved field must be",
    "movable by a fix whose signature the cell matches, and a cell is counted",
    "under each fix that actually moved one of its fields.",
    "",
    `**Result: of ${cells.length} certified cells, ${totalDiff} differ — ` +
        FIXES.map((f) => `${buckets[f.id].length} ${f.id}`).join(", ") +
        `${
            Object.keys(combos).length
                ? " (" +
                  Object.entries(combos)
                      .map(([k, v]) => `${v.length} jointly ${k}`)
                      .join(", ") +
                  ")"
                : ""
        }, ${unexplained.length} unexplained. Silent-discount cells on f_fixed: ${silentOnFixed.length}.**`,
    "",
    "| fix | cells changed | what changes |",
    "|---|---|---|",
    `| S6 — one count for gate and checker | ${buckets.S6.length} | a ranked ballot the checker flags is now gated too (or a spurious gate on first-preferences is gone): the gate and the dialog |`,
    `| S4 — empty ballot is not an under-vote | ${buckets.S4.length} | the checker no longer emits an under-vote alert on the empty ballot at min=0; the emissions, and the inline view where it rendered |`,
    `| S1 — no master mute (phase-3 judgment; grounds in the lib.rs ledger) | ${buckets.S1.length} | every emitted error renders inline under the allowed-family invalid policies — the voter is informed; gates, dialog and tally unchanged ("informed but uninterrupted") |`,
    `| D3 — dedup against the error, not self | 0 | none — latent: the error copy is always present when the alert is, so the honest dedup drops the alert exactly as the buggy one did |`,
    `| **unexplained** | **${unexplained.length}** | must be zero |`,
    "",
    "**The property the S1 fix establishes:** over the whole certified domain,",
    `f_fixed has **${silentOnFixed.length} silent-discount cells** (tally = ImplicitInvalid ∧ no`,
    "dialog ∧ nothing inline at either casting point ∧ reachable) — must be",
    "zero; the oracle has 6,336 (`no-silent-discount.md`). Silent discounting",
    "is unrepresentable in the rewrite, not merely absent.",
    "",
    "## An S6 cell",
    "",
    ...example(buckets.S6[0]),
    "## An S4 cell",
    "",
    ...example(buckets.S4[0]),
    "## An S1 cell",
    "",
    ...example(buckets.S1.find((r) => r.oracle.tally === "ImplicitInvalid" && r.oracle.dialog === "none") ?? buckets.S1[0]),
    ...(unexplained.length
        ? ["## UNEXPLAINED cells (this run FAILS)", "", ...unexplained.slice(0, 20).flatMap(example)]
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
                S1: buckets.S1.length,
                D3_latent: 0,
                combinations: Object.fromEntries(
                    Object.entries(combos).map(([k, v]) => [k, v.length])
                ),
                unexplained: unexplained.length,
            },
            silent_discount_cells_on_fixed: silentOnFixed.length,
            examples: {
                S6: buckets.S6[0] ?? null,
                S4: buckets.S4[0] ?? null,
                S1: buckets.S1[0] ?? null,
            },
            unexplained: unexplained.map((r) => ({cell: r.cell, fields: r.fields})),
            accepted: ok,
        },
        null,
        2
    ) + "\n"
)
console.log("\nwrote fix-diff.md and fix-diff.report.json")
if (!ok) process.exitCode = 1
