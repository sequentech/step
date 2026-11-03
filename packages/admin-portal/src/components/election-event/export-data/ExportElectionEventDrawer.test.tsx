/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ExportElectionEventDrawer} from "./ExportElectionEventDrawer"
import {IPermissions} from "@/types/keycloak"

const mockExport = jest.fn()
const mockAuthorized = jest.fn()
let mockTenantId: string | undefined = "tenant-1"

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [mockExport],
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
// The dialog's own rendering is not under test: its body and the two answers.
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({
            open,
            children,
            handleClose,
        }: {
            open: boolean
            children: React.ReactNode
            handleClose: (result: boolean) => void
        }) =>
            open ? (
                <div role="dialog">
                    {children}
                    <button onClick={() => handleClose(true)}>export</button>
                    <button onClick={() => handleClose(false)}>cancel</button>
                </div>
            ) : null,
    }),
    // Virtual: also resolvable where the workspace package isn't built.
    {virtual: true}
)
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [() => ({identifier: "widget-1"}), jest.fn(), jest.fn()],
}))
jest.mock("./PasswordDialog", () => ({
    PasswordDialog: () => <div data-testid="password-dialog" />,
    DecryptHelp: () => null,
}))
jest.mock("@/resources/Reports/ListReports", () => ({decryptionCommand: "decrypt"}))
jest.mock("@/providers/TenantContextProvider", () => ({
    useTenantStore: () => [mockTenantId],
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: (...args: unknown[]) => mockAuthorized(...args),
    }),
}))

const ENCRYPT = "electionEventScreen.export.encryptWithPassword"
const SECRETS = "electionEventScreen.export.integrationSecrets"
const NOTE = "electionEventScreen.export.integrationSecretsNote"

const renderDrawer = () =>
    render(
        <ExportElectionEventDrawer
            electionEventId="event-1"
            openExport={true}
            setOpenExport={jest.fn()}
            setLoadingExport={jest.fn()}
        />
    )

const sentOptions = () => mockExport.mock.calls[0][0].variables.exportConfigurations

beforeEach(() => {
    jest.clearAllMocks()
    mockTenantId = "tenant-1"
    mockAuthorized.mockReturnValue(true)
    mockExport.mockResolvedValue({
        data: {
            export_election_event: {
                password: "synthetic-password",
                document_id: "document-1",
                task_execution: {id: "task-1"},
            },
        },
    })
})

test("the integration-secrets option is offered only with integration-secret-read", () => {
    renderDrawer()
    expect(screen.getByRole("checkbox", {name: SECRETS})).not.toBeChecked()
    expect(mockAuthorized).toHaveBeenCalledWith(
        true,
        "tenant-1",
        IPermissions.INTEGRATION_SECRET_READ
    )
})

test("without the permission or a tenant the option is withheld", () => {
    mockAuthorized.mockReturnValue(false)
    const {unmount} = renderDrawer()
    expect(screen.getByRole("checkbox", {name: ENCRYPT})).toBeInTheDocument()
    expect(screen.queryByRole("checkbox", {name: SECRETS})).toBeNull()
    unmount()

    mockAuthorized.mockReturnValue(true)
    mockTenantId = undefined
    renderDrawer()
    expect(screen.queryByRole("checkbox", {name: SECRETS})).toBeNull()
})

test("choosing integration secrets forces password encryption and warns", () => {
    renderDrawer()
    expect(screen.queryByText(NOTE)).toBeNull()
    fireEvent.click(screen.getByRole("checkbox", {name: SECRETS}))
    expect(screen.getByRole("checkbox", {name: SECRETS})).toBeChecked()
    expect(screen.getByRole("checkbox", {name: ENCRYPT})).toBeChecked()
    expect(screen.getByText(NOTE)).toBeInTheDocument()
})

test("dropping encryption drops the integration secrets with it", () => {
    renderDrawer()
    fireEvent.click(screen.getByRole("checkbox", {name: SECRETS}))
    fireEvent.click(screen.getByRole("checkbox", {name: ENCRYPT}))
    expect(screen.getByRole("checkbox", {name: ENCRYPT})).not.toBeChecked()
    expect(screen.getByRole("checkbox", {name: SECRETS})).not.toBeChecked()
    expect(screen.queryByText(NOTE)).toBeNull()
    // Unchecking the option itself keeps the chosen encryption.
    fireEvent.click(screen.getByRole("checkbox", {name: SECRETS}))
    fireEvent.click(screen.getByRole("checkbox", {name: SECRETS}))
    expect(screen.getByRole("checkbox", {name: SECRETS})).not.toBeChecked()
    expect(screen.getByRole("checkbox", {name: ENCRYPT})).toBeChecked()
})

test("an export with integration secrets requests them encrypted", async () => {
    renderDrawer()
    fireEvent.click(screen.getByRole("checkbox", {name: SECRETS}))
    fireEvent.click(screen.getByRole("button", {name: "export"}))
    await waitFor(() => expect(mockExport).toHaveBeenCalledTimes(1))
    expect(sentOptions()).toMatchObject({
        is_encrypted: true,
        encrypt_with_password: true,
        include_integration_secrets: true,
    })
    expect(await screen.findByTestId("password-dialog")).toBeInTheDocument()
})

test("an ordinary export never asks for integration secrets", async () => {
    renderDrawer()
    // Chosen, then unchosen: the request is the ordinary one.
    fireEvent.click(screen.getByRole("checkbox", {name: SECRETS}))
    fireEvent.click(screen.getByRole("checkbox", {name: SECRETS}))
    fireEvent.click(screen.getByRole("checkbox", {name: ENCRYPT}))
    fireEvent.click(screen.getByRole("button", {name: "export"}))
    await waitFor(() => expect(mockExport).toHaveBeenCalledTimes(1))
    expect(sentOptions()).toMatchObject({
        is_encrypted: false,
        encrypt_with_password: false,
        include_integration_secrets: false,
    })
})
