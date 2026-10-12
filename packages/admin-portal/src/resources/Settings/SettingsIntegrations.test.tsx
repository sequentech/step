/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {SettingsIntegrations} from "./SettingsIntegrations"

const mockSetKey = jest.fn()
const mockNotify = jest.fn()
const mockSave = jest.fn()
const mockRefetch = jest.fn()
const mockController = jest.fn()
const mockIsAuthorized = jest.fn()

jest.mock("@apollo/client", () => ({gql: jest.fn(() => ({})), useMutation: () => [mockSetKey]}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-1"]}))
jest.mock("@/providers/AuthContextProvider", () => {
    const {createContext} = require("react")
    return {
        AuthContext: createContext({
            tenantId: "tenant-1",
            isAuthorized: (...args: unknown[]) => mockIsAuthorized(...args),
        }),
    }
})
jest.mock("react-admin", () => ({
    useEditController: () => mockController(),
    useNotify: () => mockNotify,
    SimpleForm: ({toolbar, children}: {toolbar: React.ReactNode; children: React.ReactNode}) => (
        <form>
            {children}
            {toolbar}
        </form>
    ),
    Toolbar: ({children}: {children: React.ReactNode}) => <div>{children}</div>,
    SaveButton: ({onClick, disabled}: {onClick: () => void; disabled: boolean}) => (
        <button type="button" onClick={onClick} disabled={disabled}>
            save
        </button>
    ),
    TextInput: ({
        label,
        onChange,
    }: {
        label: string
        onChange: (event: React.ChangeEvent<HTMLInputElement>) => void
    }) => <input aria-label={label} onChange={onChange} />,
}))

const KEY = {type: "service_account", client_email: "meet@project.iam.gserviceaccount.com"}

const type = (label: string, value: string) =>
    fireEvent.change(screen.getByLabelText(label), {target: {value}})

describe("SettingsIntegrations", () => {
    beforeEach(() => {
        jest.resetAllMocks()
        mockIsAuthorized.mockReturnValue(true)
        mockController.mockReturnValue({
            record: {settings: {gapi_key: KEY, gapi_email: "old@example.org", other: 1}},
            save: mockSave,
            isLoading: false,
            refetch: mockRefetch,
        })
        mockSetKey.mockResolvedValue({data: {}})
    })

    it("renders nothing while the tenant loads", () => {
        mockController.mockReturnValue({isLoading: true})
        const {container} = render(<SettingsIntegrations />)
        expect(container).toBeEmptyDOMElement()
    })

    it("offers no save button without the tenant write permission", () => {
        mockIsAuthorized.mockReturnValue(false)
        render(<SettingsIntegrations />)
        expect(screen.queryByRole("button", {name: "save"})).not.toBeInTheDocument()
    })

    it("keeps save disabled until a field changes", () => {
        render(<SettingsIntegrations />)
        expect(screen.getByRole("button", {name: "save"})).toBeDisabled()
        type("integrationsScreen.common.gapiEmail", "new@example.org")
        expect(screen.getByRole("button", {name: "save"})).toBeEnabled()
    })

    it("sends a changed key to the vault and not to the tenant settings", async () => {
        render(<SettingsIntegrations />)
        type("integrationsScreen.common.gapiKey", JSON.stringify(KEY))
        fireEvent.click(screen.getByRole("button", {name: "save"}))

        await waitFor(() => expect(mockRefetch).toHaveBeenCalled())
        expect(mockSetKey).toHaveBeenCalledWith({variables: {serviceAccountKey: KEY}})
        expect(mockNotify).toHaveBeenCalledWith("integrationsScreen.common.gapiKeySaved", {
            type: "success",
        })
        expect(mockSave).not.toHaveBeenCalled()
        await waitFor(() => expect(screen.getByRole("button", {name: "save"})).toBeDisabled())
    })

    it("saves the email without the key left in the settings next to a new key", async () => {
        render(<SettingsIntegrations />)
        type("integrationsScreen.common.gapiKey", JSON.stringify(KEY))
        type("integrationsScreen.common.gapiEmail", "new@example.org")
        fireEvent.click(screen.getByRole("button", {name: "save"}))

        await waitFor(() => expect(mockSave).toHaveBeenCalled())
        expect(mockSave).toHaveBeenCalledWith({
            settings: {gapi_email: "new@example.org", other: 1},
        })
        expect(mockRefetch).not.toHaveBeenCalled()
    })

    it("keeps the stored settings key when only the email changes", async () => {
        render(<SettingsIntegrations />)
        type("integrationsScreen.common.gapiEmail", "new@example.org")
        fireEvent.click(screen.getByRole("button", {name: "save"}))

        await waitFor(() => expect(mockSave).toHaveBeenCalled())
        expect(mockSetKey).not.toHaveBeenCalled()
        expect(mockSave).toHaveBeenCalledWith({
            settings: {gapi_key: KEY, gapi_email: "new@example.org", other: 1},
        })
    })

    it("clears the email when it is emptied", async () => {
        render(<SettingsIntegrations />)
        type("integrationsScreen.common.gapiEmail", "  ")
        fireEvent.click(screen.getByRole("button", {name: "save"}))

        await waitFor(() => expect(mockSave).toHaveBeenCalled())
        expect(mockSave.mock.calls[0][0].settings.gapi_email).toBeUndefined()
    })

    it("reports a key the server refuses and saves nothing else", async () => {
        mockSetKey.mockRejectedValue(new Error("Token URI not allowed"))
        render(<SettingsIntegrations />)
        type("integrationsScreen.common.gapiKey", JSON.stringify(KEY))
        type("integrationsScreen.common.gapiEmail", "new@example.org")
        fireEvent.click(screen.getByRole("button", {name: "save"}))

        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("integrationsScreen.errors.saveGapiKey", {
                type: "error",
            })
        )
        expect(mockSave).not.toHaveBeenCalled()
        expect(mockRefetch).not.toHaveBeenCalled()
    })

    it("rejects a key that is not a JSON object", () => {
        render(<SettingsIntegrations />)
        for (const value of ["{not json", "5"]) {
            mockNotify.mockClear()
            type("integrationsScreen.common.gapiKey", value)
            expect(mockNotify).toHaveBeenCalledWith("integrationsScreen.errors.invalidGapiKey", {
                type: "error",
            })
        }
        expect(screen.getByRole("button", {name: "save"})).toBeDisabled()
    })

    it("does not store a key that was emptied", async () => {
        render(<SettingsIntegrations />)
        type("integrationsScreen.common.gapiKey", JSON.stringify(KEY))
        type("integrationsScreen.common.gapiKey", "")
        type("integrationsScreen.common.gapiEmail", "new@example.org")
        fireEvent.click(screen.getByRole("button", {name: "save"}))

        await waitFor(() => expect(mockSave).toHaveBeenCalled())
        expect(mockSetKey).not.toHaveBeenCalled()
        expect(mockSave).toHaveBeenCalledWith({
            settings: {gapi_key: KEY, gapi_email: "new@example.org", other: 1},
        })
    })
})
