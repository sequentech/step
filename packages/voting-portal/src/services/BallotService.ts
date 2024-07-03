// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    decode_auditable_ballot_js,
    to_hashable_ballot_js,
    hash_auditable_ballot_js,
    encrypt_decoded_contest_js,
    test_contest_reencoding_js,
    get_write_in_available_characters_js,
    check_is_blank_js,
    IDecodedVoteContest,
    check_voting_not_allowed_next,
    check_voting_error_dialog,
} from "sequent-core"
import {BallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {IBallotStyle, IAuditableBallot, IHashableBallot, IContest} from "@sequentech/ui-core"

export interface IBallotService {
    toHashableBallot: (auditableBallot: IAuditableBallot) => IHashableBallot
    hashBallot: (auditableBallot: IAuditableBallot) => string
    encryptBallotSelection: (
        ballotSelection: BallotSelection,
        election: IBallotStyle
    ) => IAuditableBallot
    interpretContestSelection: (
        contestSelection: IDecodedVoteContest,
        election: IBallotStyle
    ) => IDecodedVoteContest
    getWriteInAvailableCharacters: (
        contestSelection: IDecodedVoteContest,
        election: IBallotStyle
    ) => number
    decodeAuditableBallot: (auditableBallot: IAuditableBallot) => Array<IDecodedVoteContest> | null
    checkIsBlank: (contest: IDecodedVoteContest) => boolean | null
}

export const toHashableBallot = (auditableBallot: IAuditableBallot): IHashableBallot => {
    try {
        return to_hashable_ballot_js(auditableBallot)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const hashBallot = (auditableBallot: IAuditableBallot): string => {
    try {
        return hash_auditable_ballot_js(auditableBallot)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const encryptBallotSelection = (
    ballotSelection: BallotSelection,
    election: IBallotStyle
): IAuditableBallot => {
    try {
        return encrypt_decoded_contest_js(ballotSelection, election)
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
    contestSelection: IDecodedVoteContest,
    election: IBallotStyle
): IDecodedVoteContest => {
    try {
        return test_contest_reencoding_js(contestSelection, election)
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
    auditableBallot: IAuditableBallot
): Array<IDecodedVoteContest> | null => {
    try {
        let decodedBallot = decode_auditable_ballot_js(auditableBallot)
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

export const provideBallotService = (): IBallotService => ({
    toHashableBallot,
    hashBallot,
    encryptBallotSelection,
    interpretContestSelection,
    getWriteInAvailableCharacters,
    decodeAuditableBallot,
    checkIsBlank,
})
