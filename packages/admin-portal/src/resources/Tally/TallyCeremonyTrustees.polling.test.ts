/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import {TallyCeremonyTrustees} from "./TallyCeremonyTrustees"
import {ITallyExecutionStatus, ITallyTrusteeStatus} from "@/types/ceremonies"

const POLL_INTERVAL_MS = 5000
const TRUSTEE_NAME = "trustee-1"
const TALLY_ID = "tally-session-1"

interface MockTallySession {
    id: string
    election_event_id: string
    execution_status: ITallyExecutionStatus
    is_execution_completed: boolean
    keys_ceremony_id: string
    election_ids: Array<string>
}

interface MockExecution {
    status: {trustees: Array<{name: string; status: ITallyTrusteeStatus}>}
}

let mockTally: MockTallySession
let mockExecutions: Array<MockExecution>
const mockGetOneOptions = jest.fn()

const tallySession = (
    execution_status: ITallyExecutionStatus,
    is_execution_completed = false
): MockTallySession => ({
    id: TALLY_ID,
    election_event_id: "event-1",
    execution_status,
    is_execution_completed,
    keys_ceremony_id: "keys-ceremony-1",
    election_ids: [],
})

const execution = (status: ITallyTrusteeStatus): MockExecution => ({
    status: {trustees: [{name: TRUSTEE_NAME, status}]},
})

jest.mock("react-admin", () => ({
    useRecordContext: () => ({id: "event-1", presentation: {}}),
    useGetOne: (resource: string, _params: unknown, options: unknown) => {
        if (resource === "sequent_backend_tally_session") {
            mockGetOneOptions(options)
        }
        return {data: mockTally, isPending: false}
    },
    useGetList: (resource: string) => ({
        data: resource === "sequent_backend_tally_session_execution" ? mockExecutions : [],
        isPending: false,
    }),
}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => ({tallyId: "tally-session-1", setTallyId: jest.fn()}),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant-1"]}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 5000},
    }),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({trustee: "trustee-1"}),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@apollo/client", () => ({useMutation: () => [jest.fn()]}))
jest.mock("@/queries/RestorePrivateKey", () => ({RESTORE_PRIVATE_KEY: "mutation"}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        BreadCrumbSteps: () => null,
        BreadCrumbStepsVariant: {Circle: "Circle"},
        DropFile: () => require("react").createElement("div", {"data-testid": "drop-file"}),
    }),
    {virtual: true}
)
jest.mock("@/components/ElectionHeader", () => ({__esModule: true, default: () => null}))
jest.mock("./TallyElectionsList", () => ({TallyElectionsList: () => null}))
jest.mock("./TallyTrusteesList", () => ({
    TallyTrusteesList: () => require("react").createElement("div", {"data-testid": "status-step"}),
}))
jest.mock("@mui/icons-material/ChevronRight", () => ({__esModule: true, default: () => null}))
jest.mock("@mui/icons-material/ArrowBackIos", () => ({__esModule: true, default: () => null}))
jest.mock("@/components/styles/TallyStyles", () => {
    const passThrough =
        () =>
        ({children}: {children?: React.ReactNode}) =>
            require("react").createElement("div", null, children)
    return {
        TallyStyles: {
            WizardContainer: passThrough(),
            ContentWrapper: passThrough(),
            StyledHeader: passThrough(),
            FooterContainer: passThrough(),
            StyledFooter: passThrough(),
        },
    }
})
jest.mock("@/components/styles/WizardStyles", () => {
    const passThrough =
        () =>
        ({children}: {children?: React.ReactNode}) =>
            require("react").createElement("div", null, children)
    return {
        WizardStyles: {
            WizardWrapper: passThrough(),
            StatusBox: passThrough(),
            DownloadProgress: () => null,
            ErrorMessage: passThrough(),
            SucessMessage: passThrough(),
        },
    }
})

describe("TallyCeremonyTrustees", () => {
    beforeEach(() => {
        mockGetOneOptions.mockClear()
        mockTally = tallySession(ITallyExecutionStatus.STARTED)
        mockExecutions = [execution(ITallyTrusteeStatus.WAITING)]
    })

    const lastGetOneOptions = () =>
        mockGetOneOptions.mock.calls[mockGetOneOptions.mock.calls.length - 1][0]

    it("polls the tally session while the ceremony is open", () => {
        render(React.createElement(TallyCeremonyTrustees))

        expect(lastGetOneOptions()).toEqual(
            expect.objectContaining({refetchInterval: POLL_INTERVAL_MS})
        )
    })

    it("stops polling the tally session once the execution is completed", () => {
        mockTally = tallySession(ITallyExecutionStatus.SUCCESS, true)

        render(React.createElement(TallyCeremonyTrustees))

        expect(lastGetOneOptions()).toEqual(expect.objectContaining({refetchInterval: undefined}))
    })

    it.each([ITallyExecutionStatus.STARTED, ITallyExecutionStatus.CONNECTED])(
        "offers the key upload to a waiting trustee of a %s tally",
        (execution_status) => {
            mockTally = tallySession(execution_status)

            render(React.createElement(TallyCeremonyTrustees))

            expect(screen.getByTestId("drop-file")).toBeDefined()
        }
    )

    it.each([ITallyExecutionStatus.IN_PROGRESS, ITallyExecutionStatus.CANCELLED])(
        "withdraws the key upload when the tally becomes %s",
        (execution_status) => {
            mockTally = tallySession(ITallyExecutionStatus.STARTED)
            const {rerender} = render(React.createElement(TallyCeremonyTrustees))
            expect(screen.getByTestId("drop-file")).toBeDefined()

            mockTally = tallySession(execution_status)
            rerender(React.createElement(TallyCeremonyTrustees))

            expect(screen.queryByTestId("drop-file")).toBeNull()
            expect(screen.getByTestId("status-step")).toBeDefined()
        }
    )

    it("keeps a non-participating trustee on the status step of a started tally", () => {
        mockTally = tallySession(ITallyExecutionStatus.STARTED)
        mockExecutions = [
            {status: {trustees: [{name: "other", status: ITallyTrusteeStatus.WAITING}]}},
        ]

        render(React.createElement(TallyCeremonyTrustees))

        expect(screen.queryByTestId("drop-file")).toBeNull()
        expect(screen.getByTestId("status-step")).toBeDefined()
    })
})
