// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {isId} from "./operationalGoldReturn"
import type {ISignedView} from "./request"
import {SigningAction, type ISigningRequest} from "./types"

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

export const signingExperience = (
    mode: unknown,
    action: string
): "legacy" | "managed" | "direct" | "unavailable" => {
    if (mode === "legacy-p12") return "legacy"
    if (mode === "hsm-demo-raw" && action === "initialize-voting") return "direct"
    return mode === "hsm-required" && action === "initialize-voting" ? "managed" : "unavailable"
}

export const approvalControl = (phase: string, reviewed: boolean): "set-up" | "approve" | null => {
    if (phase === "setup-required" || phase === "setup-retry-available") return "set-up"
    return phase === "challenge-available" && reviewed ? "approve" : null
}
