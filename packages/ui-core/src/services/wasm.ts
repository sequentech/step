// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import SequentCoreLibInit, {
    IContestLayoutProperties,
    IDecodedVoteChoice,
    generate_sample_auditable_ballot_js,
    get_candidate_points_js,
    is_eligible_acclaimed_candidate_js,
    get_layout_properties_from_contest_js,
    set_hooks,
    get_default_consolidated_report_policy_js,
    get_default_language_detection_policy_js,
    get_default_decline_to_vote_policy_js,
    get_default_blank_ballots_policy_js,
    get_default_voting_screen_back_policy_js,
    get_voting_screen_back_policy_values_js,
    IVotingScreenBackPolicy,
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
    is_preferential_js,
    test_multi_contest_reencoding_js,
    get_write_in_available_characters_js,
    check_is_blank_js,
    sign_hashable_ballot_with_ephemeral_voter_signing_key_js,
    sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js,
    IDecodedVoteContest,
    check_voting_not_allowed_next,
    check_voting_error_dialog,
    filter_visible_messages_js,
    verify_ballot_signature_js,
    verify_multi_ballot_signature_js,
    get_default_duplicated_rank_policy_js,
    get_default_preference_gaps_policy_js,
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
    ISignedContent,
    ICountingAlgorithm,
    EDuplicatedRankPolicy,
    EPreferenceGapsPolicy,
    EConsolidatedReportPolicy,
    ELanguageDetectionPolicy,
    EDeclineToVotePolicy,
    EBlankBallotsPolicy,
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

// Create a variable to hold the singleton promise
let initializationPromise: Promise<void> | null = null

/**
 * Initializes the Sequent Core WASM library.
 * This function is a singleton and will only run the initialization once.
 * @returns A promise that resolves when the library is ready.
 */
export const initCore = (): Promise<void> => {
    // If the promise doesn't exist yet, create it
    if (!initializationPromise) {
        initializationPromise = SequentCoreLibInit()
            .then((_core) => {
                // The set_hooks function is often passed the core module itself
                set_hooks()
            })
            .catch((error) => {
                console.error("Error initializing SequentCoreLib:", error)
                // Re-throw the error to let consumers handle it
                throw error
            })
    }
    // Return the existing promise on subsequent calls
    return initializationPromise
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

/**
 * Canonical eligibility policy for candidates elected by acclamation.
 * The implementation lives in Sequent Core and is shared with tally,
 * publication, the verifier, and IVR.
 */
export const isEligibleAcclaimedCandidate = (candidate: ICandidate): boolean => {
    try {
        return is_eligible_acclaimed_candidate_js(candidate)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const isPreferential = (countingAlgorithm?: ICountingAlgorithm): boolean => {
    if (!countingAlgorithm) return false
    try {
        return is_preferential_js(countingAlgorithm)
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

export const signHashableBallot = (
    ballot_id: string,
    election_id: string,
    content: IAuditableSingleBallot
): ISignedContent => {
    try {
        return sign_hashable_ballot_with_ephemeral_voter_signing_key_js(
            ballot_id,
            election_id,
            content
        )
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const signHashableMultiBallot = (
    ballot_id: string,
    election_id: string,
    content: IAuditableMultiBallot
): ISignedContent => {
    try {
        return sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js(
            ballot_id,
            election_id,
            content
        )
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

export const verifyBallotSignature = (
    ballot_id: string,
    election_id: string,
    content: IAuditableSingleBallot
): boolean | null => {
    try {
        let isVerified: boolean = verify_ballot_signature_js(ballot_id, election_id, content)
        return isVerified
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const verifyMultiBallotSignature = (
    ballot_id: string,
    election_id: string,
    content: IAuditableMultiBallot
): boolean | null => {
    try {
        let isVerified: boolean = verify_multi_ballot_signature_js(ballot_id, election_id, content)
        return isVerified
    } catch (error) {
        console.log(error)
        throw error
    }
}

/*
 * The decoded contest reduced to the messages the voter should see on the
 * screen being rendered: the same record with invalid_errors and
 * invalid_alerts filtered by the validation rules in sequent-core.
 * isReview selects the review screen over the voting screen; isTouched is
 * whether the voter has selected anything in this contest yet.
 */
export const filterVisibleMessages = (
    contest: IContest,
    decodedContest: IDecodedVoteContest,
    isReview: boolean,
    isTouched: boolean
): IDecodedVoteContest => {
    try {
        return filter_visible_messages_js(contest, decodedContest, isReview, isTouched)
    } catch (error) {
        console.log(error)
        throw error
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

export const getDefaultDuplicatedRankPolicy = (): EDuplicatedRankPolicy => {
    try {
        return get_default_duplicated_rank_policy_js() as EDuplicatedRankPolicy
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getDefaultPreferenceGapsPolicy = (): EPreferenceGapsPolicy => {
    try {
        return get_default_preference_gaps_policy_js() as EPreferenceGapsPolicy
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getDefaultConsolidatedReportPolicy = (): EConsolidatedReportPolicy => {
    try {
        return get_default_consolidated_report_policy_js() as EConsolidatedReportPolicy
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getDefaultLanguageDetectionPolicy = (): ELanguageDetectionPolicy => {
    try {
        return get_default_language_detection_policy_js() as ELanguageDetectionPolicy
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getDefaultDeclineToVotePolicy = (): EDeclineToVotePolicy => {
    try {
        return get_default_decline_to_vote_policy_js() as EDeclineToVotePolicy
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getDefaultBlankBallotsPolicy = (): EBlankBallotsPolicy => {
    try {
        return get_default_blank_ballots_policy_js() as EBlankBallotsPolicy
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getDefaultVotingScreenBackPolicy = (): IVotingScreenBackPolicy => {
    try {
        return get_default_voting_screen_back_policy_js() as IVotingScreenBackPolicy
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const getVotingScreenBackPolicyValues = (): IVotingScreenBackPolicy[] => {
    try {
        return get_voting_screen_back_policy_values_js() as IVotingScreenBackPolicy[]
    } catch (error) {
        console.log(error)
        throw error
    }
}
