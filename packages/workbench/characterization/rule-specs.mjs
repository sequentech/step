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
    setRank,
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
            bounds: {min_votes: 0, max_votes: 1},
        }),
        select: async (page, c) => {
            if (c.state === "at_max") await clickText(page, /^Ada$/)
            else if (c.state === "over_max") {
                await clickText(page, /^Ada$/)
                await clickText(page, /^Bruno$/)
            }
        },
        want: (c) => (c.state === "over_max" ? 2 : c.state === "at_max" ? 1 : 0),
        // Direct DOM evidence for the input constraint: under DISABLE, once the
        // contest is at max the (max+1)th control is disabled, so `over_max`
        // cannot form. Probe the second Council candidate's checkbox `disabled`
        // attribute — a second signal beyond behavioural non-reachability.
        // null = not applicable (only the disable policy at `over_max` applies).
        probeDisabled: async (page, c) =>
            c.over_vote_policy === "not-allowed-with-msg-and-disable" && c.state === "over_max"
                ? page.locator('input.candidate-input[aria-label="Bruno"]').isDisabled().catch(() => null)
                : null,
    },
    "minvote-rule": {
        contestFlag: "is_explicit_blank", // Referendum (Yes / No / blank marker)
        landmark: /^Yes$/,
        config: (c) => ({
            selects: {"Invalid-vote policy": c.invalid_vote_policy},
            bounds: {min_votes: c.min_votes, max_votes: 3},
        }),
        select: async (page, c) => {
            if (c.state === "one") await clickText(page, /^Yes$/)
            else if (c.state === "marker_only")
                await clickExact(page, "Blank vote (explicit blank)")
        },
        want: (c) => (c.state === "none" ? 0 : 1),
    },
    "blank-rule": {
        contestFlag: "is_explicit_blank", // Referendum (Yes / No / blank marker)
        landmark: /^Yes$/,
        config: (c) => ({
            selects: {
                "Blank-vote policy": c.blank_vote_policy,
                "Invalid-vote policy": c.invalid_vote_policy,
            },
            bounds: {min_votes: 0, max_votes: 2},
        }),
        select: async (page, c) => {
            if (c.state === "one_regular") await clickText(page, /^Yes$/)
            else if (c.state === "marker_only")
                await clickExact(page, "Blank vote (explicit blank)")
            else if (c.state === "regular_then_marker") {
                // marker exclusivity: a regular FIRST, then the blank marker,
                // which must CLEAR the regular (the mirror of invalid
                // marker_plus, which does not clear — S5).
                await clickText(page, /^Yes$/)
                await clickExact(page, "Blank vote (explicit blank)")
            }
        },
        // `regular_then_marker` wants the uncleared mixed state (2); observing
        // only 1 (the marker) is the clearing, expected by `marker_cleared`.
        want: (c) =>
            c.state === "empty" ? 0 : c.state === "regular_then_marker" ? 2 : 1,
    },
    "undervote-rule": {
        contestFlag: "is_explicit_blank", // Referendum (Yes / No / blank marker)
        landmark: /^Yes$/,
        config: (c) => ({
            selects: {
                "Under-vote policy": c.under_vote_policy,
                "Invalid-vote policy": c.invalid_vote_policy,
            },
            bounds: {min_votes: 0, max_votes: 2},
        }),
        select: async (page, c) => {
            if (c.state === "under") await clickText(page, /^Yes$/)
            else if (c.state === "full") {
                await clickText(page, /^Yes$/)
                await clickText(page, /^No$/)
            }
        },
        want: (c) => (c.state === "empty" ? 0 : c.state === "under" ? 1 : 2),
    },
    "invalid-rule": {
        contestFlag: "is_explicit_invalid", // Council (Ada / Bruno / null marker)
        landmark: /^Ada$/,
        config: (c) => ({
            selects: {"Invalid-vote policy": c.invalid_vote_policy},
            bounds: {min_votes: 0, max_votes: 2},
        }),
        select: async (page, c) => {
            if (c.state === "regular") await clickText(page, /^Ada$/)
            else if (c.state === "marker")
                await clickExact(page, "Null vote (explicit invalid)")
            else if (c.state === "marker_plus") {
                await clickExact(page, "Null vote (explicit invalid)")
                await clickText(page, /^Ada$/)
            }
        },
        // The invalid marker sets the `is_explicit_invalid` FLAG, not a counted
        // selection (unlike the blank marker), so reachability checks the flag
        // AND the regular-candidate count — `want` alone (selectionCount) can't
        // tell `marker` (flag, 0 regulars) from `none` (no flag, 0 regulars).
        want: (c) => (c.state === "regular" || c.state === "marker_plus" ? 1 : 0),
        reached: (obs, c) =>
            obs.formed === (c.state === "regular" || c.state === "marker_plus" ? 1 : 0) &&
            obs.explicitInvalid === (c.state === "marker" || c.state === "marker_plus"),
    },
    "duprank-rule": {
        contestCounting: "instant-runoff", // IRV Favourite fruit (Apple/Banana/Cherry)
        landmark: /^Apple$/,
        // ranked selection (`selected` = rank; -1 unranked). valid_full = a
        // well-ordered 0,1,2; duplicate = two candidates sharing rank 0.
        ranks: (c) => (c.state === "valid_full" ? [0, 1, 2] : [0, 0, -1]),
        config: (c) => ({
            selects: {
                "Duplicated-rank policy": c.duplicated_rank_policy,
                "Invalid-vote policy": c.invalid_vote_policy,
            },
        }),
        select: rankedSelect,
        // Reachability is the whole rank vector, not a count: valid_full and a
        // gap/duplicate can share a rank-1 count, so compare the vector.
        reached: (obs, c, spec) =>
            JSON.stringify(obs.selected) === JSON.stringify(spec.ranks(c)),
    },
    "prefgaps-rule": {
        contestCounting: "instant-runoff", // IRV Favourite fruit (Apple/Banana/Cherry)
        landmark: /^Apple$/,
        // valid_full = 0,1,2; gap = ranks 0 then 2, skipping rank 1.
        ranks: (c) => (c.state === "valid_full" ? [0, 1, 2] : [0, 2, -1]),
        config: (c) => ({
            selects: {
                "Preference-gaps policy": c.preference_gaps_policy,
                "Invalid-vote policy": c.invalid_vote_policy,
            },
        }),
        select: rankedSelect,
        reached: (obs, c, spec) =>
            JSON.stringify(obs.selected) === JSON.stringify(spec.ranks(c)),
    },
}

