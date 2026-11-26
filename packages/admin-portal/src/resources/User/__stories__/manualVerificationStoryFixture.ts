// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EVENT_ID, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {ISigningPanelData} from "@/lib/signing/api"
import {canonicalJson} from "@/lib/signing/request"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"

export const ACTOR = "ba000000-0000-4000-8000-000000000001"
export const OTHER_ACTOR = "ba000000-0000-4000-8000-000000000002"
export const REQUEST = "ba000000-0000-4000-8000-000000000003"
export const POST = "ba000000-0000-4000-8000-000000000004"
export const AREA = "ba000000-0000-4000-8000-000000000005"
export const DOCUMENT = "ba000000-0000-4000-8000-000000000006"
export const INITIATION = "ba000000-0000-4000-8000-000000000007"
export const VOTER = "f:synthetic-directory:synthetic-account"
export const recovery = {
    tenantId: TENANT_ID,
    electionEventId: EVENT_ID,
    voterId: VOTER,
    initiationId: INITIATION,
    recoverRequestId: REQUEST,
}

export function panel(status: SigningRequestStatus): ISigningPanelData {
    const payload = {
        domain: "step-signing/v1",
        request_id: REQUEST,
        code: "0123-4567",
        action: SigningAction.AuthorizeManualVerification,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: POST,
        area_id: null,
        requested_by: ACTOR,
        subject: {
            initiation_id: INITIATION,
            voter_id: VOTER,
            requester_id: ACTOR,
            post_id: POST,
            area_id: AREA,
        },
    }
    return {
        request: {...payload, id: REQUEST, status, canonical_payload: canonicalJson(payload)},
    } as unknown as ISigningPanelData
}

export function documentBoundary() {
    return graphqlBoundary({
        GetDocument: () => ({data: {sequent_backend_document: []}}),
        FetchDocument: () => ({
            data: {
                fetchDocument: {url: "data:application/pdf,%25PDF-1.4%0A%25Synthetic%0A%25%25EOF"},
            },
        }),
    })
}
