// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// THE CERTIFIED DOMAIN — defined once, here.
//
// `headless-sweep.mjs` compares production against the spec on every cell of
// this domain. Everything downstream leans on that: the analysis runners
// evaluate their properties over it, and the browser runners check that the
// cells they drive lie inside it, because that is what licenses comparing a
// DOM against a spec-derived prediction.
//
// It used to be written six times — enumerated in four runners and mirrored
// as a hand-written predicate in two more. That is a bad shape for this
// particular fact: if one copy drifts, a runner keeps claiming certification
// over cells the sweep never visited, and an evidence claim that is not true
// is the failure this apparatus exists to prevent.
//
// So the predicate is DERIVED from the enumeration's own dimensions rather
// than restated alongside them — `inCertifiedDomain` tests membership of the
// same lists `certifiedCells` draws from, and both defer to `cell.mjs` for
// what a bundled fixture can actually drive.

import {representable, rankedTriples, wellFormedRankedTriples} from "./cell.mjs"

export const POLICY_VALUES = {
    invalid: ["allowed", "warn", "warn-invalid-implicit-and-explicit", "not-allowed"],
    blank: ["allowed", "warn", "warn-only-in-review", "not-allowed"],
    over: [
        "allowed",
        "allowed-with-msg",
        "allowed-with-msg-and-alert",
        "not-allowed-with-msg-and-alert",
        "not-allowed-with-msg-and-disable",
    ],
    under: ["allowed", "warn", "warn-only-in-review", "warn-and-alert"],
    dup: ["allowed-warn-and-dialog", "not-allowed-warn-and-dialog"],
    gap: ["allowed-warn-and-dialog", "not-allowed-warn-and-dialog"],
}

/** min ∈ 0..3 × max ∈ 1..3 with min ≤ max. `max = 0` is the config-sanity
 *  scope boundary and stays out. */
export const BOUNDS = []
for (let min = 0; min <= 3; min++)
    for (let max = 1; max <= 3; max++) if (min <= max) BOUNDS.push([min, max])

/** The vote states the bundled fixtures can realize. Plurality states ride
 *  the Referendum contest; the preferential ones the IRV contest, which
 *  carries no marker candidate. `firstPreferences` — how many selections sit
 *  at rank 0 — is what the GATES count (quirk S6), so it is part of the
 *  state, not derived from it. */
export const STATES = []
for (let regulars = 0; regulars <= 2; regulars++)
    for (const blankMarker of [false, true])
        for (const explicitInvalid of [false, true])
            STATES.push({
                regulars,
                blankMarker,
                explicitInvalid,
                duplicateRanks: false,
                rankGaps: false,
                firstPreferences: regulars, // every plurality selection is at rank 0
            })
for (const triple of rankedTriples())
    for (const explicitInvalid of [false, true])
        STATES.push({...triple, blankMarker: false, explicitInvalid})
// Well-formed rankings — the ORDINARY ranked ballot, and the region the
// gate/checker count divergence hits hardest. They were unreachable until
// `cell.mjs` learned to recognise a ranked ballot by its first-preference
// count rather than only by a duplicate or a gap.
for (const triple of wellFormedRankedTriples())
    for (const explicitInvalid of [false, true])
        STATES.push({...triple, blankMarker: false, explicitInvalid})

/** Normalize a vote state so states written by different callers (rule
 *  definitions omit fields they do not vary) compare equal. */
const normalize = (vs) => ({
    regulars: vs.regulars ?? 0,
    blankMarker: Boolean(vs.blankMarker),
    explicitInvalid: Boolean(vs.explicitInvalid),
    duplicateRanks: Boolean(vs.duplicateRanks),
    rankGaps: Boolean(vs.rankGaps),
    firstPreferences: vs.firstPreferences ?? vs.regulars ?? 0,
})

const STATE_KEYS = new Set(STATES.map((s) => JSON.stringify(normalize(s))))

/** Every cell the sweep visits, in the sweep's own iteration order. */
export function certifiedCells() {
    const cells = []
    for (const invalid of POLICY_VALUES.invalid)
        for (const blank of POLICY_VALUES.blank)
            for (const over of POLICY_VALUES.over)
                for (const under of POLICY_VALUES.under)
                    for (const dup of POLICY_VALUES.dup)
                        for (const gap of POLICY_VALUES.gap) {
                            const policies = {invalid, blank, over, under, dup, gap}
                            for (const [min, max] of BOUNDS)
                                for (const state of STATES) {
                                    const cell = {
                                        config: {min, max, policies},
                                        voteState: {...state},
                                    }
                                    if (!representable(cell)) cells.push(cell)
                                }
                        }
    return cells
}

/**
 * Is this cell one the sweep certifies? Tests membership of the very lists
 * `certifiedCells` enumerates, so the two cannot drift apart.
 * @returns {null | string} null if inside, else why it is not
 */
export function inCertifiedDomain({config, voteState}) {
    const why = representable({config, voteState})
    if (why) return why
    for (const [knob, values] of Object.entries(POLICY_VALUES)) {
        const v = config.policies?.[knob]
        if (v !== undefined && !values.includes(v))
            return `policy ${knob}=${v} outside the swept values`
    }
    if (!BOUNDS.some(([mn, mx]) => mn === config.min && mx === config.max))
        return `bounds min=${config.min} max=${config.max} outside the swept bounds`
    if (!STATE_KEYS.has(JSON.stringify(normalize(voteState))))
        return `vote state outside the swept states (${JSON.stringify(normalize(voteState))})`
    return null
}

/** How the domain describes itself in recorded artifacts. Lives here so it
 *  cannot drift from the enumeration it describes — it did once: the sweep's
 *  recording claimed "no preferential state" for a day after ranked cells
 *  were added. */
export const DOMAIN_DESCRIPTION = {
    bounds: "min 0..3 × max 1..3, min ≤ max (max = 0 is the config-sanity scope boundary)",
    states:
        "plurality: regulars 0..2 × blank marker × explicit-invalid flag, on the " +
        "Referendum fixture. Preferential: every reachable (regulars, duplicate " +
        "ranks, rank gaps) triple × explicit-invalid flag, on the IRV fixture, " +
        "which carries no marker candidate — both malformed rankings (duplicate " +
        "or gap) and well-formed ones (regulars 2..3, one first preference). " +
        "No decline: the single-contest decode hardcodes it false.",
}
