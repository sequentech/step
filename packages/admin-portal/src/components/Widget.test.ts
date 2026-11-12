/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, cleanup, render, screen} from "@testing-library/react"
import {createTheme, ThemeProvider} from "@mui/material/styles"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {ETasksExecution} from "@/types/tasksExecution"
import {WidgetsContextProvider, useWidgetStore} from "@/providers/WidgetsContextProvider"

const TASK_ID = "task-1"

let mockTask: {type: string; execution_status: string; logs: Array<unknown>} | undefined

jest.mock("@sequentech/ui-core", () => ({
    ETaskExecutionStatus: {
        STARTED: "STARTED",
        IN_PROGRESS: "IN_PROGRESS",
        SUCCESS: "SUCCESS",
        FAILED: "FAILED",
    },
}))
jest.mock("@apollo/client", () => ({
    useQuery: (_query: unknown, options: {skip: boolean}) => ({
        data: options.skip || !mockTask ? undefined : {sequent_backend_tasks_execution: [mockTask]},
    }),
}))
jest.mock("@/queries/GetTaskById", () => ({GET_TASK_BY_ID: "query"}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_FAST_POLL_INTERVAL_MS: 1000, QUERY_POLL_INTERVAL_MS: 5000},
    }),
}))
jest.mock("@/resources/Tasks/ViewTask", () => ({ViewTask: () => null}))
jest.mock("@/resources/User/DownloadDocument", () => ({DownloadDocument: () => null}))
jest.mock("react-admin", () => ({Button: "button"}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("./StatusChip", () => ({
    StatusChip: ({status}: {status: string}) =>
        require("react").createElement("span", {"data-testid": "widget-status"}, status),
}))

let store: ReturnType<typeof useWidgetStore>

const StoreProbe: React.FC = () => {
    store = useWidgetStore()
    return null
}

const baseTheme = createTheme()
const theme = {...baseTheme, palette: {...baseTheme.palette, customGrey: {main: "#4b4b4b"}}}

const renderWidgets = () =>
    render(
        React.createElement(
            ThemeProvider,
            {theme},
            React.createElement(WidgetsContextProvider, null, React.createElement(StoreProbe, null))
        )
    )

beforeEach(() => {
    mockTask = undefined
})

afterEach(cleanup)

describe("Widget", () => {
    it("shows a request that fails before any task exists as failed", () => {
        renderWidgets()
        const [addWidget, , updateWidgetFail] = store

        let identifier = ""
        act(() => {
            identifier = addWidget(ETasksExecution.EXPORT_ELECTION_EVENT, undefined).identifier
        })
        expect(screen.getByTestId("widget-status").textContent).toBe(
            ETaskExecutionStatus.IN_PROGRESS
        )

        act(() => updateWidgetFail(identifier))

        expect(screen.getByTestId("widget-status").textContent).toBe(ETaskExecutionStatus.FAILED)
        expect(screen.getByRole("button", {expanded: true})).toBeTruthy()
    })

    it("keeps following the task once the request returns one", () => {
        renderWidgets()
        const [addWidget, setWidgetTaskId] = store
        mockTask = {
            type: ETasksExecution.EXPORT_ELECTION_EVENT,
            execution_status: ETaskExecutionStatus.SUCCESS,
            logs: [],
        }

        act(() => {
            const {identifier} = addWidget(ETasksExecution.EXPORT_ELECTION_EVENT, undefined)
            setWidgetTaskId(identifier, TASK_ID)
        })

        expect(screen.getByTestId("widget-status").textContent).toBe(ETaskExecutionStatus.SUCCESS)
    })
})
