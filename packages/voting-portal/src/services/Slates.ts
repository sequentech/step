// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EMobileCandidateLists,
    getBallotStyleSlates,
    IBallotStyle as IElectionDTO,
    ICandidate,
    IContest,
    ISlate,
    ISlateProblem,
    ISlatesConfig,
} from "@sequentech/ui-core"

export {getSlateName} from "@sequentech/ui-core"

/** The candidates of a slate in one contest of the ballot. */
export interface ISlateContest {
    contest: IContest
    candidates: Array<ICandidate>
}

/** A slate with its members looked up in the ballot, contests in ballot order. */
export interface IResolvedSlate {
    id: string
    name: Record<string, string>
    contests: Array<ISlateContest>
}

export interface IBallotSlates {
    mobileCandidateLists: EMobileCandidateLists
    slates: Array<IResolvedSlate>
    /** The contests at least one slate has candidates for, in ballot order. */
    contests: Array<IContest>
}

const resolveSlate = (slate: ISlate, contests: Array<IContest>): IResolvedSlate => ({
    id: slate.id,
    name: slate.name,
    contests: contests.flatMap((contest) => {
        const memberIds = slate.members[contest.id] ?? []
        const candidates = memberIds.flatMap(
            (id) => contest.candidates.find((candidate) => candidate.id === id) ?? []
        )
        return candidates.length > 0 ? [{contest, candidates}] : []
    }),
})

export const resolveSlates = (config: ISlatesConfig, contests: Array<IContest>): IBallotSlates => {
    const slates = config.slates
        .map((slate) => resolveSlate(slate, contests))
        .filter((slate) => slate.contests.length > 0)
    const covered = new Set(
        slates.flatMap((slate) => slate.contests.map(({contest}) => contest.id))
    )
    return {
        mobileCandidateLists: config.mobile_candidate_lists,
        slates,
        contests: contests.filter((contest) => covered.has(contest.id)),
    }
}

/**
 * The slates of a ballot, or null when its election has none. Throws the
 * problems of an invalid configuration, as `getBallotStyleSlates` does.
 */
export const resolveBallotStyleSlates = (ballotEml: IElectionDTO): IBallotSlates | null => {
    const config = getBallotStyleSlates(ballotEml)
    return config ? resolveSlates(config, ballotEml.contests) : null
}

const isSlateProblem = (value: unknown): value is ISlateProblem =>
    typeof value === "object" &&
    value !== null &&
    typeof (value as ISlateProblem).message === "string"

/** Why a ballot's slate configuration is refused, or undefined when it is sound. */
export const getSlateConfigurationProblem = (ballotEml: IElectionDTO): string | undefined => {
    try {
        getBallotStyleSlates(ballotEml)
        return undefined
    } catch (error) {
        const [first] = Array.isArray(error) ? error : [error]
        return isSlateProblem(first) ? first.message : String(first)
    }
}
