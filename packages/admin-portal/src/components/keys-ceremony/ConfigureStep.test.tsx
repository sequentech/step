/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {cleanup, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {ConfigureStep} from "./ConfigureStep"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"
import {LIST_KEYS_CEREMONY} from "@/queries/ListKeysCeremonies"

const mockTrustees = [
    {id: "b", name: "trustee-b"},
    {id: "a", name: "trustee-a"},
]
const mockCeremony = {id: "created-ceremony", tenant_id: "tenant", election_event_id: "event"}
const mockCreate = jest.fn()
const mockQuery = jest.fn()
const mockUseQuery = jest.fn()
const mockGenericList = jest.fn()
const mockGenericOne = jest.fn()
const mockNotify = jest.fn()
const mockRefresh = jest.fn()
const mockOpen = jest.fn()
const mockSetCeremony = jest.fn()
const mockTranslate = (key: string) => key
const mockDiv = ({children}: {children?: React.ReactNode}) => <div>{children}</div>

jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useMutation: () => [mockCreate],
    useQuery: (...args: unknown[]) => mockUseQuery(...args),
    useApolloClient: () => ({query: mockQuery}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: mockTranslate, i18n: {dir: () => "ltr", language: "en"}}),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/hooks/useAliasRenderer", () => ({
    useAliasRenderer: () => (record: {name?: string}) => record.name,
}))
jest.mock("@sequentech/ui-core", () => ({
    isNull: (value: unknown) => value == null,
    EElectionEventCeremoniesPolicy: {AUTOMATED_CEREMONIES: "automated"},
}))
jest.mock("react-admin", () => ({
    useGetList: (...args: unknown[]) => mockGenericList(...args),
    useGetOne: (...args: unknown[]) => mockGenericOne(...args),
    useNotify: () => mockNotify,
    useRefresh: () => mockRefresh,
    useTranslate: () => mockTranslate,
    SimpleForm: ({
        children,
        toolbar,
        onSubmit,
    }: {
        children: React.ReactNode
        toolbar: React.ReactNode
        onSubmit: (value: unknown) => void
    }) => (
        <form
            onSubmit={(e) => {
                e.preventDefault()
                onSubmit({
                    threshold: 2,
                    trusteeNames: ["trustee-a", "trustee-b"],
                    isAutomatic: false,
                })
            }}
        >
            {children}
            {toolbar}
        </form>
    ),
    TextInput: () => <input aria-label="threshold" />,
    CheckboxGroupInput: ({choices}: {choices: {id: string; name: string}[]}) => (
        <ul aria-label="trustees">
            {choices.map((choice) => (
                <li key={choice.id}>{choice.name}</li>
            ))}
        </ul>
    ),
    ReferenceInput: ({children}: {children: React.ReactNode}) => <div>{children}</div>,
    AutocompleteInput: () => null,
    BooleanInput: () => null,
}))
jest.mock("@/components/styles/WizardStyles", () => ({
    WizardStyles: {
        ContentBox: (props: {children?: React.ReactNode}) => mockDiv(props),
        Toolbar: (props: {children?: React.ReactNode}) => mockDiv(props),
        MainContent: (props: {children?: React.ReactNode}) => mockDiv(props),
        StepHeader: (props: {children?: React.ReactNode}) => mockDiv(props),
        BackButton: ({children}: {children: React.ReactNode}) => (
            <button type="button">{children}</button>
        ),
        CreateButton: ({label, disabled}: {label: string; disabled?: boolean}) => (
            <button type="submit" disabled={disabled}>
                {label}
            </button>
        ),
        ErrorMessage: ({children}: {children: React.ReactNode}) => (
            <div role="alert">{children}</div>
        ),
    },
}))
jest.mock("@sequentech/ui-essentials", () => ({
    Dialog: ({
        open,
        ok,
        handleClose,
    }: {
        open: boolean
        ok: string
        handleClose: (result: boolean) => void
    }) =>
        open ? (
            <div role="dialog">
                <button onClick={() => handleClose(true)}>{ok}</button>
            </div>
        ) : null,
}))

const event = {id: "event", tenant_id: "tenant", presentation: {}} as Sequent_Backend_Election_Event
const renderStep = () =>
    render(
        <ConfigureStep
            currentCeremony={null}
            setCurrentCeremony={mockSetCeremony}
            electionEvent={event}
            openCeremonyStep={mockOpen}
            goBack={jest.fn()}
        />
    )
