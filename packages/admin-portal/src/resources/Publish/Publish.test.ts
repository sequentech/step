/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, fireEvent, render, screen, within} from "@testing-library/react"
import {Publish} from "./Publish"
import {EPublishType} from "./EPublishType"

const mockGenerate = jest.fn()
const mockPublish = jest.fn()
const mockOtherMutation = jest.fn()
const mockNotify = jest.fn()
const mockRefetch = jest.fn()
const mockRefresh = jest.fn()
const mockRecord = {id: "event", status: {}, presentation: {}}
const mockT = (key: string) => key
let mockPublication: {id: string; is_generated: boolean} | undefined
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
jest.mock("./PublishGenerate", () => ({
    PublishGenerate: ({onPublish, onGenerate, onBack, status, data}: any) =>
        require("react").createElement(
            "div",
            null,
            require("react").createElement("button", {onClick: onPublish}, "Publish ballot"),
            require("react").createElement("button", {onClick: onGenerate}, "Retry generation"),
            require("react").createElement("button", {onClick: onBack}, "Back"),
            require("react").createElement(
                "span",
                {"data-testid": "detail-state"},
                `${status}/${data?.current?.ballot_publication_id ?? "empty"}`
            )
        ),
}))
jest.mock("./EditPreview", () => ({EditPreview: () => null}))
jest.mock("@/components/FormDialog", () => ({__esModule: true, default: () => null}))

const props = {electionEventId: "event", type: EPublishType.Event}

beforeEach(() => {
    jest.clearAllMocks()
    sessionStorage.clear()
    mockTask = undefined
    mockPublication = undefined
    mockOtherMutation.mockReset()
    mockGenerate.mockResolvedValue({
        data: {
            generate_ballot_publication: {
                ballot_publication_id: "publication",
                task_execution: {id: "task"},
            },
        },
    })
})

it("keeps an asynchronous generation failure visible on the publication details", async () => {
    const view = render(React.createElement(Publish, props))
    await act(async () => fireEvent.click(screen.getByText("Publish changes")))
    await screen.findByText("Publish ballot")
    mockTask = {
        id: "task",
        execution_status: "FAILED",
        logs: [{log_text: "Error: Inconsistent event presentation within publication"}],
    }
    view.rerender(React.createElement(Publish, {...props, electionId: "election"}))
    await screen.findByText("Publish ballot")
    expect(screen.queryByText("Publish changes")).toBeNull()
    const alert = await screen.findByRole("alert")
    expect(within(alert).getByText(mockTask.logs[0].log_text)).toBeTruthy()
    view.rerender(React.createElement(Publish, {...props, electionId: "election2"}))
    expect(screen.getByRole("alert")).toBeTruthy()
    fireEvent.click(within(screen.getByRole("alert")).getByRole("button", {name: "Close"}))
    expect(screen.queryByRole("alert")).toBeNull()
})

it("shows a synchronous generation error persistently and clears it on retry", async () => {
    mockGenerate.mockRejectedValueOnce(new Error("Unable to queue generation\nPlease retry"))
    render(React.createElement(Publish, props))
    await act(async () => fireEvent.click(screen.getByText("Publish changes")))
    const alert = await screen.findByRole("alert")
    expect(alert.textContent).toContain("Unable to queue generation\nPlease retry")
    await act(async () => fireEvent.click(screen.getByText("Retry generation")))
    await screen.findByText("Publish ballot")
    expect(screen.queryByRole("alert")).toBeNull()
    expect(mockGenerate).toHaveBeenCalledTimes(2)
})

it("does not apply a previous task's failure to a new generation", async () => {
    mockTask = {id: "older-task", execution_status: "FAILED", logs: [{log_text: "Old error"}]}
    render(React.createElement(Publish, props))
    await act(async () => fireEvent.click(screen.getByText("Publish changes")))
    expect(mockGenerate).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole("alert")).toBeNull()
    expect(screen.getByText("Publish ballot")).toBeTruthy()
})

it("keeps synchronous publication failures in the same persistent panel", async () => {
    mockPublish.mockRejectedValueOnce(new Error("Publication validation failed"))
    render(React.createElement(Publish, props))
    await act(async () => fireEvent.click(screen.getByText("Publish changes")))
    await act(async () => fireEvent.click(screen.getByText("Publish ballot")))
    const alert = await screen.findByRole("alert")
    expect(within(alert).getByText("Publication validation failed")).toBeTruthy()
})

it("does not return to details when a generation finishes after Back", async () => {
    let resolve: (value: any) => void = () => {}
    mockGenerate.mockImplementationOnce(
        () =>
            new Promise((done) => {
                resolve = done
            })
    )
    render(React.createElement(Publish, props))
    fireEvent.click(screen.getByText("Publish changes"))
    fireEvent.click(screen.getByText("Back"))
    await act(async () =>
        resolve({
            data: {
                generate_ballot_publication: {
                    ballot_publication_id: "old",
                    task_execution: {id: "old-task"},
                },
            },
        })
    )
    expect(screen.getByText("Publish changes")).toBeTruthy()
    expect(screen.queryByText("Publish ballot")).toBeNull()
})

it("ignores a diff response from before regeneration", async () => {
    let resolve: (value: any) => void = () => {}
    mockOtherMutation.mockImplementationOnce(
        () =>
            new Promise((done) => {
                resolve = done
            })
    )
    mockPublication = {id: "publication", is_generated: true}
    render(React.createElement(Publish, props))
    await act(async () => fireEvent.click(screen.getByText("Publish changes")))
    mockGenerate.mockResolvedValueOnce({
        data: {
            generate_ballot_publication: {
                ballot_publication_id: "new-publication",
                task_execution: {id: "new-task"},
            },
        },
    })
    await act(async () => fireEvent.click(screen.getByText("Retry generation")))
    await act(async () =>
        resolve({
            data: {
                get_ballot_publication_changes: {
                    current: {ballot_publication_id: "publication", ballot_styles: []},
                },
            },
        })
    )
    expect(screen.getByTestId("detail-state").textContent).toContain("/empty")
})

it("keeps a generated publication ready to retry after publishing fails", async () => {
    mockPublication = {id: "publication", is_generated: true}
    mockOtherMutation.mockResolvedValue({
        data: {
            get_ballot_publication_changes: {
                current: {ballot_publication_id: "publication", ballot_styles: []},
            },
        },
    })
    mockPublish.mockRejectedValueOnce(new Error("Publication validation failed"))
    render(React.createElement(Publish, props))
    await act(async () => fireEvent.click(screen.getByText("Publish changes")))
    const readyState = screen.getByTestId("detail-state").textContent
    await act(async () => fireEvent.click(screen.getByText("Publish ballot")))
    expect(screen.getByTestId("detail-state").textContent).toEqual(readyState)
})
