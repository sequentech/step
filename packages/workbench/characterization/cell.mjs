// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shared headless cell machinery — builds a production EML + selection for
// any representable cell of the spec's input domain and observes it through
// the real WASM (checker → gates → tally). Used by `headless-sweep.mjs` and
// `effect-dependencies.mjs`.
//
// TWO FIXTURES, chosen by the cell, not by the caller:
//
//   plurality — the `explicit-blank-invalid` fixture's Referendum contest.
//     Carries a blank marker, accepts the explicit-invalid flag without a
//     marker candidate (recorded: blank-rule.md, explicit_invalid state),
//     and has two regular candidates.
//
//   preferential — the `instant-runoff-3cand` fixture's IRV contest, for
//     cells with duplicate ranks or rank gaps. Three plain candidates and
//     NO markers, so a preferential cell cannot also carry a blank marker.
//
// The routing rule is deliberately narrow: preferential **iff** the cell
// has `duplicateRanks` or `rankGaps`. Every cell that was representable
// before this module gained ranked support still runs on the same contest
// it always did, so its recorded observation is unchanged — which is what
// lets the restructure use byte-identical artifacts as its check
// (docs/EVIDENCE_RESTRUCTURE.md, "The oracle").
//
// Still NOT representable: decline (the single-contest decode hardcodes
// `is_decline_to_vote: false`), `max_votes = 0` (the config-sanity scope
// boundary), more regulars than the chosen fixture carries, and a blank
// marker on a preferential cell. `representable()` names each.

