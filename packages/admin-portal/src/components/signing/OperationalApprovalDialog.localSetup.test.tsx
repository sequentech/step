/**
 * @jest-environment jsdom
 * @jest-environment-options {"url":"http://localhost:3002/"}
 */
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import type {DocumentNode, OperationDefinitionNode} from "graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {validatePanel, type ISigningApi} from "@/lib/signing/api"
import {canonicalJson, signedView} from "@/lib/signing/request"
import {OperationalApprovalDialog} from "./OperationalApprovalDialog"

jest.mock("@/lib/signing/localInitializeProfile.json", () => ({profile: "felix3-local-initialize"}))
jest.mock("@/providers/AuthContextProvider", () => ({AuthContext: require("react").createContext(null)}))
jest.mock("@/providers/SettingsContextProvider", () => ({SettingsContext: require("react").createContext(null)}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string, options?: {defaultValue?: string}) => options?.defaultValue ?? key}),
}))
jest.mock("./SigningSubject", () => ({SigningSubject: () => null, SigningSubjectVariant: {Dialog: "dialog"}}))
jest.mock("./useSignedView", () => ({useSignedView: jest.fn()}))
jest.mock("@apollo/client", () => ({...jest.requireActual("@apollo/client"), useApolloClient: () => mockApollo}))

const mockApollo = {query: jest.fn(), mutate: jest.fn()}
const id = (i: number) => "00000000-0000-4000-8000-" + String(i).padStart(12, "0")
const scope = {tenant: id(1), event: id(2), request: id(3), official: id(4)}
const attempt = id(5)
const post = id(6)
const document = {
    domain: "step-signing/v1", action: "initialize-voting", tenant_id: scope.tenant,
    election_event_id: scope.event, election_id: post, request_id: scope.request,
    code: "INIT", subject: {publication_id: id(7)},
}
const canonical = canonicalJson(document)
const digest: string = require("node:crypto").createHash("sha256").update(canonical).digest("hex")
const data = validatePanel({
    request: {...document, id: scope.request, required: 2, canonical_payload: canonical,
        payload_sha256: digest, expires_at: null, document_sha256: null, status: "waiting"},
    rule: {requester_signing: "not-allowed"}, operational_signing_mode: "hsm-required",
    count: 0, document_url: null,
    signers: [{user_id: scope.official, username: "test-official", display_name: "Test official", signed_at: null}],
})
const auth = {isAuthenticated: true, tenantId: scope.tenant, userId: scope.official,
    isGoldUser: jest.fn(), reauthWithGold: jest.fn()} as unknown as React.ContextType<typeof AuthContext>
const settings = {globalSettings: {
    KEYCLOAK_URL: "http://localhost:18090/", HASURA_URL: "http://localhost:18080/v1/graphql",
    ONLINE_VOTING_CLIENT_ID: "ordinary-admin", FELIX3_LOCAL_INITIALIZE: {
        profile: "felix3-local-initialize", deviceProfile: "softhsm-test",
        issuer: "http://localhost:18090/realms/tenant-" + scope.tenant,
        callback: "http://localhost:3002/operational-signing/return", graphql: "http://localhost:18080/v1/graphql",
        portalClient: "ordinary-admin", approvalClient: "approval-client", setupClient: "felix3-local-initialize-setup",
    },
}} as unknown as React.ContextType<typeof SettingsContext>
const name = (op: DocumentNode) => (op.definitions[0] as OperationDefinitionNode).name?.value

