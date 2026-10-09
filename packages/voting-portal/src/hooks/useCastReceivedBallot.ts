// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMutation} from "@apollo/client/react"
import {useTranslation} from "react-i18next"
import {
    IBallotBoxKey,
    ICastReceipt,
    IReceivedBallot,
    escapeTranslationValues,
} from "@sequentech/ui-core"
import {CastBallotMutation} from "../gql/graphql"
import {CAST_BALLOT} from "../queries/CastBallot"
import {provideBallotService} from "../services/BallotService"
import {castVoteErrorMessage} from "../services/CastVoteErrors"
import {CastBallotsErrorType} from "../services/VotingPortalError"
import {addCastVotes} from "../store/castVotes/castVotesSlice"
import {useAppDispatch} from "../store/hooks"

// A ballot the ballot box received at review, with the voter's signed request
// to cast it. It holds no key, so it can wait in the session for a fresh
// sign-in.
export interface IReceivedCast {
    receivedBallot: IReceivedBallot
    ballotBoxKey: IBallotBoxKey
    castSignature: string
}

// Signs "cast this Ballot ID" with the key that signed the ballot at review.
// Throws when the page no longer holds that key.
export const signReceivedCast = (
    receivedBallot: IReceivedBallot,
    ballotBoxKey: IBallotBoxKey
): IReceivedCast => ({
    receivedBallot,
    ballotBoxKey,
    castSignature: provideBallotService().signBallotCast(
        receivedBallot.election_id,
        receivedBallot.voter_signing_pk,
        receivedBallot.ballot_id
    ),
})

/**
 * Casts a received ballot. The cast counts as done only once this device has
 * checked the ballot box's signature over the cast receipt.
 */
export const useCastReceivedBallot = () => {
    const {t} = useTranslation()
    const dispatch = useAppDispatch()
    const [castBallot] = useMutation<CastBallotMutation>(CAST_BALLOT)
    const {verifyCastReceipt, forgetVoterSigningKey} = provideBallotService()

    return async (
        {receivedBallot, ballotBoxKey, castSignature}: IReceivedCast,
        setErrorMsg: (msg: string) => void
    ): Promise<boolean> => {
        let answer: NonNullable<CastBallotMutation["cast_ballot"]>
        try {
            const result = await castBallot({
                variables: {
                    electionId: receivedBallot.election_id,
                    ballotId: receivedBallot.ballot_id,
                    castSignature,
                },
            })
            const cast = result.data?.cast_ballot
            if (result.error || !cast) {
                console.log(result.error?.message)
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
                return false
            }
            answer = cast
        } catch (error) {
            console.log(error)
            setErrorMsg(castVoteErrorMessage(error, t))
            return false
        }

        // The signature must cover the ballot this device asked to cast, so
        // only the time, the key and the signature are taken from the answer.
        const castReceipt: ICastReceipt = {
            election_event_id: receivedBallot.election_event_id,
            election_id: receivedBallot.election_id,
            ballot_id: receivedBallot.ballot_id,
            received_at: receivedBallot.received_at,
            cast_at: answer.cast_at,
            key_id: answer.key_id,
            cast_signature: castSignature,
            cast_receipt_signature: answer.cast_receipt_signature,
        }
        try {
            verifyCastReceipt(ballotBoxKey, castReceipt)
        } catch (error) {
            console.error("The ballot box's cast receipt does not verify:", error)
            setErrorMsg(
                t(
                    `reviewScreen.error.${CastBallotsErrorType.INCONSISTENT_HASH}`,
                    escapeTranslationValues({
                        ballotId: receivedBallot.ballot_id,
                        auditableBallotHash: receivedBallot.ballot_hash,
                    })
                )
            )
            return false
        }

        forgetVoterSigningKey(receivedBallot.election_id)
        dispatch(
            addCastVotes([
                {
                    id: answer.cast_vote_id,
                    tenant_id: receivedBallot.tenant_id,
                    election_id: receivedBallot.election_id,
                    election_event_id: receivedBallot.election_event_id,
                    created_at: answer.cast_at,
                },
            ])
        )
        return true
    }
}
