// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useRef, useState} from "react"
import {useApolloClient, useMutation} from "@apollo/client/react"
import {useStore} from "react-redux"
import type {RootState} from "../store/store"
import type {AuditBallotMutation} from "../gql/graphql"
import {AUDIT_BALLOT} from "../queries/AuditBallot"
import {provideBallotService} from "../services/BallotService"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    EBallotAuditStatus,
    markBallotAudited,
    selectBallotAuditStatus,
    selectBallotForAudit,
} from "../store/auditableBallots/auditableBallotsSlice"
import {
    clearReceivedBallot,
    selectReceivedBallot,
} from "../store/receivedBallots/receivedBallotsSlice"
import {clearSessionStorageBallotData} from "../store/castVotes/sessionBallotData"
import {clearBallotSent} from "../store/receivedBallots/sentBallotMarker"
import {EStageCheck, IReceiptView, runCastReceipt} from "../services/vitrine/castReceipt"
import type {ITransparency} from "../services/vitrine/presentation"
import {trackingDigestHex, trackingIdOf} from "../services/vitrine/trackingId"
import {ETransparencyMode} from "../services/vitrine/types"
import {voterContextFor} from "../services/vitrine/voterContext"
import {voterForward} from "../services/vitrine/forward"

/** Reveal audit data only after the ballot box can no longer accept its cast. */
export const useAuditBallot = (
    electionId: string,
    serverReceiptRequired: boolean,
    transparency?: ITransparency
) => {
    const client = useApolloClient()
    const [view, setView] = useState<IReceiptView>()
    // When the ballot box received the encrypted ballot, which the audit
    // screen shows; the received ballot itself is dropped with the audit.
    const [receivedAt, setReceivedAt] = useState<string>()
    const dispatch = useAppDispatch()
    const store = useStore<RootState>()
    const ballot = useAppSelector(selectBallotForAudit(electionId))
    const status = useAppSelector(selectBallotAuditStatus(electionId))
    const received = useAppSelector(selectReceivedBallot(electionId))
    const [auditBallot] = useMutation<AuditBallotMutation>(AUDIT_BALLOT)
    const [error, setError] = useState(false)
    const [loading, setLoading] = useState(false)
    const attempted = useRef<string | undefined>(undefined)
    const audit = useCallback(async () => {
        if (!ballot || status === EBallotAuditStatus.AUDITED) return
        setError(false)
        setLoading(true)
        try {
            if (serverReceiptRequired) {
                if (!received || received.ballot_hash !== ballot.ballot_hash)
                    throw new Error("The received ballot is unavailable")
                const result = await auditBallot({
                    variables: {electionId, ballotId: received.ballot_id},
                })
                const answer = result.data?.audit_ballot
                if (
                    result.error ||
                    answer?.ballot_id !== received.ballot_id ||
                    answer.status !== "audited"
                )
                    throw new Error("The ballot box did not confirm the audit")
                // With transparency on, the audit counts when the log's receipt
                // (or an acceptable promise) for the `audited` entry checks.
                if (transparency && transparency.mode !== ETransparencyMode.DISABLED) {
                    const trackingId = trackingIdOf(received.ballot_hash, received.voter_signing_pk)
                    const digest = trackingId && (await trackingDigestHex(trackingId))
                    if (!digest) throw new Error("The Tracking ID cannot be computed")
                    const eventId = received.election_event_id
                    const controller = new AbortController()
                    const check = await new Promise<EStageCheck>((resolve) => {
                        void runCastReceipt({
                            cast: answer?.vitrine ?? null,
                            target: {eventId, electionId, stage: "audited", ballotId: digest},
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
                        controller.abort()
                        throw new Error("The log holds no receipt for the audited ballot")
                    }
                }
            }
            // The user may have gone Back and encrypted a new ballot while the
            // request was pending. Never invalidate that new ballot or its key.
            if (
                store.getState().auditableBallots[electionId]?.auditableBallot?.ballot_hash ===
                ballot.ballot_hash
            ) {
                if (serverReceiptRequired && received) setReceivedAt(received.received_at)
                provideBallotService().forgetVoterSigningKey(electionId)
                dispatch(markBallotAudited({electionId, ballotHash: ballot.ballot_hash}))
                dispatch(clearReceivedBallot(electionId))
                clearBallotSent(electionId)
                clearSessionStorageBallotData()
            }
        } catch {
            setError(true)
        } finally {
            setLoading(false)
        }
    }, [
        auditBallot,
        ballot,
        client,
        dispatch,
        electionId,
        received,
        serverReceiptRequired,
        status,
        store,
        transparency,
    ])
    useEffect(() => {
        if (!ballot || status === EBallotAuditStatus.AUDITED) return
        const key = `${electionId}:${ballot.ballot_hash}`
        if (attempted.current === key) return
        attempted.current = key
        void audit()
    }, [audit, ballot, electionId, status])
    return {
        ready: status === EBallotAuditStatus.AUDITED,
        loading,
        error,
        retry: audit,
        view,
        receivedAt,
    }
}
