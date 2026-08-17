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
// hard gate (the gate values on this subdomain are production-certified
// by headless-sweep, so the spec's gate is a sound pre-filter).
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
import {
    warnIds,
    dialogKind,
    setPanelConfig,
    enterBooth,
    clearSelections,
    dismissDialog,
    backToInspector,
    loadSnapshot,
} from "./browser-harness.mjs"
import {f as specF} from "./spec.mjs"

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

const shortKey = (k) => k.split(".").pop()

/** Why a witness cannot be booth-observed, or null if it can. */
function boothDefer(component, cells) {
    for (const cell of cells) {
        const {config, voteState: vs} = parseCell(cell)
        if (vs.duplicateRanks || vs.rankGaps) return "preferential state (IRV recipe pending)"
        if (vs.decline) return "decline (no booth route)"
        if (vs.regulars > 2) return "regulars > 2 (no fixture)"
        if (config.max === 0) return "max_votes = 0 (config-sanity scope boundary)"
        if (vs.blankMarker && vs.explicitInvalid)
            return "blank marker + invalid flag (no booth contest carries both)"
        const spec = specF(config, vs)
        if (component.includes("inline") && spec.reachability !== "yes")
            return "state prevented in the booth (inline unobservable there)"
        if (component.endsWith("inline.review") && spec.gate.hard)
            return "unobservable by construction (hard gate precludes review; the dialog is the signal)"
    }
    return null
}

// ---------------------------------------------------------------------------
// Generic booth driver
// ---------------------------------------------------------------------------
const browser = await chromium.launch({channel: "chrome", headless: true})
const page = await browser.newPage()
await loadSnapshot(page, base, SNAPSHOT)

const CONTESTS = await page.evaluate((electionId) => {
    const bs = window.__store.getState().ballotStyles[electionId]
    const byFlag = (flag) =>
        bs.ballot_eml.contests.find((x) => x.candidates.some((cd) => cd.presentation?.[flag]))
    const raw = localStorage.getItem("workbench:state:v1")
    return {
        referendum: byFlag("is_explicit_blank").id,
        council: byFlag("is_explicit_invalid").id,
        voter: raw ? JSON.parse(raw)?.workbench?.voters?.[0]?.id ?? null : null,
    }
}, ELECTION)

const RECIPES = {
    referendum: {
        contestId: CONTESTS.referendum,
        landmark: /^Yes$/,
        regulars: [/^Yes$/, /^No$/],
        marker: "Blank vote (explicit blank)",
    },
    council: {
        contestId: CONTESTS.council,
        landmark: /^Ada$/,
        regulars: [/^Ada$/, /^Bruno$/],
        marker: "Null vote (explicit invalid)",
    },
}

const clickText = (rx) => page.getByText(rx).first().click().catch(() => {})
const clickExact = (s) => page.getByText(s, {exact: true}).first().click().catch(() => {})

// Platform defaults + fixture bounds, for neutralizing the contest a cell
// does NOT target: overrides accumulate per contest (the CLAUDE.md gotcha),
// the review screen renders every contest, and warnIds reads the whole
// page — a stale override on the other contest would pollute the
// observation (it did: the first run's three "disagreements" were exactly
// this artifact).
const NEUTRAL = {
    selects: {
        "Invalid-vote policy": "allowed",
        "Blank-vote policy": "allowed",
        "Over-vote policy": "allowed-with-msg-and-alert",
        "Under-vote policy": "allowed",
    },
}
const FIXTURE_BOUNDS = {referendum: {min_votes: 0, max_votes: 2}, council: {min_votes: 0, max_votes: 1}}

/** Drive one arbitrary plurality cell and observe inline (touched voting +
 *  review), the selection state, and the dialog. */
async function observeCell({config, voteState: vs}) {
    const recipe = vs.explicitInvalid ? RECIPES.council : RECIPES.referendum
    const otherName = vs.explicitInvalid ? "referendum" : "council"
    await setPanelConfig(page, RECIPES[otherName].contestId, {
        ...NEUTRAL,
        bounds: FIXTURE_BOUNDS[otherName],
    })
    // Back-to-back panel configs on different contests are safe: the
    // navigation race this used to trip is fixed inside setPanelConfig
    // (browser-harness.mjs).
    await setPanelConfig(page, recipe.contestId, {
        selects: {
            "Invalid-vote policy": config.policies.invalid,
            "Blank-vote policy": config.policies.blank,
            "Over-vote policy": config.policies.over,
            "Under-vote policy": config.policies.under,
        },
        bounds: {min_votes: config.min, max_votes: config.max},
    })
    await enterBooth(page, CONTESTS.voter)
    await page.getByText(recipe.landmark).first().waitFor({timeout: 15000})
    await clearSelections(page)
    // Deterministic touch: tick-untick the landmark so the untouched-clear
    // never masks the voting observation (rule-specs.mjs does the same).
    await clickText(recipe.landmark)
    await clickText(recipe.landmark)
    // Form the state: regulars first, marker last (a marker-clear must be
    // an observed collapse, not a race).
    for (let i = 0; i < vs.regulars; i++) await clickText(recipe.regulars[i])
    if (vs.blankMarker) await clickExact(RECIPES.referendum.marker)
    if (vs.explicitInvalid) await clickExact(RECIPES.council.marker)

    const sel = await page.evaluate(
        ({electionId, contestId}) => {
            const s = window.__store.getState().ballotSelections[electionId] ?? []
            const c = s.find((x) => x.contest_id === contestId)
            return {
                formed: c ? c.choices.filter((ch) => ch.selected === 0).length : 0,
                flag: !!(c && c.is_explicit_invalid),
                selected: c ? c.choices.map((ch) => ch.selected) : [],
            }
        },
        {electionId: ELECTION, contestId: recipe.contestId}
    )
    const inlineVoting = (await warnIds(page)).map(shortKey)

    let dialog = "none"
    let inlineReview = null
    const next = page.getByRole("button", {name: /next|review/i}).first()
    if (await next.count().catch(() => 0)) {
        await next.click().catch(() => {})
        dialog = await dialogKind(page)
    }
    if (dialog === "dismissible") {
        await page.getByRole("button", {name: /continue/i}).first().click().catch(() => {})
    }
    if (dialog !== "blocking") {
        await page.locator(".cast-ballot-button").first().waitFor({timeout: 8000}).catch(() => {})
        inlineReview = (await warnIds(page)).map(shortKey)
    } else {
        await dismissDialog(page)
    }
    await backToInspector(page)
    return {recipe, sel, inlineVoting, inlineReview, dialog}
}

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
            const obs = await observeCell(inputs)
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
    "the label means this witness lane has not re-run them itself (IRV",
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
