// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Browser witness validation — stage 2 of the dependency-driven validation
// pipeline. Takes the browser-pending dependence witnesses from
// `effect-dependencies.recorded.json` (inline-view and reachability
// components — the effects only a real booth can observe) and drives each
// witness's two cells through the booth, comparing the observed value
// against `spec.f` on both. A dependence claim is existential, so the
// witness pair settles it: both cells matching the spec confirms the
// dependence exists in production with the predicted values.
//
// Witness cells are arbitrary points of the spec domain, so this uses a
// GENERIC booth recipe rather than the per-rule ones:
//   - contest by vote state: the explicit-invalid flag needs the Council
//     contest (its null-vote marker sets the flag; Referendum has no
//     route to it in the booth), the blank marker needs Referendum; a
//     cell needing BOTH has no booth contest on this fixture (labelled).
//   - config through the Policy-overrides panel: all four panel-settable
//     policies and both bounds are set explicitly per cell (overrides
//     accumulate per contest — the CLAUDE.md gotcha). dup/gap policies
//     are hidden on plurality contests and provably inert here
//     (headless-sweep.md swept them; spec's filter never reads them).
//   - deterministic touch-arming (tick-untick of the landmark), then the
//     state clicks: regulars, then the marker (so a marker-clear is
//     observed, not raced).
//
// Labels (never silently dropped): cells the booth cannot represent
// (marker+flag, regulars > candidates, max = 0), inline observations on
// states prevention keeps from forming, and review observations behind a
// hard gate. Production's gates are INJECTED (voting_screen.rs routes
// through the query-provider), so whether review is reachable is decided
// by the RATIONALIZED implementation's hard gate (f_fixed — certified
// against production by headless-sweep); the witness comparisons
// themselves (inline views, reachability) stay against the frozen oracle
// `f`, whose components are not injected.
//
// Requires the dev server on :5173. Writes browser-witnesses.md +
// .recorded.json; exits nonzero if any observed cell disagrees with the
// spec.
//
// Run:  node characterization/browser-witnesses.mjs   (from packages/workbench)

