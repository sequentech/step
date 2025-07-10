// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SequentCoreLibInit, {
    IContestLayoutProperties,
    IDecodedVoteChoice,
    generate_sample_auditable_ballot_js,
    get_candidate_points_js,
    get_layout_properties_from_contest_js,
    set_hooks,
} from "sequent-core"
import {
    sort_elections_list_js,
    sort_contests_list_js,
    sort_candidates_list_js,
    decode_auditable_ballot_js,
    decode_auditable_multi_ballot_js,
    to_hashable_ballot_js,
    to_hashable_multi_ballot_js,
    hash_auditable_ballot_js,
    hash_auditable_multi_ballot_js,
    encrypt_decoded_contest_js,
    encrypt_decoded_multi_contest_js,
    test_contest_reencoding_js,
    test_multi_contest_reencoding_js,
    get_write_in_available_characters_js,
    check_is_blank_js,
    IDecodedVoteContest,
    check_voting_not_allowed_next,
    check_voting_error_dialog,
} from "sequent-core"
import {
    CandidatesOrder,
    ContestsOrder,
    ElectionsOrder,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    IBallotStyle,
    ICandidate,
    IContest,
    IElection,
    IHashableSingleBallot,
    IHashableMultiBallot,
} from ".."

export type {
    IPermission,
    IRole,
    IUser,
    IDecodedVoteContest,
    IDecodedVoteChoice,
    IInvalidPlaintextError,
    IContestLayoutProperties,
} from "sequent-core"

export type BallotSelection = Array<IDecodedVoteContest>

export const initCore = async () => {
    try {
        // SequentCoreLibInit().then(set_hooks)
        const wasmModule = await SequentCoreLibInit()
        set_hooks()
        return wasmModule
    } catch (error) {
        console.error("Error initializing SequentCoreLib:", error)
        throw error
    }
}

export const sortElectionList = (
    elections: Array<IElection>,
    order?: ElectionsOrder,
    applyRandom?: boolean
): Array<IElection> => {
    try {
        if (!elections || !elections.length) return elections
        return sort_elections_list_js(elections, order, applyRandom)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const sortContestList = (
    contests: Array<IContest>,
    order?: ContestsOrder,
    applyRandom?: boolean
): Array<IContest> => {
    try {
        if (!contests || !contests.length) return contests
        return sort_contests_list_js(contests, order, applyRandom)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const sortCandidatesInContest = (
    candidates: Array<ICandidate>,
    order?: CandidatesOrder,
    applyRandom?: boolean
): Array<ICandidate> => {
    try {
        if (!candidates || !candidates.length) return candidates
        return sort_candidates_list_js(candidates, order, applyRandom)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const toHashableBallot = (
    auditableBallot: IAuditableSingleBallot
): IHashableSingleBallot => {
    try {
        return to_hashable_ballot_js(auditableBallot)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const toHashableMultiBallot = (
    auditableMultiBallot: IAuditableMultiBallot
): IHashableMultiBallot => {
    try {
        return to_hashable_multi_ballot_js(auditableMultiBallot)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const hashBallot = (auditableBallot: IAuditableSingleBallot): string => {
    try {
        return hash_auditable_ballot_js(auditableBallot)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const hashMultiBallot = (auditableMultiBallot: IAuditableMultiBallot): string => {
    try {
        return hash_auditable_multi_ballot_js(auditableMultiBallot)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const encryptBallotSelection = (
    ballotSelection: BallotSelection,
    election: IBallotStyle
): IAuditableSingleBallot => {
    try {
        return encrypt_decoded_contest_js(ballotSelection, election)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const encryptMultiBallotSelection = (
    ballotSelection: BallotSelection,
    election: IBallotStyle
): IAuditableMultiBallot => {
    try {
        return encrypt_decoded_multi_contest_js(ballotSelection, election)
    } catch (error) {
        console.log(error)
        throw error
    }
}

/*
 * Encodes and decodes the contest selection.
 * The result is getting the ballot selection back from sequent-core,
 * but this time with the invalid errors. Also this allows the system
 * to check that the ballot selection is the same.
 */
export const interpretContestSelection = (
    ballotSelection: BallotSelection,
    election: IBallotStyle
): BallotSelection => {
    try {
        return ballotSelection.map((contestSelection) =>
            test_contest_reencoding_js(contestSelection, election)
        )
    } catch (error) {
        console.log(error)
        throw error
    }
}

/*
 * Encodes and decodes the multi contest selection.
 * The result is getting the ballot selection back from sequent-core,
 * but this time with the invalid errors. Also this allows the system
 * to check that the multi ballot selection is the same.
 */
export const interpretMultiContestSelection = (
    ballotSelection: BallotSelection,
    election: IBallotStyle
): BallotSelection => {
    try {
        return test_multi_contest_reencoding_js(ballotSelection, election)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getWriteInAvailableCharacters = (
    contestSelection: IDecodedVoteContest,
    election: IBallotStyle
): number => {
    try {
        return get_write_in_available_characters_js(contestSelection, election)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const decodeAuditableBallot = (
    auditableBallot: IAuditableSingleBallot
): Array<IDecodedVoteContest> | null => {
    try {
        let decodedBallot = decode_auditable_ballot_js(auditableBallot)
        return decodedBallot as Array<IDecodedVoteContest>
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const decodeAuditableMultiBallot = (
    auditableBallot: IAuditableMultiBallot
): Array<IDecodedVoteContest> | null => {
    try {
        let decodedBallot = decode_auditable_multi_ballot_js(auditableBallot)
        return decodedBallot as Array<IDecodedVoteContest>
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const checkIsBlank = (contest: IDecodedVoteContest): boolean | null => {
    try {
        let is_blank: boolean = check_is_blank_js(contest)
        return is_blank
    } catch (error) {
        console.log(error)
        return null
    }
}

export const check_voting_not_allowed_next_bool = (
    contests: IContest[] | undefined,
    decodedContests: Record<string, IDecodedVoteContest>
): boolean => {
    try {
        return check_voting_not_allowed_next(contests, decodedContests)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const check_voting_error_dialog_bool = (
    contests: IContest[] | undefined,
    decodedContests: Record<string, IDecodedVoteContest>
): boolean => {
    try {
        return check_voting_error_dialog(contests, decodedContests)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const hashBallot512 = (auditableBallot: IAuditableSingleBallot): string => {
    try {
        return hash_auditable_ballot_js(auditableBallot)
    } catch (e) {
        console.log(e)
        throw e
    }
}

export const getLayoutProperties = (question: IContest): IContestLayoutProperties | null => {
    try {
        let properties = get_layout_properties_from_contest_js(question)
        return (properties || null) as IContestLayoutProperties | null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const getPoints = (question: IContest, answer: IDecodedVoteChoice): number | null => {
    try {
        let points: number | undefined = get_candidate_points_js(question, answer)
        return points || null
    } catch (error) {
        console.log(error)
        return null
    }
}

export const generateSampleAuditableBallot = (): IAuditableSingleBallot | null => {
    try {
        let auditableBallot: IAuditableSingleBallot = generate_sample_auditable_ballot_js()
        return auditableBallot
    } catch (error) {
        console.log(error)
        return null
    }
}