const create = async () => {
    fireEvent.click(screen.getByRole("button", {name: "keysGeneration.configureStep.create"}))
    fireEvent.click(
        await screen.findByRole("button", {name: "keysGeneration.configureStep.confirmdDialog.ok"})
    )
}
beforeEach(() => {
    jest.clearAllMocks()
    mockCreate.mockResolvedValue({
        data: {create_keys_ceremony: {keys_ceremony_id: mockCeremony.id}},
    })
    mockQuery.mockResolvedValue({data: {list_keys_ceremony: {items: [mockCeremony]}}})
    mockUseQuery.mockReturnValue({data: {sequent_backend_trustee: mockTrustees}, loading: false})
    // The board's publish-read introspection intentionally lacks these generic resources.
    mockGenericList.mockReturnValue({error: new Error("resource absent from introspection")})
    mockGenericOne.mockImplementation((resource: string) =>
        resource === "sequent_backend_keys_ceremony"
            ? {error: new Error("resource absent from introspection"), isLoading: false}
            : {}
    )
})
afterEach(cleanup)

describe("ConfigureStep scoped ceremony reads", () => {
    it("shows sorted minimal trustee choices without the unavailable generic trustee resource", async () => {
        renderStep()
        await screen.findByText("trustee-a")
        expect(screen.getAllByRole("listitem").map((node) => node.textContent)).toEqual([
            "trustee-a",
            "trustee-b",
        ])
        expect(mockGenericList).not.toHaveBeenCalled()
        expect(mockUseQuery).toHaveBeenCalledWith(
            GET_TRUSTEES_NAMES,
            expect.objectContaining({
                variables: {tenantId: "tenant"},
                context: {headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY}},
            })
        )
    })
    it("opens only the newly created ceremony using a fresh scoped direct query", async () => {
        mockGenericList.mockReturnValue({data: mockTrustees})
        renderStep()
        await create()
        await waitFor(() => expect(mockOpen).toHaveBeenCalledTimes(1))
        expect(mockCreate).toHaveBeenCalledTimes(1)
        expect(mockQuery).toHaveBeenCalledWith(
            expect.objectContaining({
                query: LIST_KEYS_CEREMONY,
                variables: {electionEventId: "event"},
                fetchPolicy: "network-only",
                context: {headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY}},
            })
        )
        expect(mockSetCeremony).toHaveBeenCalledWith(mockCeremony)
        expect(
            mockGenericOne.mock.calls.some(
                ([resource]) => resource === "sequent_backend_keys_ceremony"
            )
        ).toBe(false)
    })
    it.each([
        {id: "other", tenant_id: "tenant", election_event_id: "event"},
        {...mockCeremony, tenant_id: "other"},
        {...mockCeremony, election_event_id: "other"},
    ])("refuses a returned ceremony outside the exact created scope", async (row) => {
        mockGenericList.mockReturnValue({data: mockTrustees})
        mockQuery.mockResolvedValue({data: {list_keys_ceremony: {items: [row]}}})
        renderStep()
        await create()
        await screen.findByRole("alert")
        expect(mockOpen).not.toHaveBeenCalled()
        expect(mockSetCeremony).not.toHaveBeenCalled()
        expect(
            (
                screen.getByRole("button", {
                    name: "keysGeneration.configureStep.create",
                }) as HTMLButtonElement
            ).disabled
        ).toBe(true)
        expect(mockCreate).toHaveBeenCalledTimes(1)
    })
    it("shows a trustee read failure instead of a blank wizard", async () => {
        mockUseQuery.mockReturnValue({error: new Error("read failed"), loading: false})
        renderStep()
        await screen.findByRole("alert")
        expect(
            (
                screen.getByRole("button", {
                    name: "keysGeneration.configureStep.create",
                }) as HTMLButtonElement
            ).disabled
        ).toBe(true)
        expect(mockCreate).not.toHaveBeenCalled()
    })
    it("keeps the created request from being submitted again if its follow-up read fails", async () => {
        mockGenericList.mockReturnValue({data: mockTrustees})
        mockQuery.mockRejectedValue(new Error("read failed"))
        renderStep()
        await create()
        await screen.findByRole("alert")
        fireEvent.click(screen.getByRole("button", {name: "keysGeneration.configureStep.create"}))
        expect(mockCreate).toHaveBeenCalledTimes(1)
        expect(mockOpen).not.toHaveBeenCalled()
    })
})
