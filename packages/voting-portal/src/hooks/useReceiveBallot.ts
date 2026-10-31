// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useRef, useState} from "react"
import {useApolloClient, useMutation} from "@apollo/client/react"
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
import {EStageCheck, IReceiptView, runCastReceipt} from "../services/vitrine/castReceipt"
import type {ITransparency} from "../services/vitrine/presentation"
import {trackingDigestHex, trackingIdOf} from "../services/vitrine/trackingId"
import {ETransparencyMode} from "../services/vitrine/types"
import {voterContextFor} from "../services/vitrine/voterContext"
import {voterForward} from "../services/vitrine/forward"
import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    selectReceivedBallotId,
    setReceivedBallot,
} from "../store/receivedBallots/receivedBallotsSlice"
import {markBallotSent} from "../store/receivedBallots/sentBallotMarker"
import {AuthContext} from "../providers/AuthContextProvider"

export enum EReceiveBallotStatus {
    NOT_REQUIRED = "not-required",
    PENDING = "pending",
    RECEIVED = "received",
    FAILED = "failed",
}

export interface IReceiveBallotState {
    status: EReceiveBallotStatus
    // The ballot box's key for the ballot, once this device has checked its
    // signature. It is what the cast request names, never what the voter reads.
    ballotId?: string
    // The Tracking ID the voter reads: known as soon as the ballot is, since
    // nothing the ballot box returns goes into it.
    trackingId?: string
    // The check of the ballot's `received` entry in the ballots log, for events
    // with transparency on; it upgrades while the review screen is open.
    view?: IReceiptView
    errorMsg?: string
}

interface IReceiveBallotParams {
    ballotStyle?: IBallotStyle
    auditableBallot?: IAuditableBallot
    ballotHash?: string
    isMultiContest: boolean
    // The event's transparency: with it on, the ballot box's answer must come
    // with a receipt (or a promise) for the ballot's `received` entry.
    transparency?: ITransparency
    // Nothing reaches the ballot box: a demo ballot, or no ballot at all.
    skip: boolean
}

export const isReceivedAtReview = (ballotStyle?: IBallotStyle): boolean =>
    ballotStyle?.ballot_eml.election_event_presentation?.receipts?.policy ===
    EReceiptsPolicy.SIGNED_BY_BALLOT_BOX

/**
 * Sends the voter-signed ballot to the ballot box when the review screen
 * opens. The Tracking ID shows at once; the ballot counts as received only
 * once the ballot box has stored and signed it, this device has checked that
 * signature and, with transparency on, the log's receipt or promise for its
 * `received` entry.
 */
export const useReceiveBallot = ({
    ballotStyle,
    auditableBallot,
    ballotHash,
    isMultiContest,
    transparency,
    skip,
}: IReceiveBallotParams): IReceiveBallotState => {
    const client = useApolloClient()
    const {t} = useTranslation()
    const dispatch = useAppDispatch()
    const {userId} = useContext(AuthContext)
    const [receiveBallot] = useMutation<ReceiveBallotMutation>(RECEIVE_BALLOT)
    const {toHashableBallot, toHashableMultiBallot, verifyReceivedBallot} = provideBallotService()
    const required = isReceivedAtReview(ballotStyle) && !skip
    const electionId = ballotStyle?.election_id ?? ""
    const receivedBallotId = useAppSelector(selectReceivedBallotId(electionId, ballotHash))
    const [failure, setFailure] = useState<{errorMsg: string}>()
    // The same ballot is sent once: going back to the ballot makes a new one.
    const sentBallotHash = useRef<string | undefined>(undefined)
    const [view, setView] = useState<IReceiptView>()
    const checking = useRef<AbortController | undefined>(undefined)
    useEffect(() => () => checking.current?.abort(), [])
    const trackingId = trackingIdOf(ballotHash, auditableBallot?.voter_signing_pk)

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

            // With transparency on, the stage counts as reached when the log's
            // receipt (or an acceptable promise) for the `received` entry checks.
            // A queued entry, or a log that cannot be reached, leaves the ballot
            // box's own signature, checked above, and the log is asked again.
            if (transparency && transparency.mode !== ETransparencyMode.DISABLED) {
                const digest = trackingId && (await trackingDigestHex(trackingId))
                if (!digest) {
                    setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NOT_RECEIVED}`))
                    return
                }
                checking.current?.abort()
                const controller = new AbortController()
                checking.current = controller
                const {election_event_id: eventId, election_id: electionId} = ballotStyle
                const check = await new Promise<EStageCheck>((resolve) => {
                    void runCastReceipt({
                        cast: answer.vitrine ?? null,
                        target: {eventId, electionId, stage: "received", ballotId: digest},
                        transparency,
                        context: () => voterContextFor(client, eventId, electionId),
                        forward: voterForward(client, eventId),
                        signal: controller.signal,
                        checkOnly: true,
                        onChecked: resolve,
                        onView: setView,
                    }).then(() => resolve(EStageCheck.FAILED))
                })
                if (check === EStageCheck.FAILED) {
                    setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NOT_RECEIVED}`))
                    return
                }
            }
            dispatch(setReceivedBallot({electionId: ballotStyle.election_id, receivedBallot}))
            // A reload now drops the key that casts this ballot; the review
            // screen tells the voter when it finds this after one.
            markBallotSent({
                owner: {
                    tenantId: ballotStyle.tenant_id,
                    eventId: ballotStyle.election_event_id,
                    userId,
                },
                electionId: ballotStyle.election_id,
                trackingId: trackingId ?? "",
            })
        }
        send()
    }, [required, ballotStyle, auditableBallot, ballotHash, receivedBallotId])

    if (!required) {
        return {status: EReceiveBallotStatus.NOT_REQUIRED}
    }
    if (receivedBallotId) {
        return {
            status: EReceiveBallotStatus.RECEIVED,
            ballotId: receivedBallotId,
            trackingId,
            view,
        }
    }
    if (failure) {
        return {status: EReceiveBallotStatus.FAILED, errorMsg: failure.errorMsg, trackingId, view}
    }
    return {status: EReceiveBallotStatus.PENDING, trackingId}
}
