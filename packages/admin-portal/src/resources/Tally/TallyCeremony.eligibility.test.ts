/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {TallyCeremony} from "./TallyCeremony"

const mockCreate = jest.fn()
const mockNotify = jest.fn()
const mockRefetch = jest.fn()
const mockT = (key: string) => key
const mockEmpty: unknown[] = []
const mockRecord = {
    id: "event",
    tenant_id: "tenant",
    status: {voting_status: "OPEN"},
    presentation: {},
}
let mockElections = [
    {
        id: "closed",
        keys_ceremony_id: "keys",
        status: {
            is_published: true,
            voting_status: "CLOSED",
            allow_tally: "requires-voting-period-end",
        },
    },
]
const mockKeys = {
    list_keys_ceremony: {
        items: [{id: "keys", name: "Keys", settings: {policy: "manual-ceremonies"}}],
    },
}
const mockStore = {
    tallyId: null,
    creatingType: "ELECTORAL_RESULTS",
    setTallyId: jest.fn(),
    setCreatingFlag: jest.fn(),
}

jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    useRecordContext: () => mockRecord,
    useGetOne: () => ({refetch: mockRefetch}),
    useGetList: (resource: string) => ({
        data: resource === "sequent_backend_election" ? mockElections : mockEmpty,
        refetch: mockRefetch,
    }),
}))
jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useMutation: () => [mockCreate],
    useQuery: () => ({data: mockKeys}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: mockT, i18n: {language: "en", dir: () => "ltr"}}),
}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => mockStore,
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/WidgetsContextProvider", () => ({useWidgetStore: () => []}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 1000},
    }),
}))
jest.mock("../ElectionEvent/useKeysPermissions", () => ({useKeysPermissions: () => ({})}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "Election event"}))
jest.mock("@sequentech/ui-core", () => ({
    ...require("../../../../ui-core/src/types/CoreTypes"),
    ...require("../../../../ui-core/src/types/ElectionPresentation"),
    ...require("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        BreadCrumbSteps: () => null,
        BreadCrumbStepsVariant: {Circle: "Circle"},
        theme: {},
        Dialog: ({
            open,
            handleClose,
        }: {
            open: boolean
            handleClose: (confirmed: boolean) => void
        }) =>
            open
                ? require("react").createElement(
                      "button",
                      {onClick: () => handleClose(true)},
                      "Confirm tally"
                  )
                : null,
    }),
    {virtual: true}
)
jest.mock("./TallyElectionsList", () => ({
    TallyElectionsList: ({
        update,
        disabled,
    }: {
        update: (ids: string[]) => void
        disabled: boolean
    }) =>
        require("react").createElement(
            "button",
            {disabled, onClick: () => update(["closed"])},
            "Select closed election"
        ),
}))
jest.mock("@/components/styles/TallyStyles", () => ({
    TallyStyles: new Proxy({}, {get: () => "div"}),
}))
jest.mock("@/components/styles/WizardStyles", () => ({
    WizardStyles: new Proxy({}, {get: () => "div"}),
}))
jest.mock("./styles", () => ({NextButton: "button", CancelButton: "button"}))
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/ListActions", () => ({ListActions: () => null}))
jest.mock("@/components/tally/ExportElectionMenu", () => ({ExportElectionMenu: () => null}))
jest.mock("./TallyTrusteesList", () => ({TallyTrusteesList: () => null}))
jest.mock("./TallyStartDate", () => ({TallyStartDate: () => null}))
jest.mock("./TallyElectionsProgress", () => ({TallyElectionsProgress: () => null}))
jest.mock("./TallyElectionsResults", () => ({TallyElectionsResults: () => null}))
jest.mock("./TallyResults", () => ({TallyResults: () => null}))
jest.mock("./TallyLogs", () => ({TallyLogs: () => null}))
jest.mock("./TallyResolutionPanel", () => ({TallyResolutionPanel: () => null}))
jest.mock("./ResultsWebsitePublication", () => ({ResultsWebsitePublication: () => null}))
jest.mock("./ResultsDataLoader", () => ({ResultsDataLoader: () => null}))
jest.mock("@/atoms/tally-candidates", () => ({tallyQueryData: {}}))

beforeEach(() => {
    jest.clearAllMocks()
    mockElections = [
        {
            id: "closed",
            keys_ceremony_id: "keys",
            status: {
                is_published: true,
                voting_status: "CLOSED",
                allow_tally: "requires-voting-period-end",
            },
        },
    ]
})

it("creates the closed election's ceremony while the event stays open", async () => {
    mockCreate.mockResolvedValue({data: {create_tally_ceremony: {tally_session_id: "tally"}}})
    render(React.createElement(TallyCeremony))
    fireEvent.click(screen.getByText("Select closed election"))
    fireEvent.click(screen.getByRole("button", {name: "tally.common.ceremony"}))
    await act(async () => fireEvent.click(screen.getByText("Confirm tally")))
    expect(mockCreate).toHaveBeenCalledWith(
        expect.objectContaining({variables: expect.objectContaining({election_ids: ["closed"]})})
    )
    expect(mockStore.setTallyId).toHaveBeenCalledWith("tally")
})

it("blocks manual creation and explains why when the selected election is still open", () => {
    mockElections[0].status.voting_status = "OPEN"
    render(React.createElement(TallyCeremony))
    fireEvent.click(screen.getByText("Select closed election"))
    expect(
        (screen.getByRole("button", {name: "tally.common.ceremony"}) as HTMLButtonElement).disabled
    ).toBe(true)
    expect(screen.getByText("tally.eligibility.endVoting")).toBeTruthy()
    expect(mockCreate).not.toHaveBeenCalled()
})

it("displays the Hasura validation reason and lets the user retry after a stale-state rejection", async () => {
    const reason = "Election closed: end its voting period before tallying."
    mockCreate.mockRejectedValue({
        graphQLErrors: [{message: reason, extensions: {code: "TallyValidation"}}],
    })
    render(React.createElement(TallyCeremony))
    fireEvent.click(screen.getByText("Select closed election"))
    fireEvent.click(screen.getByRole("button", {name: "tally.common.ceremony"}))
    await act(async () => fireEvent.click(screen.getByText("Confirm tally")))
    expect(mockNotify).toHaveBeenCalledWith(reason, {type: "error"})
    await waitFor(() =>
        expect((screen.getByText("Select closed election") as HTMLButtonElement).disabled).toBe(
            false
        )
    )
    expect(
        (screen.getByRole("button", {name: "tally.common.ceremony"}) as HTMLButtonElement).disabled
    ).toBe(false)
})
