/**
 * @jest-environment jsdom
 * @jest-environment-options {"url":"http://localhost:3002/"}
 */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {createHash, webcrypto} from "node:crypto"
import {TextEncoder} from "node:util"
import {AuthContext} from "@/providers/AuthContextProvider"
import {validatePanel, type ISigningApi, type ISigningPanelData} from "@/lib/signing/api"
import {canonicalJson} from "@/lib/signing/request"
import {SigningSignerStatus} from "@/lib/signing/types"
import {SigningRequestPanel} from "./SigningRequestPanel"

jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext(null),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => [tenantId]}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@/hooks/useZonedFormat", () => ({useZonedFormat: () => ({zone: "UTC"})}))
jest.mock("./ConfigurationAuthorizes", () => ({LIFECYCLE_SUBJECT_KEYS: []}))
jest.mock("./SigningDialog", () => ({SigningDialog: () => <p>Legacy signing dialog</p>}))
jest.mock("./OperationalApprovalDialog", () => ({OperationalApprovalDialog: () => null}))

const tenantId = "01000000-0000-4000-8000-000000000001"
const actor = "02000000-0000-4000-8000-000000000002"
const subject = {publication_id: "03000000-0000-4000-8000-000000000003"}
const payload = {
    domain: "step-signing/v1",
    request_id: "04000000-0000-4000-8000-000000000004",
    tenant_id: tenantId,
    election_event_id: "05000000-0000-4000-8000-000000000005",
    election_id: "06000000-0000-4000-8000-000000000006",
    action: "initialize-voting",
    code: "ABCD-1234",
    subject,
}
const canonical = canonicalJson(payload)
const panel = validatePanel({
    request: {
        ...payload,
        id: payload.request_id,
        expires_at: null,
        status: "waiting",
        required: 2,
        canonical_payload: canonical,
        payload_sha256: createHash("sha256").update(canonical).digest("hex"),
    },
    operational_signing_mode: "hsm-demo-raw",
    rule: {requester_signing: "allowed"},
    count: 0,
    signers: [{user_id: actor, username: "member", display_name: "Member", signed_at: null}],
})

beforeAll(() => {
    Object.defineProperty(window.crypto, "subtle", {configurable: true, value: webcrypto.subtle})
    Object.defineProperty(globalThis, "TextEncoder", {configurable: true, value: TextEncoder})
})

function setup() {
    const getRequest = jest.fn(async () => panel)
    const approveDirectInitialize = jest.fn(async () => {})
    const api = {getRequest, approveDirectInitialize} as unknown as ISigningApi
    const drawer = (canSign: boolean, userId = actor) => (
        <AuthContext.Provider
            value={
                {
                    userId,
                    tenantId,
                    isAuthenticated: true,
                    isAuthorized: () => canSign,
                } as unknown as React.ContextType<typeof AuthContext>
            }
        >
            <SigningRequestPanel
                open
                autoSign
                requestId={panel.request.id}
                api={api}
                onClose={jest.fn()}
            />
        </AuthContext.Provider>
    )
    const {rerender} = render(drawer(true))
    return {
        getRequest,
        approveDirectInitialize,
        show: (allowed: boolean, user?: string) => rerender(drawer(allowed, user)),
    }
}

it.each(["permission", "account", "navigation"])(
    "rechecks the current %s after the fresh request read",
    async (change) => {
        const {getRequest, approveDirectInitialize, show} = setup()
        const dialog = await screen.findByRole("dialog", {name: "signing.dialog.title"})
        const sign = within(dialog).getByRole("button", {name: "signing.dialog.sign"})
        await waitFor(() => expect((sign as HTMLButtonElement).disabled).toBe(false))
        let finish!: (value: ISigningPanelData) => void
        getRequest.mockImplementationOnce(
            () =>
                new Promise((resolve) => {
                    finish = resolve
                })
        )
        fireEvent.click(sign)
        await waitFor(() => expect(getRequest).toHaveBeenCalledTimes(2))
        if (change === "permission") show(false)
        if (change === "account") show(true, "07000000-0000-4000-8000-000000000007")
        if (change === "navigation") window.history.pushState({}, "", "/another-request")
        await act(async () => finish(panel))
        expect(approveDirectInitialize).not.toHaveBeenCalled()
        window.history.replaceState({}, "", "/")
    }
)

it("cannot switch an open HSM review to client certificate signing when the profile changes", async () => {
    const {getRequest, approveDirectInitialize} = setup()
    const dialog = await screen.findByRole("dialog", {name: "signing.dialog.title"})
    const sign = within(dialog).getByRole("button", {name: "signing.dialog.sign"})
    await waitFor(() => expect((sign as HTMLButtonElement).disabled).toBe(false))
    getRequest.mockResolvedValue({...panel, operational_signing_mode: "legacy-p12"})
    fireEvent.click(sign)
    await waitFor(() => expect(getRequest).toHaveBeenCalledTimes(2))
    expect(approveDirectInitialize).not.toHaveBeenCalled()
    expect(screen.queryByText("Legacy signing dialog")).toBeNull()
})

it.each<[string, ISigningPanelData]>([
    ["configuration revision", {...panel, request: {...panel.request, config_revision: "updated"}}],
    [
        "document reference",
        {...panel, request: {...panel.request, document_id: subject.publication_id}},
    ],
    [
        "signed member without a receipt time",
        {...panel, signers: [{...panel.signers[0], status: SigningSignerStatus.Signed}]},
    ],
])("blocks a refreshed %s without signing", async (_, fresh) => {
    const {getRequest, approveDirectInitialize} = setup()
    const dialog = await screen.findByRole("dialog", {name: "signing.dialog.title"})
    const sign = within(dialog).getByRole("button", {name: "signing.dialog.sign"})
    await waitFor(() => expect((sign as HTMLButtonElement).disabled).toBe(false))
    getRequest.mockResolvedValue(fresh)
    fireEvent.click(sign)
    await within(dialog).findByText("signing.operational.phase.unavailable")
    expect(approveDirectInitialize).not.toHaveBeenCalled()
    expect(within(dialog).queryByText("signing.operational.phase.approval-recorded")).toBeNull()
})