/** Selection driver for preferential rules: set every candidate's rank
 *  explicitly (including to "none") so each cell fully specifies the ranking
 *  and nothing bleeds from the previous cell. `position` = rank + 1, 0 = none. */
async function rankedSelect(page, c, spec) {
    const r = spec.ranks(c)
    for (let i = 0; i < r.length; i++) {
        await setRank(page, i, r[i] < 0 ? 0 : r[i] + 1)
    }
}

/** Resolve a rule's contest id and the first voter id. The contest is found by
 *  a candidate marker `flag` (plurality rules) or by `counting` algorithm
 *  (preferential rules, which carry no marker). */
export async function contestAndVoter(page, electionId, {flag, counting}) {
    return page.evaluate(
        ({electionId, flag, counting}) => {
            const bs = window.__store.getState().ballotStyles[electionId]
            const c = bs.ballot_eml.contests.find((x) =>
                counting
                    ? x.counting_algorithm === counting
                    : x.candidates.some((cd) => cd.presentation?.[flag])
            )
            const raw = localStorage.getItem("workbench:state:v1")
            const voter = raw
                ? JSON.parse(raw)?.workbench?.voters?.[0]?.id ?? null
                : null
            return {contestId: c.id, voterId: voter}
        },
        {electionId, flag, counting}
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
    await spec.select(page, cell, spec)

    const formed = await selectionCount(page, electionId, contestId)
    const {explicitInvalid, selected} = await page.evaluate(
        ({electionId, contestId}) => {
            const sel = window.__store.getState().ballotSelections[electionId] ?? []
            const c = sel.find((x) => x.contest_id === contestId)
            return {
                explicitInvalid: !!(c && c.is_explicit_invalid),
                selected: c ? c.choices.map((ch) => ch.selected) : [],
            }
        },
        {electionId, contestId}
    )
    const inlineAtVote = await warnIds(page)
    // Optional direct DOM probe on the voting screen (before Next) — e.g. the
    // over-vote DISABLE policy disables the (max+1)th control. null = the spec
    // has no probe, or it does not apply to this cell.
    const constraintProbe = spec.probeDisabled ? await spec.probeDisabled(page, cell) : null

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
    return {formed, selected, explicitInvalid, constraintProbe, inlineAtVote, dialog, inlineAtReview}
}

/** Did the intended cell state form? Default: the marker-inclusive selection
 *  count matches `want`. A spec may override with `reached(obs, cell, spec)`
 *  when the count alone is insufficient — e.g. the invalid marker sets the
 *  `is_explicit_invalid` FLAG, not a counted selection (`invalid-rule`), or a
 *  preferential rule's reachability is the whole rank vector (`duprank`). */
export const isReached = (spec, obs, cell) =>
    spec.reached ? spec.reached(obs, cell, spec) : obs.formed === spec.want(cell)
