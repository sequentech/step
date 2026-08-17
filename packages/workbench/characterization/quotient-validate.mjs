// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Quotient validation — stage 3 (final) of the dependency-driven validation
// pipeline: discharges the browser-side INDEPENDENCE claims on the
// representable subdomain by sufficiency (conditional independence given a
// computed mediator) instead of by brute force.
//
// THE LICENSE (re-verified by source read, 2026-08-17, of
// voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx):
// `filterErrorList` is a closure whose body references NOTHING beyond its
// explicit parameters — the decoded record (checker errors/alerts), the
// four consulted policies (read raw from `question.presentation`, no
// default resolution), `isReview`, `isTouched`, and the dead
// `isVotedState` (console.log only — UPSTREAM_FINDINGS.md Defect 4). No
// store reads, no globals. The inline effect therefore depends on the
// inputs ONLY THROUGH (emissions, invalid, blank, over, under, point) —
// so one booth run per reachable class of that tuple covers the inline
// behaviour of EVERY cell in the class. RE-ENTRY CONDITION: any future
// reference inside filterErrorList beyond its parameter list, or a new
// consulted policy/prop, breaks this license — re-verify it on every
// portal refresh (LIFTING.md runbook) before trusting this artifact.
//
// Spec-side, the same factorization was checked extensionally: the
// headless sweep asserted inline is constant within every class across all
// 138,240 cells (headless-sweep.md).
//
// Method: for each of the quotient classes emitted by headless-sweep, find
// a BOOTH-FORMABLE member (the sweep's stored representative was chosen
// headlessly and may be unformable — flag+marker, marker-clear collapse,
// disable-prevented, max = 0): search bounds × formable vote states for
// one whose spec emissions under the class's policies match the class key
// (sound — emissions ≡ production on this subdomain by the sweep). Drive
// it, compare inline at the touched voting screen and at review against
// the class's spec inline views. The hard gate is class-determined (its
// clauses key off emissions-visible facts and policies), so review is
// compared exactly when the class does not hard-gate — otherwise the
// dialog is the signal, and it is certified headlessly.
//
// Classes with no booth-formable member are LABELLED, never dropped.
//
// Requires the dev server on :5173. Writes quotient-validate.md +
// .recorded.json; exits nonzero on any disagreement. ~2 h for ~2,300
// classes; progress every 100.
//
// Run:  node characterization/quotient-validate.mjs   (from packages/workbench)

import {createRequire} from "node:module"
import {readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {performance} from "node:perf_hooks"
import {loadSnapshot} from "./browser-harness.mjs"
import {boothContext, observeCell, boothFormable, shortKey} from "./booth-cell.mjs"
import {f as specF} from "./spec.mjs"

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const SNAPSHOT = "bundled:explicit-blank-invalid"

const sweep = JSON.parse(
    readFileSync(path.join(here, "headless-sweep.recorded.json"), "utf8")
)
const classes = sweep.quotient.classes

const sortedUniq = (xs) => [...new Set(xs)].sort()
const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b)

// Booth-formable vote states (see booth-cell.mjs): no marker+flag, no
// marker+regulars (the blank reducer collapses it), regulars ≤ 2.
const FORMABLE_STATES = []
for (let regulars = 0; regulars <= 2; regulars++) {
    FORMABLE_STATES.push({regulars, blankMarker: false, explicitInvalid: false})
    FORMABLE_STATES.push({regulars, blankMarker: false, explicitInvalid: true})
}
FORMABLE_STATES.push({regulars: 0, blankMarker: true, explicitInvalid: false})
const BOUNDS = []
for (let min = 0; min <= 3; min++)
    for (let max = 1; max <= 3; max++) if (min <= max) BOUNDS.push([min, max])

/** Find a booth-formable, booth-reachable member of a class, or null. */
function formableMember(cls) {
    const [errors, alerts, invalid, blank, over, under] = cls.key
    const policies = {invalid, blank, over, under, dup: "allowed-warn-and-dialog", gap: "allowed-warn-and-dialog"}
    for (const [min, max] of BOUNDS) {
        for (const state of FORMABLE_STATES) {
            const cell = {config: {min, max, policies}, voteState: {...state}}
            if (boothFormable(cell)) continue
            const e = specF(cell.config, cell.voteState)
            if (e.reachability !== "yes") continue
            if (
                eq(sortedUniq(e.emissions.errors.map(shortKey)), errors) &&
                eq(sortedUniq(e.emissions.alerts.map(shortKey)), alerts)
            ) {
                return {cell, hard: e.gate.hard}
            }
        }
    }
    return null
}

const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()
await loadSnapshot(page, base, SNAPSHOT)
const ctx = await boothContext(page, ELECTION)

