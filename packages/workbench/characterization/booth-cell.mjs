// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shared generic booth driver for the spec-domain browser tools
// (`browser-witnesses.mjs`, `quotient-validate.mjs`): drives an ARBITRARY
// plurality cell of the spec domain through the real booth and observes
// inline (touched voting + review), the selection state, and the dialog.
// The per-rule tools keep their own recipes (rule-specs.mjs); this driver
// exists for cells that belong to no grid.
//
//   - contest by vote state: the explicit-invalid flag needs the Council
//     contest (its null-vote marker is the only booth route to the flag),
//     the blank marker needs Referendum; a cell needing BOTH has no booth
//     contest on this fixture.
//   - the NON-target contest is neutralized every cell (platform defaults
//     + fixture bounds): the review screen renders every contest and
//     warn-id reads are page-wide, so a stale override there pollutes the
//     observation (the CLAUDE.md gotcha).
//   - deterministic touch-arming (tick-untick of the landmark), then the
//     state clicks: regulars first, marker last, so a marker-clear is an
//     observed collapse.

import {
    warnIds,
    dialogKind,
    setPanelConfig,
    enterBooth,
    clearSelections,
    dismissDialog,
    backToInspector,
} from "./browser-harness.mjs"

export const shortKey = (k) => k.split(".").pop()

/** Read the rendered warn-ids only once they are QUIESCENT: two consecutive
 *  reads 150 ms apart must agree (capped at ~3 s). The warnings render from
 *  component-local decode state that lags the last click; an immediate read
 *  races it (three quotient "disagreements" were exactly this artifact — a
 *  400 ms-settled probe showed the DOM matching the spec). Quiescence is
 *  honest where a fixed sleep is not: a real divergence stays divergent. */
async function stableWarnIds(page) {
    let prev = await warnIds(page)
    for (let i = 0; i < 20; i++) {
        await page.waitForTimeout(150)
        const next = await warnIds(page)
        if (JSON.stringify(next) === JSON.stringify(prev)) return next
        prev = next
    }
    return prev
}

const NEUTRAL = {
    selects: {
        "Invalid-vote policy": "allowed",
        "Blank-vote policy": "allowed",
        "Over-vote policy": "allowed-with-msg-and-alert",
        "Under-vote policy": "allowed",
    },
}
const FIXTURE_BOUNDS = {
    referendum: {min_votes: 0, max_votes: 2},
    council: {min_votes: 0, max_votes: 1},
}

/** Resolve the two contests + voter once per page load. */
export async function boothContext(page, electionId) {
    const ids = await page.evaluate((eid) => {
        const bs = window.__store.getState().ballotStyles[eid]
        const byFlag = (flag) =>
            bs.ballot_eml.contests.find((x) =>
                x.candidates.some((cd) => cd.presentation?.[flag])
            )
        const raw = localStorage.getItem("workbench:state:v1")
        return {
            referendum: byFlag("is_explicit_blank").id,
            council: byFlag("is_explicit_invalid").id,
            voter: raw ? JSON.parse(raw)?.workbench?.voters?.[0]?.id ?? null : null,
        }
    }, electionId)
    return {
        electionId,
        ids,
        recipes: {
            referendum: {
                contestId: ids.referendum,
                landmark: /^Yes$/,
                regulars: [/^Yes$/, /^No$/],
            },
            council: {
                contestId: ids.council,
                landmark: /^Ada$/,
                regulars: [/^Ada$/, /^Bruno$/],
            },
        },
    }
}

/** Can the booth form this cell's vote state at all? Returns null if yes,
 *  else the label. (Beyond plurality-representability: the booth cannot
 *  set the flag alongside the blank marker, and prevention collapses or
 *  blocks some states — reachability is the caller's concern via spec.) */
export function boothFormable({voteState: vs}) {
    if (vs.duplicateRanks || vs.rankGaps) return "preferential state (IRV recipe pending)"
    if (vs.decline) return "decline (no booth route)"
    if (vs.regulars > 2) return "regulars > 2 (no fixture)"
    if (vs.blankMarker && vs.explicitInvalid)
        return "blank marker + invalid flag (no booth contest carries both)"
    return null
}

/** Drive one cell; returns {sel, inlineVoting, inlineReview, dialog}.
 *  inlineReview is null when a blocking dialog precludes review. */
export async function observeCell(page, ctx, {config, voteState: vs}) {
    const target = vs.explicitInvalid ? "council" : "referendum"
    const other = vs.explicitInvalid ? "referendum" : "council"
    const recipe = ctx.recipes[target]
    await setPanelConfig(page, ctx.recipes[other].contestId, {
        ...NEUTRAL,
        bounds: FIXTURE_BOUNDS[other],
    })
    await setPanelConfig(page, recipe.contestId, {
        selects: {
            "Invalid-vote policy": config.policies.invalid,
            "Blank-vote policy": config.policies.blank,
            "Over-vote policy": config.policies.over,
            "Under-vote policy": config.policies.under,
        },
        bounds: {min_votes: config.min, max_votes: config.max},
    })
    await enterBooth(page, ctx.ids.voter)
    await page.getByText(recipe.landmark).first().waitFor({timeout: 15000})
    await clearSelections(page)
    const clickText = (rx) => page.getByText(rx).first().click().catch(() => {})
    const clickExact = (s) =>
        page.getByText(s, {exact: true}).first().click().catch(() => {})
    // Deterministic touch (the untouched-clear must never mask the voting
    // observation), then the state: regulars first, marker last.
    await clickText(recipe.landmark)
    await clickText(recipe.landmark)
    for (let i = 0; i < vs.regulars; i++) await clickText(recipe.regulars[i])
    if (vs.blankMarker) await clickExact("Blank vote (explicit blank)")
    if (vs.explicitInvalid) await clickExact("Null vote (explicit invalid)")

    const sel = await page.evaluate(
        ({eid, cid}) => {
            const s = window.__store.getState().ballotSelections[eid] ?? []
            const c = s.find((x) => x.contest_id === cid)
            return {
                formed: c ? c.choices.filter((ch) => ch.selected === 0).length : 0,
                flag: !!(c && c.is_explicit_invalid),
                selected: c ? c.choices.map((ch) => ch.selected) : [],
            }
        },
        {eid: ctx.electionId, cid: recipe.contestId}
    )
    const inlineVoting = (await stableWarnIds(page)).map(shortKey)

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
        inlineReview = (await stableWarnIds(page)).map(shortKey)
    } else {
        await dismissDialog(page)
    }
    await backToInspector(page)
    return {sel, inlineVoting, inlineReview, dialog}
}
