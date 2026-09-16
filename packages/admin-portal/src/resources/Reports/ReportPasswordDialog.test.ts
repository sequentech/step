/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, fireEvent} from "@testing-library/react"
import {useQuery} from "@apollo/client"
import {ReportPasswordDialog} from "./ReportPasswordDialog"

let mockAllowed = true
jest.mock("@apollo/client", () => ({useQuery: jest.fn(), gql: jest.fn()}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => mockAllowed}),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (s: string) => s})}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({children, handleClose}: any) =>
            require("react").createElement(
                "div",
                {},
                children,
                require("react").createElement("button", {onClick: handleClose}, "Close")
            ),
    }),
    {virtual: true}
)
jest.mock("@/components/election-event/export-data/PasswordDialog", () => ({
    PasswordDialog: ({password, onClose, children}: any) =>
        require("react").createElement(
            "div",
            {},
            password,
            children,
            require("react").createElement("button", {onClick: onClose}, "Close")
        ),
    DecryptHelp: ({decryptionCommand}: any) =>
        require("react").createElement("span", {}, decryptionCommand),
}))

beforeEach(() => {
    mockAllowed = true
    jest.clearAllMocks()
    ;(useQuery as jest.Mock).mockReturnValue({
        data: {get_document_password: {password: "test-only-password"}},
    })
})

it("uses the existing dialog and no-cache permission-checked password query", () => {
    const close = jest.fn()
    render(
        React.createElement(ReportPasswordDialog, {
            documentId: "document",
            access: {password_secret_id: "vault-id", voter_secret_attributes: true},
            onClose: close,
        })
    )
    expect(screen.getByText("test-only-password")).toBeTruthy()
    expect(useQuery).toHaveBeenCalledWith(
        undefined,
        expect.objectContaining({
            variables: {documentId: "document"},
            fetchPolicy: "no-cache",
            skip: false,
            context: {headers: {"x-hasura-role": "document-password-read"}},
        })
    )
    fireEvent.click(screen.getByText("Close"))
    expect(close).toHaveBeenCalledTimes(1)
})

it("hides a previously returned password immediately when permission is lost", () => {
    const props = {
        documentId: "document",
        access: {password_secret_id: "vault-id"},
        onClose: jest.fn(),
    }
    const view = render(React.createElement(ReportPasswordDialog, props))
    mockAllowed = false
    view.rerender(React.createElement(ReportPasswordDialog, props))
    expect(screen.queryByText("test-only-password")).toBeNull()
    expect(screen.getByText("tasksScreen.documentAccess.passwordError")).toBeTruthy()
})

it("keeps instructions available for legacy reports without a saved password", () => {
    ;(useQuery as jest.Mock).mockReturnValue({})
    render(React.createElement(ReportPasswordDialog, {documentId: "legacy", onClose: jest.fn()}))
    expect(useQuery).toHaveBeenCalledWith(undefined, expect.objectContaining({skip: true}))
    expect(screen.getByText(/openssl enc/)).toBeTruthy()
})

it("shows a recoverable error without exposing a failed query payload", () => {
    ;(useQuery as jest.Mock).mockReturnValue({error: new Error("private backend detail")})
    render(
        React.createElement(ReportPasswordDialog, {
            documentId: "document",
            access: {password_secret_id: "vault-id"},
            onClose: jest.fn(),
        })
    )
    expect(screen.getByText("tasksScreen.documentAccess.passwordError")).toBeTruthy()
    expect(screen.queryByText("private backend detail")).toBeNull()
})
