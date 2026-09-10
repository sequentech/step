/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, within, waitFor} from "@testing-library/react"
import {Publish} from "./Publish"
import {EPublishType} from "./EPublishType"

const mockGenerate = jest.fn()
const mockPublish = jest.fn()
const mockOtherMutation = jest.fn()
const mockNotify = jest.fn()
const mockRefetch = jest.fn()
const mockRefresh = jest.fn()
let mockRecord = {id: "event", status: {voting_status: "CLOSED"}, presentation: {}}
let mockPublication: {id: string; is_generated: boolean} | undefined
const mockT = (key: string) => key
let mockTask: {id: string; execution_status: string; logs: Array<{log_text: string}>} | undefined

jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useMutation: (query: string) => [
        query.includes("mutation GenerateBallotPublication")
            ? mockGenerate
            : query.includes("mutation PublishBallot")
              ? mockPublish
              : mockOtherMutation,
        {},
    ],
    useQuery: (_query: string, options: {skip?: boolean}) => ({
        data: options.skip
            ? undefined
            : {sequent_backend_tasks_execution: mockTask ? [mockTask] : []},
    }),
}))
jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    useRefresh: () => mockRefresh,
    useRecordContext: () => mockRecord,
    useGetOne: () => ({refetch: mockRefetch, data: mockPublication}),
    Button: ({label, children, ...props}: any) =>
        require("react").createElement("button", props, label, children),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockT})}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: () => true,
        isGoldUser: () => false,
    }),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_POLL_INTERVAL_MS: 100},
    }),
}))
jest.mock("@/lib/helpers", () => ({convertToNumber: () => undefined}))
jest.mock("@sequentech/ui-core", () => ({
    ETaskExecutionStatus: {SUCCESS: "SUCCESS", FAILED: "FAILED"},
    EVotingStatus: {NOT_STARTED: "NOT_STARTED"},
}))
jest.mock("./PublishList", () => ({
    PublishList: ({onGenerate}: {onGenerate: () => void}) =>
        require("react").createElement("button", {onClick: onGenerate}, "Publish changes"),
}))
jest.mock("./PublishActions", () => ({PublishActions: () => null}))
jest.mock("./PublishExport", () => ({__esModule: true, default: () => null}))
jest.mock("./usePublishPermissions", () => ({
    usePublishPermissions: () => ({canWritePublish: true, showPublishButtonBack: true}),
}))
jest.mock("@sequentech/ui-essentials", () => ({Dialog: () => null}))
jest.mock("./EditPreview", () => ({EditPreview: () => null}))
jest.mock("@/components/FormDialog", () => ({__esModule: true, default: () => null}))

const props = {electionEventId: "event", type: EPublishType.Event}

beforeEach(() => {
    jest.clearAllMocks()
    sessionStorage.clear()
    mockTask = undefined
    mockPublication = undefined
    mockGenerate.mockResolvedValue({
        data: {
            generate_ballot_publication: {
                ballot_publication_id: "publication",
                task_execution: {id: "task"},
            },
        },
    })
    mockOtherMutation.mockResolvedValue({
        data: {
            get_ballot_publication_changes: {
                previous: {title: "before"},
                current: {title: "after"},
            },
        },
    })
})

it.each(["CLOSED", "OPEN", "PAUSED", "NOT_STARTED"])(
    "shows loading then enables publishing independently of voting status %s",
    async (voting_status) => {
        mockRecord = {id: "event", status: {voting_status}, presentation: {}}
        const view = render(React.createElement(Publish, props))
        await act(async () => fireEvent.click(screen.getByText("Publish changes")))
        expect(await screen.findByRole("progressbar")).toBeTruthy()
        expect(
            (screen.getByRole("button", {name: "publish.action.publish"}) as HTMLButtonElement)
                .disabled
        ).toBe(true)
        // A refreshed event record must not reset the publication's progress.
        mockRecord = {...mockRecord}
        view.rerender(React.createElement(Publish, {...props, electionId: "refresh"}))
        expect(screen.getByRole("progressbar")).toBeTruthy()
        mockTask = {id: "task", execution_status: "SUCCESS", logs: []}
        mockPublication = {id: "publication", is_generated: true}
        view.rerender(React.createElement(Publish, {...props, electionId: "complete"}))
        await waitFor(() =>
            expect(
                (screen.getByRole("button", {name: "publish.action.publish"}) as HTMLButtonElement)
                    .disabled
            ).toBe(false)
        )
        await waitFor(() => expect(screen.queryByRole("progressbar")).toBeNull())
        expect(mockPublish).not.toHaveBeenCalled()
    }
)
