/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {SigningDialog} from "./SigningDialog"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {
    SigningAction,
    SigningRequestStatus,
    SigningRequirement,
    RequesterSigning,
} from "@/lib/signing/types"
import {BOARD_SIGNING_RESUME_KEY} from "@/lib/boardAuthentication"
const mockReauthenticate = jest.fn(async (_uri: string) => {})
let mockFresh = false
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        userId: "board1",
        username: "board1",
        isFreshBoardAuthentication: () => mockFresh,
        reauthenticateBoard: (uri: string) => mockReauthenticate(uri),
    }),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (x: string) => x, i18n: {language: "en"}}),
}))
jest.mock("@/lib/signing/api", () => ({
    ...jest.requireActual("@/lib/signing/api"),
    validatePanel: (data: ISigningPanelData) => data,
}))
jest.mock("./useSignedView", () => ({
    useSignedView: () => ({verified: true, problem: null, view: {}}),
}))
jest.mock("./SigningSubject", () => ({
    SigningSubject: () => null,
    SigningSubjectVariant: {Dialog: "dialog"},
}))
jest.mock("./format", () => ({
    requestTitle: () => "Review",
    actionDescription: () => "Close voting",
    actionObject: () => "voting",
    useSigningFormat: () => ({date: () => "date", time: () => "time"}),
    shortFingerprint: () => "public",
    organizationName: () => "org",
    formatList: () => "people",
}))
jest.mock("@/providers/TenantContextProvider", () => ({
    TenantContext: require("react").createContext({tenant: undefined}),
}))
const mockOpen = jest.fn()
const mockSign = jest.fn()
jest.mock("@/lib/signing/sign", () => ({
    openP12Choices: (...args: unknown[]) => mockOpen(...args),
    signAndApprove: (...args: unknown[]) => mockSign(...args),
}))
const data: ISigningPanelData = {
    request: {
        id: "request",
        tenant_id: "tenant",
        election_event_id: "event",
        action: SigningAction.CloseVoting,
        election_id: null,
        area_id: null,
        trustee_id: null,
        subject: {channels: ["ONLINE"]},
        canonical_payload: "{}",
        payload_sha256: "a".repeat(64),
        document_id: null,
        document_sha256: null,
        code: "ABCD-EFGH",
        config_revision: null,
        rule_revision: 1,
        required: 2,
        status: SigningRequestStatus.Waiting,
        cancel_reason: null,
        cancelled_by: null,
        requested_by: "board1",
        requested_by_username: "board1",
        created_at: "2026-01-01T00:00:00Z",
        expires_at: null,
        completed_at: null,
        executed_at: null,
        execution_result: null,
    },
    signers: [],
    rule: {
        action: SigningAction.CloseVoting,
        requirement: SigningRequirement.Required,
        signatures: 2,
        requester_signing: RequesterSigning.Allowed,
        expires_minutes: null,
        revision: 1,
    },
    count: 0,
    document_url: null,
}
const api = {
    getRequest: jest.fn(async () => data),
    reportOpenFailure: jest.fn(),
    checkCertificate: jest.fn(async () => ({checks: [{id: "chain", ok: true}]})),
    approve: jest.fn(),
} as unknown as ISigningApi
beforeEach(() => {
    mockFresh = false
    mockReauthenticate.mockClear()
    mockOpen.mockClear()
    mockSign.mockClear()
    sessionStorage.clear()
})
it("reauthenticates before exposing the private file input and retains only exact public bindings", async () => {
    render(<SigningDialog open data={data} api={api} onClose={() => {}} />)
    fireEvent.click(screen.getByRole("button", {name: "signing.widget.continue"}))
    await waitFor(() => expect(mockReauthenticate).toHaveBeenCalledTimes(1))
    expect(document.querySelector('input[type="file"]')).toBeNull()
    expect(mockOpen).not.toHaveBeenCalled()
    expect(api.approve).not.toHaveBeenCalled()
    expect(JSON.parse(sessionStorage.getItem(BOARD_SIGNING_RESUME_KEY)!)).toMatchObject({
        requestId: "request",
        tenantId: "tenant",
        eventId: "event",
        userId: "board1",
        action: SigningAction.CloseVoting,
        payloadHash: "a".repeat(64),
        code: "ABCD-EFGH",
    })
    expect(await screen.findByText("publish.dialog.errorReauth")).toBeTruthy()
})
it("allows a freshly authenticated person to choose a private file without a redirect", async () => {
    mockFresh = true
    render(<SigningDialog open data={data} api={api} onClose={() => {}} />)
    fireEvent.click(screen.getByRole("button", {name: "signing.widget.continue"}))
    await waitFor(() => expect(document.querySelector('input[type="file"]')).not.toBeNull())
    expect(mockReauthenticate).not.toHaveBeenCalled()
    expect(mockOpen).not.toHaveBeenCalled()
})
it.each([SigningAction.ConfirmCustodyProof, SigningAction.ContributeCustodyProof])(
    "does not add board fresh-login policy to trustee action %s",
    async (action) => {
        render(
            <SigningDialog
                open
                data={{...data, request: {...data.request, action}}}
                api={api}
                onClose={() => {}}
            />
        )
        fireEvent.click(screen.getByRole("button", {name: "signing.widget.continue"}))
        await waitFor(() => expect(document.querySelector('input[type="file"]')).not.toBeNull())
        expect(mockReauthenticate).not.toHaveBeenCalled()
    }
)

