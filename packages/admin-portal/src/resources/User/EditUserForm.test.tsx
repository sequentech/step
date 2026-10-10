/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {EditUserForm} from "./EditUserForm"

jest.mock("@mui/x-data-grid", () => ({DataGrid: () => null}))

const mockReveal = jest.fn()
const mockNotify = jest.fn()
const mockPermissions = new Set<string>()
const mockRoles = {list_user_roles: []}
const mockMutation = jest.fn()
jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useLazyQuery: () => [mockReveal],
    useQuery: () => ({data: mockRoles}),
    useMutation: () => [mockMutation],
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    useRefresh: () => jest.fn(),
    useGetList: () => ({data: []}),
    SimpleForm: ({children}: React.PropsWithChildren) => children,
    SaveButton: () => null,
    BooleanInput: () => null,
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-a"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: (_super: boolean, _tenant: string, permission: string) =>
            mockPermissions.has(permission),
    }),
}))
jest.mock("@sequentech/ui-core", () => ({isUndefined: (value: unknown) => value === undefined}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({CustomAutocompleteArrayInput: () => null, ReviewChangesTable: () => null}),
    {virtual: true}
)
jest.mock("@/components/styles/FormStyles", () => ({
    FormStyles: {
        TextField: require("@mui/material").TextField,
        CheckboxControlLabel: require("@mui/material").FormControlLabel,
    },
}))
jest.mock("@/components/styles/PageHeaderStyles", () => ({
    PageHeaderStyles: {Wrapper: "div", Title: "h1", SubTitle: "p"},
}))
jest.mock("@/components/styles/ElectionHeaderStyles", () => ({
    ElectionHeaderStyles: {Title: "label"},
}))
jest.mock("@/components/styles/WizardStyles", () => ({WizardStyles: {ErrorMessage: "p"}}))
jest.mock("@/components/area/SelectArea", () => ({__esModule: true, default: () => null}))
jest.mock("./SelectActedTrustee", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/PhoneInput", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/IconTooltip", () => ({__esModule: true, default: () => null}))
jest.mock("./EditPassword", () => ({
    InputContainerStyle: "div",
    InputLabelStyle: "label",
    PasswordInputStyle: "input",
}))
jest.mock("./ListUsers", () => ({
    SUPPORT_MATERIALS_ACKNOWLEDGED: "support-materials-acknowledged",
    VOTED_CHANNEL: "voted-channel",
}))

const REVEAL = "usersAndRolesScreen.voters.secretAttribute.reveal"
const HIDE = "usersAndRolesScreen.voters.secretAttribute.hide"
const REVEAL_ERROR = "usersAndRolesScreen.voters.secretAttribute.revealError"
const props = {
    id: "voter-a",
    electionEventId: "event-a",
    rolesList: [],
    userAttributeGroups: [],
    userAttributes: [
        {name: "secret", display_name: "Secret", annotations: {"sequent.secret": true}},
    ],
    record: {id: "voter-a", enabled: true, attributes: {secret: ["redacted"]}},
}
const success = {
    data: {
        reveal_voter_secret_attribute: {attribute_name: "secret", values: ["synthetic plaintext"]},
    },
}
beforeEach(() => {
    jest.clearAllMocks()
    mockPermissions.clear()
    mockPermissions.add("voter-secret-attribute-read")
    mockReveal.mockReset().mockResolvedValue(success)
    mockMutation.mockImplementation(() => {
        throw new Error("Unexpected mutation")
    })
})
function editor() {
    return render(<EditUserForm {...props} />)
}
function reveal() {
    fireEvent.click(screen.getByRole("button", {name: REVEAL}))
}
function expectHidden() {
    expect(screen.getByLabelText("Secret")).toHaveValue("")
    expect(screen.getByLabelText("Secret")).toHaveAttribute("type", "password")
    expect(screen.queryByRole("button", {name: HIDE})).not.toBeInTheDocument()
    expect(mockMutation).not.toHaveBeenCalled()
}
it("reads the scoped secret once and erases unmodified plaintext when hidden", async () => {
    editor()
    reveal()
    await waitFor(() => expect(screen.getByLabelText("Secret")).toHaveValue("synthetic plaintext"))
    expect(mockReveal).toHaveBeenCalledWith({
        variables: {
            tenantId: "tenant-a",
            electionEventId: "event-a",
            userId: "voter-a",
            attributeName: "secret",
        },
    })
    fireEvent.click(screen.getByRole("button", {name: HIDE}))
    expectHidden()
    expect(mockNotify).not.toHaveBeenCalled()
})
it.each([
    ["resolved transport error", {error: new Error("transport unavailable")}],
    ["absent response payload", {data: {}}],
])("keeps plaintext hidden and reports a %s", async (_name, response) => {
    mockReveal.mockResolvedValueOnce(response)
    editor()
    reveal()
    await waitFor(() => expect(mockNotify).toHaveBeenCalledWith(REVEAL_ERROR, {type: "error"}))
    expectHidden()
    expect(screen.getByRole("button", {name: REVEAL})).toBeEnabled()
    reveal()
    await waitFor(() => expect(screen.getByLabelText("Secret")).toHaveValue("synthetic plaintext"))
    expect(mockReveal).toHaveBeenCalledTimes(2)
})
it("reports a rejected request and leaves the reveal control available", async () => {
    mockReveal.mockRejectedValueOnce(new Error("network unavailable"))
    editor()
    reveal()
    await waitFor(() => expect(mockNotify).toHaveBeenCalledWith(REVEAL_ERROR, {type: "error"}))
    expectHidden()
    expect(screen.getByRole("button", {name: REVEAL})).toBeEnabled()
})
it("accepts a valid response containing no stored values", async () => {
    mockReveal.mockResolvedValueOnce({
        data: {reveal_voter_secret_attribute: {attribute_name: "secret", values: []}},
    })
    editor()
    reveal()
    await waitFor(() => expect(screen.getByRole("button", {name: HIDE})).toBeEnabled())
    expect(screen.getByLabelText("Secret")).toHaveValue("")
    expect(mockNotify).not.toHaveBeenCalled()
})
it("discards plaintext returned after secret read permission is revoked", async () => {
    let finish: ((value: typeof success) => void) | undefined
    mockReveal.mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                finish = resolve
            })
    )
    const {rerender} = editor()
    reveal()
    expect(screen.getByRole("button", {name: REVEAL})).toBeDisabled()
    mockPermissions.delete("voter-secret-attribute-read")
    rerender(<EditUserForm {...props} />)
    await act(async () => {
        if (!finish) throw new Error("Reveal did not start")
        finish(success)
    })
    expectHidden()
    expect(screen.queryByRole("button", {name: REVEAL})).not.toBeInTheDocument()
    expect(mockNotify).not.toHaveBeenCalled()
})