function open(phase: string, revision: number | null = null) {
    let current = {scope, phase, device_profile: "softhsm-test", attempt: revision !== null || phase === "waiting-for-setup-gold" ? attempt : null,
        delivery_revision: revision}
    mockApollo.query.mockImplementation(async ({query}) => {
        if (name(query) === "OperationalInitializeStatus")
            return {data: {operationalInitializeStatus: {scope, phase: "unavailable", device_profile: "softhsm-test"}}}
        if (name(query) !== "LocalInitializeSetupStatus") throw new Error("Unexpected query")
        return {data: {operationalInitializeSelfSetupStatus: current}}
    })
    mockApollo.mutate.mockImplementation(async ({mutation}) => {
        if (name(mutation) !== "LocalInitializeSetupRegister") throw new Error("Unexpected mutation")
        current = {scope, phase: "setup-pending", device_profile: "softhsm-test", attempt: null, delivery_revision: null}
        return {data: {operationalInitializeSelfSetupRegister: current}}
    })
    const api = {getRequest: jest.fn().mockResolvedValue(data)} as unknown as ISigningApi
    const element = (eligible: boolean) => (
        <AuthContext.Provider value={auth}>
            <SettingsContext.Provider value={settings}>
                <OperationalApprovalDialog data={data} api={api} eligible={eligible} onClose={jest.fn()} onRefresh={jest.fn()} />
            </SettingsContext.Provider>
        </AuthContext.Provider>
    )
    const view = render(element(true))
    return {view, changeAttempt: () => { current = {...current, attempt: id(9)} }}
}

beforeEach(() => {
    jest.clearAllMocks()
    require("./useSignedView").useSignedView.mockReturnValue({view: signedView(data), verified: true, problem: null})
})

it("offers Start Setup on first use and only a sign-in continuation for an opened attempt", async () => {
    const first = open("setup-available")
    await screen.findByRole("button", {name: "Start Setup"})
    expect(screen.queryByRole("button", {name: "Register my signing key"})).toBeNull()
    first.view.unmount()
    open("waiting-for-setup-gold")
    await screen.findByRole("button", {name: "Continue setup sign-in"})
    expect(mockApollo.mutate).not.toHaveBeenCalled()
    expect(auth.reauthWithGold).not.toHaveBeenCalled()
})

it("reopens the confirmed attempt, polls read-only, and sends exactly one reviewed registration click", async () => {
    jest.useFakeTimers()
    try {
        open("setup-confirmation-required", 0)
        const button = await screen.findByRole("button", {name: "Register my signing key"})
        expect(screen.getByRole("status").textContent).toMatch(/Identity verified for this request/)
        const reads = mockApollo.query.mock.calls.length
        await act(async () => { await jest.advanceTimersByTimeAsync(2600) })
        expect(mockApollo.query.mock.calls.length).toBeGreaterThan(reads)
        expect(mockApollo.mutate).not.toHaveBeenCalled()
        fireEvent.click(button)
        fireEvent.click(button)
        await waitFor(() => expect(screen.getByRole("status").textContent).toMatch(/Setup is pending/))
        expect(mockApollo.mutate).toHaveBeenCalledTimes(1)
        expect(mockApollo.mutate.mock.calls[0][0].variables).toEqual({
            request: scope.request, digest, attempt, revision: 0,
        })
        expect(screen.queryByRole("button", {name: "Register my signing key"})).toBeNull()
    } finally {
        jest.useRealTimers()
    }
})

it("a Start Setup click requests only a setup redirect and refuses the approval client's redirect", async () => {
    open("setup-available")
    const button = await screen.findByRole("button", {name: "Start Setup"})
    mockApollo.mutate.mockResolvedValueOnce({data: {operationalInitializeSelfSetupStart: {
        attempt, page: {path: "event-show", panel: "signatures-requests"},
        redirect: "http://localhost:18090/realms/tenant-" + scope.tenant +
            "/protocol/openid-connect/auth?client_id=approval-client&request_uri=urn%3Aietf%3Aparams%3Aoauth%3Arequest_uri%3Aone",
    }}})
    fireEvent.click(button)
    await screen.findByText("signing.operational.error")
    expect(mockApollo.mutate).toHaveBeenCalledTimes(1)
    const call = mockApollo.mutate.mock.calls[0][0]
    expect(name(call.mutation)).toBe("LocalInitializeSetupStart")
    expect(call.variables).toEqual({request: scope.request, digest, page: {path: "event-show", panel: "signatures-requests"},
        attempt: undefined})
    expect(window.sessionStorage.length).toBe(0)
})

it("refuses a stale redelivery click when the original status reference changed", async () => {
    const {changeAttempt} = open("setup-retry-available", 1)
    const button = await screen.findByRole("button", {name: "Register my signing key"})
    changeAttempt()
    fireEvent.click(button)
    await screen.findByText("signing.operational.error")
    expect(mockApollo.mutate).not.toHaveBeenCalled()
})