import {createRequire} from "node:module"
import {readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {loadSnapshot} from "./browser-harness.mjs"
import {boothContext, observeCell, boothFormable, shortKey} from "./booth-cell.mjs"
// One-cell batches: this runner evaluates a couple of hundred cells in
// total, so the per-call subprocess cost is irrelevant here.
import {specF as specBatch, specFixed as specFixedBatch} from "./rust-spec.mjs"
const specF = (config, voteState) => specBatch([{config, voteState}])[0]
const specFixed = (config, voteState) => specFixedBatch([{config, voteState}])[0]

const require = createRequire("C:/work/projects/step/packages/")
const {chromium} = require("playwright")

const here = path.dirname(fileURLToPath(import.meta.url))
const base = "http://localhost:5173"
const ELECTION = "44444444-4444-4444-4444-444444444003"
const SNAPSHOT = "bundled:explicit-blank-invalid"

const deps = JSON.parse(
    readFileSync(path.join(here, "effect-dependencies.recorded.json"), "utf8")
)

const HEADLESS = (name) =>
    name.endsWith("∈ errors") ||
    name.endsWith("∈ alerts") ||
    ["gate.hard", "gate.soft", "dialog", "tally"].includes(name)

const parseCell = (cell) => ({
    config: {
        min: Number(cell.min_votes),
        max: Number(cell.max_votes),
        policies: {
            invalid: cell.invalid_vote_policy,
            blank: cell.blank_vote_policy,
            over: cell.over_vote_policy,
            under: cell.under_vote_policy,
            dup: cell.duplicated_rank_policy,
            gap: cell.preference_gaps_policy,
        },
    },
    voteState: {
        regulars: Number(cell.regulars),
        blankMarker: cell.blank_marker === "true",
        explicitInvalid: cell.explicit_invalid === "true",
        decline: cell.decline === "true",
        duplicateRanks: cell.duplicate_ranks === "true",
        rankGaps: cell.rank_gaps === "true",
    },
})

/** Why a witness cannot be booth-observed, or null if it can. */
function boothDefer(component, cells) {
    for (const cell of cells) {
        const parsed = parseCell(cell)
        const formable = boothFormable(parsed)
        if (formable) return formable
        if (parsed.config.max === 0) return "max_votes = 0 (config-sanity scope boundary)"
        const spec = specF(parsed.config, parsed.voteState)
        if (component.includes("inline") && spec.reachability !== "yes")
            return "state prevented in the booth (inline unobservable there)"
        // Review reachability is decided by production's ACTUAL gate — the
        // injected, rationalized one (see the header).
        if (
            component.endsWith("inline.review") &&
            specFixed(parsed.config, parsed.voteState).gate.hard
        )
            return "unobservable by construction (hard gate precludes review; the dialog is the signal)"
    }
    return null
}

// ---------------------------------------------------------------------------
// Booth driver — shared with quotient-validate (booth-cell.mjs)
// ---------------------------------------------------------------------------
const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()
await loadSnapshot(page, base, SNAPSHOT)
const ctx = await boothContext(page, ELECTION)

/** Project a booth observation onto a component value. */
function observedValue(component, cell, obs) {
    if (component.endsWith("∈ inline.voting"))
        return obs.inlineVoting.includes(component.split(" ")[0])
    if (component.endsWith("∈ inline.review"))
        return (obs.inlineReview ?? []).includes(component.split(" ")[0])
    if (component === "reachability") {
        const {config, voteState: vs} = cell
        const wantFormed = vs.regulars + (vs.blankMarker ? 1 : 0)
        const asRequested = obs.sel.formed === wantFormed && obs.sel.flag === vs.explicitInvalid
        if (asRequested) return "yes"
        if (vs.blankMarker && vs.regulars > 0 && obs.sel.formed < wantFormed)
            return "marker_cleared"
        if (
            config.policies.over === "not-allowed-with-msg-and-disable" &&
            vs.regulars + (vs.blankMarker ? 1 : 0) + (vs.explicitInvalid ? 1 : 0) > config.max &&
            obs.sel.formed < wantFormed
        )
            return "inputs_disabled"
        return `unexpected state (formed=${obs.sel.formed}, flag=${obs.sel.flag})`
    }
    throw new Error(`not a browser component: ${component}`)
}

function specValue(component, {config, voteState}) {
    const e = specF(config, voteState)
    if (component.endsWith("∈ inline.voting"))
        return e.inline.voting.map(shortKey).includes(component.split(" ")[0])
    if (component.endsWith("∈ inline.review"))
        return e.inline.review.map(shortKey).includes(component.split(" ")[0])
    if (component === "reachability") return e.reachability
    throw new Error(`not a browser component: ${component}`)
}

// ---------------------------------------------------------------------------
// Run every browser-pending witness
// ---------------------------------------------------------------------------
const checked = []
const deferred = []
const disagreements = []
for (const comp of deps.components) {
    if (HEADLESS(comp.component) || comp.constant) continue
    for (const w of comp.witnesses) {
        const cellA = {...w.cell, [w.varies]: w.from}
        const cellB = {...w.cell, [w.varies]: w.to}
        const reason = boothDefer(comp.component, [cellA, cellB])
        if (reason) {
            deferred.push({component: comp.component, varies: w.varies, reason})
            continue
        }
        const row = {component: comp.component, varies: w.varies, cells: [], ok: true}
        for (const cell of [cellA, cellB]) {
            const inputs = parseCell(cell)
            const obs = await observeCell(page, ctx, inputs)
            const got = observedValue(comp.component, inputs, obs)
            const want = specValue(comp.component, inputs)
            row.cells.push({cell, spec: String(want), production: String(got)})
            if (String(got) !== String(want)) row.ok = false
        }
        checked.push(row)
        if (!row.ok) disagreements.push(row)
        console.log(
            `  ${row.ok ? "✓" : "✗"} ${comp.component} varying ${w.varies}` +
                (row.ok ? "" : ` — ${JSON.stringify(row.cells)}`)
        )
    }
}
await browser.close()
console.log(
    `\n${checked.length} witnesses booth-confirmed pairs run, ` +
        `${disagreements.length} disagreements; ${deferred.length} deferred (labelled)`
)

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------
writeFileSync(
    path.join(here, "browser-witnesses.recorded.json"),
    JSON.stringify({checked, deferred, disagreements}, null, 2) + "\n"
)
const byReason = {}
for (const d of deferred) byReason[d.reason] = (byReason[d.reason] ?? 0) + 1
const md = [
    "<!--",
    " SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>",
    "",
    "SPDX-License-Identifier: AGPL-3.0-only",
    "-->",
    "",
    "# Browser witness validation — inline and reachability dependences",
    "",
    "Generated by `characterization/browser-witnesses.mjs`; do not edit by hand.",
    "",
    "**Experiment.** Every browser-pending dependence witness from",
    "`effect-dependencies.md` — a concrete cell pair proving that an",
    "inline-view or reachability component depends on an input dimension —",
    "is driven through the real booth (panel-configured, touch-armed,",
    "regulars before markers) and the observed component value is compared",
    "against `spec.f` on both cells. A dependence claim is existential, so",
    "the pair settles it. Witnesses the booth cannot observe are labelled",
    "below, never dropped.",
    "",
    "Reading the labels: *unobservable by construction* is a disposition,",
    "not a debt — review never renders under a hard gate, so the spec's",
    "inline.review value there is a counterfactual no booth can exhibit (the",
    "dialog is the signal; the gate itself is production-certified by",
    "`headless-sweep.md`). The *preferential state* deferrals are mostly",
    "already exhibited by `dom-validate.md`'s duplicate-rank and",
    "preference-gaps tables, which vary the same dimensions cell-by-cell —",
    "the label means this runner has not re-run them itself (IRV",
    "generic recipe pending). The *marker + flag* deferrals await a fixture",
    "whose contest carries both markers.",
    "",
    `**Result: ${checked.length} witnesses confirmed, ${disagreements.length} disagreement(s); ` +
        `${deferred.length} deferred:** ` +
        Object.entries(byReason)
            .map(([r, n]) => `${r}: ${n}`)
            .join("; ") +
        ".",
    "",
    "| component | varies | cells (spec = production) |",
    "|---|---|---|",
    ...checked.map(
        (r) =>
            `| ${r.ok ? "" : "**✗** "}${r.component} | ${r.varies} | ` +
            r.cells.map((c) => c.production).join(" / ") +
            " |"
    ),
    "",
].join("\n")
writeFileSync(path.join(here, "browser-witnesses.md"), md)
console.log("wrote browser-witnesses.md and browser-witnesses.recorded.json")
if (disagreements.length) process.exitCode = 1