const checked = []
const deferred = []
const disagreements = []
let done = 0
const t0 = performance.now()
for (const cls of classes) {
    done++
    const member = formableMember(cls)
    if (!member) {
        deferred.push({key: cls.key, cells: cls.cells, reason: "no booth-formable member (state prevented or unformable on this fixture)"})
        continue
    }
    const obs = await observeCell(page, ctx, member.cell)
    const gotVoting = sortedUniq(obs.inlineVoting)
    const wantVoting = cls.spec_inline.voting
    const votingOk = eq(gotVoting, wantVoting)
    let reviewOk = true
    let gotReview = null
    if (!member.hard) {
        gotReview = sortedUniq(obs.inlineReview ?? [])
        reviewOk = eq(gotReview, cls.spec_inline.review)
    }
    const row = {
        key: cls.key,
        cells: cls.cells,
        member: member.cell,
        voting: {want: wantVoting, got: gotVoting},
        review: member.hard ? "hard-gated (dialog is the signal)" : {want: cls.spec_inline.review, got: gotReview},
        ok: votingOk && reviewOk,
    }
    checked.push(row)
    if (!row.ok) {
        disagreements.push(row)
        console.log(`  ✗ class ${JSON.stringify(cls.key)} — ${JSON.stringify(row)}`)
    }
    if (done % 100 === 0) {
        console.log(
            `  ${done}/${classes.length} classes; ${checked.length} run, ` +
                `${disagreements.length} disagreements, ${deferred.length} deferred; ` +
                `${Math.round((performance.now() - t0) / 1000)}s`
        )
    }
}
await browser.close()

const coveredCells = checked.filter((r) => r.ok).reduce((n, r) => n + r.cells, 0)
const deferredCells = deferred.reduce((n, r) => n + r.cells, 0)
console.log(
    `\n${checked.length}/${classes.length} classes booth-validated ` +
        `(${disagreements.length} disagreements), covering ${coveredCells} cells by sufficiency; ` +
        `${deferred.length} classes (${deferredCells} cells) deferred`
)

writeFileSync(
    path.join(here, "quotient-validate.recorded.json"),
    JSON.stringify(
        {
            license:
                "filterErrorList reads only (record, invalid, blank, over, under, isReview, isTouched); " +
                "isVotedState dead (Defect 4). Source-verified 2026-08-17. Re-verify on portal refresh.",
            classes_total: classes.length,
            checked,
            deferred,
            disagreements,
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
    "# Quotient validation — browser independence claims by sufficiency",
    "",
    "Generated by `characterization/quotient-validate.mjs`; do not edit by hand.",
    "",
    "**Experiment.** The final stage of the dependency-driven validation",
    "pipeline: the spec's browser-side *independence* claims (which inputs",
    "the inline views provably do not depend on) are universal, and the",
    "booth is too slow to discharge them by sweeping. They are discharged",
    "instead by **sufficiency** — conditional independence given a computed",
    "mediator: the booth's message filter reads the inputs only through the",
    "checker record, its four consulted policies, and the observation point,",
    "so one booth run per reachable class of that tuple covers the inline",
    "behaviour of every cell in the class. One booth-formable member of each",
    "class from `headless-sweep.md`'s quotient inventory is driven through",
    "the real booth and its inline content at the touched voting screen and",
    "at review is compared against the class's spec prediction. The hard",
    "gate is class-determined, so review is compared exactly where it",
    "renders; hard-gated classes' review is the dialog, certified headlessly.",
    "",
    "**The license** (what makes one-per-class sound), source-verified",
    "2026-08-17: `filterErrorList` is a closure referencing nothing beyond",
    "its parameters — the record, the four policies (read raw from",
    "`question.presentation`), `isReview`, `isTouched`, and the dead",
    "`isVotedState` (Defect 4). No store reads, no globals. Spec-side, the",
    "same factorization was checked extensionally over all 138,240 swept",
    "cells. **Re-entry condition:** any reference inside `filterErrorList`",
    "beyond its parameter list, or a new consulted policy, voids this",
    "artifact — re-verify the boundary on every portal refresh.",
    "",
]
const okCells = checked.filter((r) => r.ok).reduce((n, r) => n + r.cells, 0)
md.push(
    `**Result: ${checked.length}/${classes.length} classes booth-validated, ` +
        `${disagreements.length} disagreement(s) — covering ` +
        `${okCells.toLocaleString("en-US")} of the subdomain's cells by sufficiency. ` +
        `${deferred.length} classes (${deferred.reduce((n, r) => n + r.cells, 0)} cells) deferred: ` +
        "no booth-formable member (their states are prevention-collapsed, " +
        "flag+marker, or otherwise unformable on this fixture; their headless " +
        "effects are already sweep-certified, and their inline values remain " +
        "spec-only until a fixture can form them).**",
    ""
)
if (disagreements.length) {
    md.push("## DISAGREEMENTS", "")
    for (const d of disagreements) md.push(`- ${JSON.stringify(d)}`)
    md.push("")
}
writeFileSync(path.join(here, "quotient-validate.md"), md.join("\n") + "\n")
console.log("wrote quotient-validate.md and quotient-validate.recorded.json")
if (disagreements.length) process.exitCode = 1