import {readFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {runChecker, runGates, tallyClass, extractErrors} from "./harness.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const load = (name) =>
    JSON.parse(
        readFileSync(path.resolve(here, `../app/src/fixtures/snapshots/${name}.json`), "utf8")
    )

const emlOf = (snap) => Object.values(snap.state.ballotStyles)[0].ballot_eml

// --- plurality fixture -----------------------------------------------------
const plurEml = emlOf(load("explicit-blank-invalid"))
export const contest = plurEml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const markerId = contest.candidates.find((x) => x.presentation?.is_explicit_blank).id
export const regularIds = contest.candidates
    .filter((x) => !x.presentation?.is_explicit_blank)
    .map((x) => x.id)

// --- preferential fixture --------------------------------------------------
const prefEml = emlOf(load("instant-runoff-3cand"))
const prefContest = prefEml.contests[0]
const prefIds = prefContest.candidates.map((c) => c.id)

/** Is this cell's vote state preferential? Decides the fixture. */
const isPreferential = (vs) => Boolean(vs.duplicateRanks || vs.rankGaps)

// ---------------------------------------------------------------------------
// Ranked-state realization
//
// `plaintext.rs::validate_preferencial_order` decides both flags from the
// SELECTED ranks: duplicates from the multiset, gaps by comparing the sorted
// UNIQUE ranks against 0..n-1. Note what that means — [0,0,1] is a duplicate
// but NOT a gap, because deduplication happens before the gap test.
//
// Rather than hand-pick rank arrays per case, enumerate small assignments,
// derive each one's triple by the same rule, and keep one representative per
// triple. A representative is therefore correct by construction, and a
// triple with no representative is genuinely unreachable (nothing can have
// duplicates with fewer than two selections).
//
// A SECOND constraint is the codec, not the checker. A ranked assignment
// survives encode -> decode exactly when every rank is below `max_votes`;
// probed 2026-08-18 over the assignments below at max_votes 1..3, where the
// rule held without exception. So representatives are chosen to minimise the
// top rank, and `representable()` rejects a ranked cell whose top rank the
// cell's own `max_votes` cannot carry. One triple is lost to this on the
// bundled fixture: (regulars 3, no duplicate, gap) needs three distinct
// ranks other than {0,1,2}, hence a rank of 3, hence `max_votes >= 4` --
// beyond both the fixture's three candidates and the sweep's bounds.
// ---------------------------------------------------------------------------

/** The triple production would derive from a rank assignment. */
function orderTriple(ranks) {
    const chosen = ranks.filter((r) => r >= 0)
    const uniq = [...new Set(chosen)]
    const sorted = [...uniq].sort((a, b) => a - b)
    return {
        regulars: chosen.length,
        duplicateRanks: chosen.length !== uniq.length,
        rankGaps: sorted.some((v, i) => v !== i),
    }
}

const key = (regulars, dup, gap) => `${regulars}|${dup ? 1 : 0}|${gap ? 1 : 0}`

const RANKED = new Map()
{
    const VALUES = [-1, 0, 1, 2, 3]
    for (const a of VALUES)
        for (const b of VALUES)
            for (const c of VALUES) {
                const ranks = [a, b, c]
                const t = orderTriple(ranks)
                const k = key(t.regulars, t.duplicateRanks, t.rankGaps)
                const maxRank = Math.max(...ranks)
                const prev = RANKED.get(k)
                // Keep the assignment with the SMALLEST top rank: the codec
                // round trip needs every rank below max_votes, so a lower top
                // rank stays representable across more of the bounds domain.
                if (!prev || maxRank < prev.maxRank) RANKED.set(k, {ranks, maxRank})
            }
}

/** The reachable preferential (regulars, duplicateRanks, rankGaps) triples —
 *  derived from the enumeration above, not hand-listed, so callers cannot
 *  drift from what the fixture can actually express. */
export function rankedTriples() {
    return [...RANKED.keys()]
        .map((k) => {
            const [regulars, dup, gap] = k.split("|")
            const {ranks} = RANKED.get(k)
            return {
                regulars: Number(regulars),
                duplicateRanks: dup === "1",
                rankGaps: gap === "1",
                // How many of this state's selections sit at rank 0. The
                // GATES count only those (`voting_screen.rs`), so the spec
                // needs it as an input of its own — quirk
                // S6_GATES_COUNT_FIRST_PREFERENCES_ONLY.
                firstPreferences: ranks.filter((r) => r === 0).length,
            }
        })
        .filter((t) => t.duplicateRanks || t.rankGaps)
}

/** Can this (config, voteState) cell be driven through a bundled fixture?
 *  Returns null if yes, else the reason label the caller should record. */
export function representable({config, voteState}) {
    if (voteState.decline) return "decline (classifier-direct pending)"
    if (config.max === 0) return "max_votes = 0 (config-sanity scope boundary)"
    if (isPreferential(voteState)) {
        if (voteState.blankMarker)
            return "blank marker + preferential state (no fixture carries both)"
        const k = key(voteState.regulars, voteState.duplicateRanks, voteState.rankGaps)
        const r = RANKED.get(k)
        if (!r)
            return `unreachable ranked state (regulars=${voteState.regulars}, dup=${voteState.duplicateRanks}, gap=${voteState.rankGaps})`
        if (r.maxRank >= config.max)
            return `ranked state needs max_votes > ${r.maxRank} (codec round trip)`
        return null
    }
    if (voteState.regulars > regularIds.length)
        return `regulars > ${regularIds.length} (no fixture)`
    return null
}

function applyConfig(clone, contestId, config) {
    const c = clone.contests.find((x) => x.id === contestId)
    c.min_votes = config.min
    c.max_votes = config.max
    c.presentation = {
        ...(c.presentation ?? {}),
        invalid_vote_policy: config.policies.invalid,
        blank_vote_policy: config.policies.blank,
        over_vote_policy: config.policies.over,
        under_vote_policy: config.policies.under,
        duplicated_rank_policy: config.policies.dup,
        preference_gaps_policy: config.policies.gap,
    }
    return clone
}

export function makeEml(config, vs) {
    const pref = isPreferential(vs)
    const src = pref ? prefEml : plurEml
    const id = pref ? prefContest.id : contest.id
    return applyConfig(structuredClone(src), id, config)
}

export function makeSelection(vs) {
    if (isPreferential(vs)) {
        const {ranks} = RANKED.get(key(vs.regulars, vs.duplicateRanks, vs.rankGaps))
        return {
            contest_id: prefContest.id,
            is_explicit_invalid: Boolean(vs.explicitInvalid),
            is_decline_to_vote: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: prefIds.map((id, i) => ({
                id,
                selected: ranks[i],
                write_in_text: null,
            })),
        }
    }
    const picked = regularIds.slice(0, vs.regulars)
    return {
        contest_id: contest.id,
        is_explicit_invalid: Boolean(vs.explicitInvalid),
        is_decline_to_vote: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((c) => ({
            id: c.id,
            selected:
                picked.includes(c.id) || (vs.blankMarker && c.id === markerId) ? 0 : -1,
            write_in_text: null,
        })),
    }
}

export const shortKey = (k) => k.split(".").pop()

/** Drive one cell through the real WASM and return the headless
 *  observations (message keys shortened to their last segment, matching
 *  the tables). */
export function observeHeadless(cellInputs) {
    const vs = cellInputs.voteState
    const cellEml = makeEml(cellInputs.config, vs)
    const contestId = isPreferential(vs) ? prefContest.id : contest.id
    const cellContest = cellEml.contests.find((x) => x.id === contestId)
    const decoded = runChecker(makeSelection(vs), cellEml)
    const {errors, alerts} = extractErrors(decoded)
    const gates = runGates([cellContest], {[contestId]: decoded})
    return {
        errors: errors.map(shortKey),
        alerts: alerts.map(shortKey),
        hard: gates.hard,
        soft: gates.soft,
        tally: tallyClass(cellContest, decoded),
    }
}
