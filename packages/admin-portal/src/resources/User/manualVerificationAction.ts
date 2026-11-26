// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ManualVerificationMutation, ManualVerificationMutationVariables} from "@/gql/graphql"
import type {ISigningPanelData} from "@/lib/signing/api"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {signedView} from "@/lib/signing/request"
import {isId} from "@/lib/signing/operationalGoldReturn"
import {isVoterAccountId, reviewedManualVerification} from "@/lib/signing/operationalApprovalPolicy"

export function verificationResult(
    result: ManualVerificationMutation["get_manual_verification_pdf"]
) {
    if (result?.signing_request != null) {
        // An ambiguous result must never start a download.
        if (
            result.status === "awaiting-signatures" &&
            !result.document_id &&
            isId(result.signing_request.id)
        ) {
            return {requestId: result.signing_request.id as string}
        }
    } else if (isId(result?.document_id) && result?.status === "pending") {
        return {documentId: result.document_id}
    }
    throw new Error("Unexpected manual verification response")
}

/** A user returning through Signatures > Requests retrieves the exact document, not a fresh request. */
export function verificationRetrieval(
    data: ISigningPanelData,
    userId: string,
    tenantId?: string | null
): ManualVerificationMutationVariables | null {
    const {request} = data
    if (
        !userId ||
        !isId(request.id) ||
        request.requested_by !== userId ||
        request.tenant_id !== tenantId ||
        request.action !== SigningAction.AuthorizeManualVerification ||
        request.status !== SigningRequestStatus.Executed
    )
        return null
    try {
        const view = signedView(data)
        if (!reviewedManualVerification(request, view)) return null
        const {subject} = view
        if (!isVoterAccountId(subject.voter_id) || !isId(subject.initiation_id)) return null
        return {
            tenantId: request.tenant_id,
            electionEventId: request.election_event_id,
            voterId: subject.voter_id,
            initiationId: subject.initiation_id,
            recoverRequestId: request.id,
        }
    } catch {
        return null
    }
}
