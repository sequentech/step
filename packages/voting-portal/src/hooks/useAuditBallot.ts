// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useRef, useState} from "react"
import {useMutation} from "@apollo/client/react"
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

/** Reveal audit data only after the ballot box can no longer accept its cast. */
export const useAuditBallot = (electionId: string, serverReceiptRequired: boolean) => {
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
            }
            // The user may have gone Back and encrypted a new ballot while the
            // request was pending. Never invalidate that new ballot or its key.
            if (
                store.getState().auditableBallots[electionId]?.auditableBallot?.ballot_hash ===
                ballot.ballot_hash
            ) {
                provideBallotService().forgetVoterSigningKey(electionId)
                dispatch(markBallotAudited({electionId, ballotHash: ballot.ballot_hash}))
                dispatch(clearReceivedBallot(electionId))
                clearSessionStorageBallotData()
            }
        } catch {
            setError(true)
        } finally {
            setLoading(false)
        }
    }, [auditBallot, ballot, dispatch, electionId, received, serverReceiptRequired, status, store])
    useEffect(() => {
        if (!ballot || status === EBallotAuditStatus.AUDITED) return
        const key = `${electionId}:${ballot.ballot_hash}`
        if (attempted.current === key) return
        attempted.current = key
        void audit()
    }, [audit, ballot, electionId, status])
    return {ready: status === EBallotAuditStatus.AUDITED, loading, error, retry: audit}
}
