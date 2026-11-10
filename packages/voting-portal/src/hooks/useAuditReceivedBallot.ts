// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMutation} from "@apollo/client/react"
import {useTranslation} from "react-i18next"
import {EReceiptsPolicy, IAuditableBallot} from "@sequentech/ui-core"
import {
    AUDIT_BALLOT,
    AuditBallotMutation,
    AuditBallotMutationVariables,
} from "../queries/AuditBallot"
import {CastBallotsErrorType} from "../services/VotingPortalError"
import {confirmBallotAudited} from "../store/auditableBallots/auditableBallotsSlice"
import {useAppDispatch} from "../store/hooks"

// Whether the ballot box receives this ballot at review, so that showing its
// secret values takes the ballot box's confirmation that it marked the ballot
// audited. Demo ballots and portals without authentication send nothing to the
// ballot box.
export const isAuditedByBallotBox = (
    auditableBallot: IAuditableBallot | undefined,
    disableAuth: boolean
): boolean =>
    !!auditableBallot &&
    auditableBallot.config.election_event_presentation?.receipts?.policy ===
        EReceiptsPolicy.SIGNED_BY_BALLOT_BOX &&
    !auditableBallot.config.public_key?.is_demo &&
    !disableAuth

// The ballot the voter audits: the Ballot ID of the ballot box's receipt and
// the hash this device computed for the ballot.
export interface IAuditedBallot {
    electionId: string
    ballotId: string
    ballotHash: string
}

/**
 * Asks the ballot box to mark a received ballot audited, so that it is never
 * cast. The audit counts as confirmed only when the ballot box names this
 * same ballot in its answer.
 */
export const useAuditReceivedBallot = () => {
    const {t} = useTranslation()
    const dispatch = useAppDispatch()
    const [auditBallot] = useMutation<AuditBallotMutation, AuditBallotMutationVariables>(
        AUDIT_BALLOT
    )

    return async (
        {electionId, ballotId, ballotHash}: IAuditedBallot,
        setErrorMsg: (msg: string) => void
    ): Promise<boolean> => {
        const notConfirmed = () => {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.AUDIT_BALLOT}`))
            return false
        }
        try {
            const result = await auditBallot({variables: {electionId, ballotId}})
            const audited = result.data?.audit_ballot
            if (result.error || !audited) {
                console.log(result.error?.message)
                return notConfirmed()
            }
            if (audited.ballot_id !== ballotId || audited.ballot_hash !== ballotHash) {
                console.error("The ballot box confirmed the audit of another ballot")
                return notConfirmed()
            }
        } catch (error) {
            console.log(error)
            return notConfirmed()
        }
        dispatch(confirmBallotAudited(electionId))
        return true
    }
}
