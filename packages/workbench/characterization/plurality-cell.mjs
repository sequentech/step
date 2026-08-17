// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shared headless cell machinery for the spec-domain tools
// (`effect-dependencies.mjs` lane B, `headless-sweep.mjs`): builds a
// production EML + selection for any plurality-representable cell of the
// spec's input domain, and observes it through the real WASM
// (checker → gates → tally).
//
// Everything runs on the explicit-blank-invalid fixture's Referendum
// contest — it carries a blank marker, accepts the explicit-invalid flag
// without a marker candidate (recorded: blank-rule.md, explicit_invalid
// state), and has two regular candidates, so it represents every plurality
// cell with regulars ≤ 2. Cells with preferential state, decline, more
// regulars, or max_votes = 0 (the config-sanity scope boundary) are NOT
// representable here — callers must filter/label those; see
// `representable()`.

import {readFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import path from "node:path"
import {runChecker, runGates, tallyClass, extractErrors} from "./harness.mjs"

const here = path.dirname(fileURLToPath(import.meta.url))

const snap = JSON.parse(
    readFileSync(
        path.resolve(here, "../app/src/fixtures/snapshots/explicit-blank-invalid.json"),
        "utf8"
    )
)
const eml = Object.values(snap.state.ballotStyles)[0].ballot_eml
export const contest = eml.contests.find((c) =>
    c.candidates.some((x) => x.presentation?.is_explicit_blank)
)
const markerId = contest.candidates.find((x) => x.presentation?.is_explicit_blank).id
export const regularIds = contest.candidates
    .filter((x) => !x.presentation?.is_explicit_blank)
    .map((x) => x.id)

/** Can this (config, voteState) cell be driven through the fixture?
 *  Returns null if yes, else the reason label the caller should record. */
export function representable({config, voteState}) {
    if (voteState.duplicateRanks || voteState.rankGaps)
        return "preferential state (IRV recipe pending)"
    if (voteState.decline) return "decline (classifier-direct pending)"
    if (voteState.regulars > regularIds.length)
        return `regulars > ${regularIds.length} (no fixture)`
    if (config.max === 0) return "max_votes = 0 (config-sanity scope boundary)"
    return null
}

export function makeEml(config) {
    const clone = structuredClone(eml)
    const c = clone.contests.find((x) => x.id === contest.id)
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

export function makeSelection(vs) {
    const picked = regularIds.slice(0, vs.regulars)
    return {
        contest_id: contest.id,
        is_explicit_invalid: vs.explicitInvalid,
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
    const cellEml = makeEml(cellInputs.config)
    const cellContest = cellEml.contests.find((x) => x.id === contest.id)
    const decoded = runChecker(makeSelection(cellInputs.voteState), cellEml)
    const {errors, alerts} = extractErrors(decoded)
    const gates = runGates([cellContest], {[contest.id]: decoded})
    return {
        errors: errors.map(shortKey),
        alerts: alerts.map(shortKey),
        hard: gates.hard,
        soft: gates.soft,
        tally: tallyClass(cellContest, decoded),
    }
}
