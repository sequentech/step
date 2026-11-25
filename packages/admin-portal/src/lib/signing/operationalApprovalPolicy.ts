// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {isId} from "./operationalGoldReturn"
import type {ISignedView} from "./request"
import {DocumentKind, SIGNING_ACTIONS, SigningAction, type ISigningRequest} from "./types"

/** Check the subject displayed by the real canonical-payload reviewer. */
export function reviewedInitializePost(
    request: Pick<ISigningRequest, "action" | "election_id" | "subject">,
    review: ISignedView | null
) {
    return (
        review !== null &&
        request.action === SigningAction.InitializeVoting &&
        isId(request.election_id) &&
        review.payload.election_id === request.election_id &&
        review.payload.action === request.action &&
        isId(review.subject.publication_id) &&
        review.subject.publication_id === request.subject.publication_id
    )
}

/** The extended demo reuses the canonical reviewer and never a certificate prompt. */
export function reviewedDirectAction(request: ISigningRequest, review: ISignedView | null) {
    if (review === null || review.payload.action !== request.action) return false
    if (request.action === SigningAction.InitializeVoting) {
        return reviewedInitializePost(request, review)
    }
    if (request.action === SigningAction.ApproveConfiguration) {
        return isId(review.subject.ballot_publication_id)
    }
    if (
        request.action === SigningAction.ConfirmCustodyProof ||
        request.action === SigningAction.ContributeCustodyProof
    ) {
        return isId(request.trustee_id) && request.trustee_id === review.subject.trustee_id
    }
    const info = SIGNING_ACTIONS[request.action]
    if (!info || !isId(request.election_id)) return false
    if (info.document === DocumentKind.NoDocument) return true
    return (
        typeof request.document_sha256 === "string" &&
        /^[a-f0-9]{64}$/.test(request.document_sha256) &&
        request.document_sha256 === review.documentSha256
    )
}

export const signingExperience = (
    mode: unknown,
    action: string
): "legacy" | "managed" | "direct" | "unavailable" => {
    if (mode === "legacy-p12") return "legacy"
    if (mode === "hsm-demo-raw" && action === SigningAction.InitializeVoting) return "direct"
    if (mode === "hsm-demo-direct-action" && Object.values(SigningAction).some((id) => id === action))
        return "direct"
    return mode === "hsm-required" && action === "initialize-voting" ? "managed" : "unavailable"
}

export const approvalControl = (phase: string, reviewed: boolean): "set-up" | "approve" | null => {
    if (phase === "setup-required" || phase === "setup-retry-available") return "set-up"
    return phase === "challenge-available" && reviewed ? "approve" : null
}
