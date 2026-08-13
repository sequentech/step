// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Per-rule browser-driving specs + the shared drive-and-observe flow, used by
// the browser tools that need to reach a specific (config × state) cell in the
// booth: `dom-validate` (validate spec predictions vs the DOM) and
// `no-silent-discount` (confirm candidates). Single-sources "how to configure
// and form each cell's state" so a contest/candidate/label change is fixed
// once, not per tool — the browser-side echo of `spec.mjs`.
//
// Keyed by the rule name used in the recorded JSONs ("overvote-rule", …), so a
// candidate row `{rule, ...cell}` maps straight to its spec.

import {
    warnIds,
    dialogKind,
    setPanelConfig,
    enterBooth,
    clearSelections,
    dismissDialog,
    backToInspector,
    selectionCount,
} from "./browser-harness.mjs"

const clickText = (page, rx) => page.getByText(rx).first().click().catch(() => {})
const clickExact = (page, s) =>
    page.getByText(s, {exact: true}).first().click().catch(() => {})

export const RULE_SPECS = {
    "overvote-rule": {
        contestFlag: "is_explicit_invalid", // Council seat (Ada / Bruno / null)
        landmark: /^Ada$/,
        config: (c) => ({
            selects: {
                "Over-vote policy": c.over_vote_policy,
                "Invalid-vote policy": c.invalid_vote_policy,
            },
        }),
        select: async (page, c) => {
            if (c.state === "at_max") await clickText(page, /^Ada$/)
            else if (c.state === "over_max") {
                await clickText(page, /^Ada$/)
                await clickText(page, /^Bruno$/)
            }
        },
        want: (c) => (c.state === "over_max" ? 2 : c.state === "at_max" ? 1 : 0),
    },
    "minvote-rule": {
        contestFlag: "is_explicit_blank", // Referendum (Yes / No / blank marker)
        landmark: /^Yes$/,
        config: (c) => ({
            selects: {"Invalid-vote policy": c.invalid_vote_policy},
            bounds: {min_votes: c.min_votes},
        }),
        select: async (page, c) => {
            if (c.state === "one") await clickText(page, /^Yes$/)
            else if (c.state === "marker_only")
                await clickExact(page, "Blank vote (explicit blank)")
        },
        want: (c) => (c.state === "none" ? 0 : 1),
    },
}

/** Resolve a rule's contest id (by its marker flag) and the first voter id. */
export async function contestAndVoter(page, electionId, contestFlag) {
    return page.evaluate(
        ({electionId, contestFlag}) => {
            const bs = window.__store.getState().ballotStyles[electionId]
            const c = bs.ballot_eml.contests.find((x) =>
                x.candidates.some((cd) => cd.presentation?.[contestFlag])
            )
            const raw = localStorage.getItem("workbench:state:v1")
            const voter = raw
                ? JSON.parse(raw)?.workbench?.voters?.[0]?.id ?? null
                : null
            return {contestId: c.id, voterId: voter}
        },
        {electionId, contestFlag}
    )
}

/**
 * Drive one cell through the booth reload-free (panel config → form state →
 * Next) and observe every surface:
 *   - `formed`         — selection count (reachability: did the state form?)
 *   - `inlineAtVote`   — inline warnings on the voting screen
 *   - `dialog`         — the Next-dialog kind ("none"|"dismissible"|"blocking")
 *   - `inlineAtReview` — inline warnings on the REVIEW screen, the decisive
 *     last surface before cast (the untouched-clear does not apply there);
 *     null when a gate blocks the path to review.
 * Does NOT cast. Returns to the inspector, ready for the next cell.
 */
export async function observeBooth(page, {electionId, contestId, voterId, spec, cell}) {
    await setPanelConfig(page, contestId, spec.config(cell))
    await enterBooth(page, voterId)
    await page.getByText(spec.landmark).first().waitFor({timeout: 15000})
    await clearSelections(page)
    await spec.select(page, cell)

    const formed = await selectionCount(page, electionId, contestId)
    const inlineAtVote = await warnIds(page)

    let dialog = "none"
    let inlineAtReview = null
    const next = page.getByRole("button", {name: /next|review/i}).first()
    if (await next.count().catch(() => 0)) {
        await next.click().catch(() => {})
        dialog = await dialogKind(page)
    }
    // A dismissible dialog is a signal but does not block: continue through it
    // to reach review. A blocking dialog cannot be passed — review is
    // unreachable, so `inlineAtReview` stays null (the dialog is the signal).
    if (dialog === "dismissible") {
        await page.getByRole("button", {name: /continue/i}).first().click().catch(() => {})
    }
    if (dialog !== "blocking") {
        await page
            .locator(".cast-ballot-button")
            .first()
            .waitFor({timeout: 8000})
            .catch(() => {})
        inlineAtReview = await warnIds(page)
    } else {
        await dismissDialog(page)
    }
    await backToInspector(page)
    return {formed, inlineAtVote, dialog, inlineAtReview}
}