const chooseFile = async () => {
    fireEvent.click(screen.getByRole("button", {name: "signing.widget.continue"}))
    await waitFor(() => expect(document.querySelector('input[type="file"]')).not.toBeNull())
    const file = new File(["synthetic fixture"], "synthetic.p12")
    Object.defineProperty(file, "arrayBuffer", {value: async () => new ArrayBuffer(1)})
    fireEvent.change(document.querySelector('input[type="file"]')!, {target: {files: [file]}})
}
it("rechecks freshness before reading an already selected file", async () => {
    mockFresh = true
    render(<SigningDialog open data={data} api={api} onClose={() => {}} />)
    await chooseFile()
    mockFresh = false
    fireEvent.click(screen.getByRole("button", {name: "signing.dialog.certificate.open"}))
    await waitFor(() => expect(mockReauthenticate).toHaveBeenCalledTimes(1))
    expect(mockOpen).not.toHaveBeenCalled()
    expect(document.querySelector('input[type="file"]')).toBeNull()
})
it("discards opened private state and refuses approve when authentication expires during signing", async () => {
    mockFresh = true
    mockOpen.mockResolvedValue({
        choices: [
            {
                commonName: "Synthetic board",
                subject: "CN=board",
                issuer: "CN=issuer",
                notAfter: new Date("2030-01-01"),
                fingerprintSha256: "a".repeat(64),
                algorithm: "ecdsa-p256-sha256",
                chainPem: ["public fixture"],
                privateKey: {synthetic: true},
            },
        ],
        ambiguous: false,
    })
    mockSign.mockImplementation(async (guarded: ISigningApi) => {
        mockFresh = false
        return guarded.approve("request", {} as never)
    })
    render(<SigningDialog open data={data} api={api} onClose={() => {}} />)
    await chooseFile()
    fireEvent.click(screen.getByRole("button", {name: "signing.dialog.certificate.open"}))
    const sign = await screen.findByRole("button", {name: "signing.dialog.sign"})
    await waitFor(() => expect((sign as HTMLButtonElement).disabled).toBe(false))
    fireEvent.click(sign)
    await waitFor(() => expect(mockReauthenticate).toHaveBeenCalledTimes(1))
    expect(api.approve).not.toHaveBeenCalled()
    expect(document.querySelector('input[type="file"]')).toBeNull()
    expect(
        Object.keys(JSON.parse(sessionStorage.getItem(BOARD_SIGNING_RESUME_KEY)!)).sort()
    ).toEqual(
        [
            "action",
            "code",
            "eventId",
            "expiresAt",
            "payloadHash",
            "requestId",
            "tenantId",
            "userId",
        ].sort()
    )
})
