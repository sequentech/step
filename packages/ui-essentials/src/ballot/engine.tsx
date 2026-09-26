// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The compiled calls a ballot cannot draw itself without, supplied by the host.
 *
 * Four of them, and every one is WebAssembly rather than TypeScript: the ordering
 * rule, the blank-vote test, the preferential predicate and the write-in character
 * budget. They are in the render path — `Question` cannot decide candidate order without one,
 * `Answer` cannot decide between an ordinal picker and a checkbox — so they are not
 * something a shared component can do without or approximate.
 *
 * **Why injected rather than imported.** The two hosts load *different builds of
 * the same Rust crate*. The voting portal has `sequent-core`, vendored as a tarball
 * beside `ui-core`. The Election Architect has `sequent-election-config`, built
 * from the same `packages/sequent-core` with a different feature set for the
 * configuration tools. Both export these functions — verified in the wizard's
 * `index.d.ts`: `sort_candidates_list_js`, `is_preferential_js`,
 * `check_is_blank_js`, `get_write_in_available_characters_js`,
 * `get_layout_properties_from_contest_js`. So if this package imported one of them,
 * the other host would end up loading two WebAssembly runtimes to draw one ballot —
 * about four megabytes of duplicate encoder, and two implementations of the rule
 * that decides what order candidates appear in.
 *
 * Taking them as an interface means each host supplies the build it already has,
 * and the ordering a voter sees comes from the same Rust either way.
 *
 * Four rather than six: `interpretContestSelection` and its multi-contest sibling
 * were imported by the warning list and never called — they arrived with a
 * fifteen-function service object that was destructured wholesale. An interface
 * member no caller uses is work for every host that implements it, so they are not
 * here. They can be added when something asks.
 *
 * The default throws rather than approximating. A JavaScript `sort` standing in for
 * `sort_candidates_list_js` would be a second opinion about candidate order, which
 * for a randomised-order contest is a fairness property and not a detail.
 */

import React, {createContext, PropsWithChildren, useContext} from "react"
import type {
    CandidatesOrder,
    ICandidate,
    ICountingAlgorithm,
    IDecodedVoteContest,
} from "@sequentech/ui-core"

import type {IBallotStyle} from "./types"

export interface BallotEngine {
    /**
     * Candidates in the order this contest presents them.
     *
     * `applyRandom` is what makes a random order actually random for this voter, so
     * it is passed through rather than defaulted here.
     */
    sortCandidatesInContest(
        candidates: Array<ICandidate>,
        order?: CandidatesOrder,
        applyRandom?: boolean
    ): Array<ICandidate>

    /** Whether the algorithm ranks, so the row draws ordinals rather than a box. */
    isPreferential(countingAlgorithm?: ICountingAlgorithm): boolean

    /** Whether these marks amount to a blank ballot. */
    checkIsBlank(contest: IDecodedVoteContest): boolean | null

    /** How many characters are left for write-ins on this contest. */
    getWriteInAvailableCharacters(
        contestSelection: IDecodedVoteContest,
        election: IBallotStyle["ballot_eml"]
    ): number
}

const refuse = (name: string) => (): never => {
    throw new Error(
        `No BallotEngine above this ballot, so ${name} cannot be answered. A host has to ` +
            `supply one from the build of sequent-core it already loads: the voting portal ` +
            `passes ui-core's wrappers, the Election Architect passes its ` +
            `sequent-election-config exports. There is deliberately no JavaScript fallback — ` +
            `an approximation of the ordering rule is a second opinion about what a voter sees.`
    )
}

const NONE: BallotEngine = {
    sortCandidatesInContest: refuse("sortCandidatesInContest"),
    isPreferential: refuse("isPreferential"),
    checkIsBlank: refuse("checkIsBlank"),
    getWriteInAvailableCharacters: refuse("getWriteInAvailableCharacters"),
}

const BallotEngineContext = createContext<BallotEngine>(NONE)

export const BallotEngineProvider = ({
    engine,
    children,
}: PropsWithChildren<{engine: BallotEngine}>): React.JSX.Element => (
    <BallotEngineContext.Provider value={engine}>{children}</BallotEngineContext.Provider>
)

/** The host's build of the core, from inside the ballot. */
export const useBallotEngine = (): BallotEngine => useContext(BallotEngineContext)
