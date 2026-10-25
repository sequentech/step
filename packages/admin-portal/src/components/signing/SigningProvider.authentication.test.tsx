/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import {SigningProvider} from "./SigningProvider"
import {saveBoardSigningResume, BOARD_SIGNING_RESUME_KEY} from "@/lib/boardAuthentication"
import {SigningAction, type ISigningRequest} from "@/lib/signing/types"
import type {ISigningApi} from "@/lib/signing/api"
let mockFresh = true
let mockUser = "board1"
let mockTenant = "tenant"
const mockClient = {}
const mockPanel = jest.fn()
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useApolloClient: () => mockClient,
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthenticated: true,
        get userId() {
            return mockUser
        },
        isAuthorized: () => true,
        isFreshBoardAuthentication: () => mockFresh,
    }),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => [mockTenant]}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (x: string) => x})}))
jest.mock("./SigningRequestPanel", () => ({
    SigningRequestPanel: (p: {requestId: string; autoSign: boolean}) => {
        mockPanel(p)
        return require("react").createElement(
            "div",
            {"data-testid": "resumed-request"},
            p.requestId
        )
    },
}))
const request = {
    id: "request",
    tenant_id: "tenant",
    election_event_id: "event",
    action: SigningAction.ApproveConfiguration,
    payload_sha256: "a".repeat(64),
    code: "ABCD-EFGH",
    expires_at: null,
} as ISigningRequest
const api = {approve: jest.fn()} as unknown as ISigningApi
beforeEach(() => {
    sessionStorage.clear()
    mockFresh = true
    mockUser = "board1"
    mockTenant = "tenant"
    mockPanel.mockClear()
    saveBoardSigningResume(sessionStorage, request, "board1")
})
it("reopens only the same public request in review mode after fresh external login", async () => {
    render(<SigningProvider api={api} />)
    expect((await screen.findByTestId("resumed-request")).textContent).toBe("request")
    expect(mockPanel.mock.calls[0][0]).toMatchObject({
        requestId: "request",
        autoSign: true,
        expectedEventId: "event",
        authenticationBinding: {
            userId: "board1",
            requestId: "request",
            payloadHash: "a".repeat(64),
            code: "ABCD-EFGH",
            action: SigningAction.ApproveConfiguration,
        },
    })
    expect(sessionStorage.getItem(BOARD_SIGNING_RESUME_KEY)).toBeNull()
    expect(api.approve).not.toHaveBeenCalled()
})
it("retains the request and shows failure without reopening or looping after a nonfresh return", async () => {
    mockFresh = false
    render(<SigningProvider api={api} />)
    await screen.findByRole("alert")
    expect(mockPanel).not.toHaveBeenCalled()
    expect(sessionStorage.getItem(BOARD_SIGNING_RESUME_KEY)).not.toBeNull()
    expect(api.approve).not.toHaveBeenCalled()
})
it.each(["person", "tenant"])(
    "refuses a switched %s on the authentication return",
    async (kind) => {
        if (kind === "person") mockUser = "board2"
        else mockTenant = "other"
        render(<SigningProvider api={api} />)
        await waitFor(() => expect(screen.getByRole("alert")).toBeTruthy())
        expect(mockPanel).not.toHaveBeenCalled()
        expect(sessionStorage.getItem(BOARD_SIGNING_RESUME_KEY)).toBeNull()
        expect(api.approve).not.toHaveBeenCalled()
    }
)
