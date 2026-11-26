// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ISigningPanelData} from "@/lib/signing/api"
import {canonicalJson} from "@/lib/signing/request"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {verificationResult, verificationRetrieval} from "./manualVerificationAction"

const id = (n: number) => `ba000000-0000-4000-8000-${n.toString().padStart(12, "0")}`
const tenant = id(1),
    event = id(2),
    post = id(3),
    voter = "f:synthetic-directory:synthetic-account",
    actor = id(5),
    initiation = id(6)
const action = SigningAction.AuthorizeManualVerification
const subject = {
    initiation_id: initiation,
    voter_id: voter,
    requester_id: actor,
    post_id: post,
    area_id: id(8),
}
const payload = {
    domain: "step-signing/v1",
    request_id: id(7),
    code: "0123-4567",
    action,
    tenant_id: tenant,
    election_event_id: event,
    election_id: post,
    area_id: null,
    requested_by: actor,
    subject,
}
const panel = (status: SigningRequestStatus, change: Record<string, unknown> = {}) =>
    ({
        request: {
            id: payload.request_id,
            code: payload.code,
            action,
            tenant_id: tenant,
            election_event_id: event,
            election_id: post,
            area_id: null,
            requested_by: actor,
            canonical_payload: canonicalJson({...payload, ...change}),
            status,
            subject,
        },
    }) as unknown as ISigningPanelData

describe("requesting a manual identity verification document", () => {
    it("accepts only a private document or a waiting request, never both", () => {
        expect(verificationResult({document_id: id(8), status: "pending"})).toEqual({
            documentId: id(8),
        })
        expect(
            verificationResult({
                document_id: null,
                status: "awaiting-signatures",
                signing_request: {id: id(9)},
            })
        ).toEqual({requestId: id(9)})
        for (const reply of [
            {document_id: id(8), status: "pending", signing_request: {id: id(9)}},
            {document_id: id(8), status: "awaiting-signatures"},
            {document_id: null, status: "pending", signing_request: {id: id(9)}},
            {document_id: id(8), status: "failed"},
            {document_id: null, status: "awaiting-signatures", signing_request: {}},
            {document_id: null, status: "pending"},
            null,
        ])
            expect(() => verificationResult(reply)).toThrow()
    })

    it("offers the document only to the requester of the executed, exact-bound operation", () => {
        expect(verificationRetrieval(panel(SigningRequestStatus.Executed), actor, tenant)).toEqual({
            tenantId: tenant,
            electionEventId: event,
            voterId: voter,
            initiationId: initiation,
            recoverRequestId: payload.request_id,
        })
        for (const state of [
            SigningRequestStatus.Waiting,
            SigningRequestStatus.Completed,
            SigningRequestStatus.Failed,
            SigningRequestStatus.Cancelled,
            SigningRequestStatus.Expired,
        ])
            expect(verificationRetrieval(panel(state), actor, tenant)).toBeNull()
        expect(
            verificationRetrieval(panel(SigningRequestStatus.Executed), id(10), tenant)
        ).toBeNull()
        expect(
            verificationRetrieval(panel(SigningRequestStatus.Executed), actor, id(10))
        ).toBeNull()
        for (const change of [
            {action: SigningAction.ApproveVoter},
            {election_event_id: id(11)},
            {requested_by: id(11)},
            {subject: {...subject, requester_id: id(12)}},
            {subject: {...subject, post_id: id(11)}},
            {subject: {...subject, voter_id: id(14)}},
            {subject: {...subject, initiation_id: id(15)}},
            {subject: {...subject, initiation_id: ""}},
            {subject: {...subject, voter_id: null}},
        ])
            expect(
                verificationRetrieval(panel(SigningRequestStatus.Executed, change), actor, tenant)
            ).toBeNull()
    })
})
