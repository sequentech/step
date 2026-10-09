// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useRef, useState} from "react"
import {useMutation} from "@apollo/client/react"
import {useTranslation} from "react-i18next"
import {
    EReceiptsPolicy,
    IAuditableBallot,
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    IReceivedBallot,
    escapeTranslationValues,
} from "@sequentech/ui-core"
import {ReceiveBallotMutation} from "../gql/graphql"
import {RECEIVE_BALLOT} from "../queries/ReceiveBallot"
import {provideBallotService} from "../services/BallotService"
import {castVoteErrorMessage} from "../services/CastVoteErrors"
import {CastBallotsErrorType} from "../services/VotingPortalError"
import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    selectReceivedBallotId,
    setReceivedBallot,
} from "../store/receivedBallots/receivedBallotsSlice"

export enum EReceiveBallotStatus {
    NOT_REQUIRED = "not-required",
    PENDING = "pending",
    RECEIVED = "received",
    FAILED = "failed",
}

export interface IReceiveBallotState {
    status: EReceiveBallotStatus
    // The ballot box's Ballot ID, once this device has checked its signature.
    ballotId?: string
    errorMsg?: string
}

interface IReceiveBallotParams {
    ballotStyle?: IBallotStyle
    auditableBallot?: IAuditableBallot
    ballotHash?: string
    isMultiContest: boolean
    // Nothing reaches the ballot box: a demo ballot, or no ballot at all.
    skip: boolean
}

export const isReceivedAtReview = (ballotStyle?: IBallotStyle): boolean =>
    ballotStyle?.ballot_eml.election_event_presentation?.receipts?.policy ===
    EReceiptsPolicy.SIGNED_BY_BALLOT_BOX

/**
 * Sends the voter-signed ballot to the ballot box when the review screen
 * opens. The Ballot ID is known only after the ballot box has stored and
 * signed the ballot and this device has checked that signature.
 */
export const useReceiveBallot = ({
    ballotStyle,
    auditableBallot,
    ballotHash,
    isMultiContest,
    skip,
}: IReceiveBallotParams): IReceiveBallotState => {
    const {t} = useTranslation()
    const dispatch = useAppDispatch()
    const [receiveBallot] = useMutation<ReceiveBallotMutation>(RECEIVE_BALLOT)
    const {toHashableBallot, toHashableMultiBallot, verifyReceivedBallot} = provideBallotService()
    const required = isReceivedAtReview(ballotStyle) && !skip
    const electionId = ballotStyle?.election_id ?? ""
    const receivedBallotId = useAppSelector(selectReceivedBallotId(electionId, ballotHash))
    const [failure, setFailure] = useState<{errorMsg: string}>()
    // The same ballot is sent once: going back to the ballot makes a new one.
    const sentBallotHash = useRef<string | undefined>(undefined)

    useEffect(() => {
        if (
            !required ||
            !ballotStyle ||
            !auditableBallot ||
            !ballotHash ||
            receivedBallotId ||
            sentBallotHash.current === ballotHash
        ) {
            return
        }
        sentBallotHash.current = ballotHash
        setFailure(undefined)
        const setErrorMsg = (errorMsg: string) => setFailure({errorMsg})

        const send = async () => {
            const ballotBoxKey = ballotStyle.ballot_eml.ballot_box_key
            const {voter_signing_pk, voter_ballot_signature} = auditableBallot
            if (!ballotBoxKey || !voter_signing_pk || !voter_ballot_signature) {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`))
                return
            }

            let answer: NonNullable<ReceiveBallotMutation["receive_ballot"]>
            try {
                const hashableBallot = isMultiContest
                    ? toHashableMultiBallot(auditableBallot as IAuditableMultiBallot)
                    : toHashableBallot(auditableBallot as IAuditableSingleBallot)
                const result = await receiveBallot({
                    variables: {
                        electionId: ballotStyle.election_id,
                        ballotId: ballotHash,
                        content: JSON.stringify(hashableBallot),
                    },
                })
                const received = result.data?.receive_ballot
                if (result.error || !received) {
                    console.log(result.error?.message)
                    setErrorMsg(
                        t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`)
                    )
                    return
                }
                answer = received
            } catch (error) {
                console.log(error)
                setErrorMsg(castVoteErrorMessage(error, t))
                return
            }

            // The signature must cover what this device sent, so only the
            // time, the key and the signatures are taken from the answer.
            const receivedBallot: IReceivedBallot = {
                tenant_id: ballotStyle.tenant_id,
                election_event_id: ballotStyle.election_event_id,
                election_id: ballotStyle.election_id,
                ballot_hash: ballotHash,
                voter_signing_pk,
                voter_ballot_signature,
                received_at: answer.received_at,
                key_id: answer.key_id,
                received_signature: answer.received_signature,
                ballot_id: answer.ballot_id,
            }
            try {
                verifyReceivedBallot(ballotBoxKey, receivedBallot)
            } catch (error) {
                console.error("The ballot box's receipt does not verify:", error)
                setErrorMsg(
                    t(
                        `reviewScreen.error.${CastBallotsErrorType.INCONSISTENT_HASH}`,
                        escapeTranslationValues({
                            ballotId: answer.ballot_id,
                            auditableBallotHash: ballotHash,
                        })
                    )
                )
                return
            }
            dispatch(setReceivedBallot({electionId: ballotStyle.election_id, receivedBallot}))
        }
        send()
    }, [required, ballotStyle, auditableBallot, ballotHash, receivedBallotId])

    if (!required) {
        return {status: EReceiveBallotStatus.NOT_REQUIRED}
    }
    if (receivedBallotId) {
        return {status: EReceiveBallotStatus.RECEIVED, ballotId: receivedBallotId}
    }
    if (failure) {
        return {status: EReceiveBallotStatus.FAILED, errorMsg: failure.errorMsg}
    }
    return {status: EReceiveBallotStatus.PENDING}
}
