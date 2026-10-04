/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ExportLogsDialog} from "./ExportLogsDialog"
import {testI18n} from "../timezones/__fixtures__/testI18n"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
}))
const mockI18n = testI18n()
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: mockI18n.t, i18n: mockI18n}),
}))
const mockExport = jest.fn(async () => ({
    data: {export_election_event_logs: {task_execution: {id: "export-task"}}},
}))
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useMutation: () => [mockExport],
}))
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [() => ({identifier: "widget"}), jest.fn(), jest.fn()],
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({children, ok, okEnabled, handleClose}: any) => (
            <div>
                {children}
                <button disabled={!okEnabled()} onClick={() => handleClose(true)}>
                    {ok}
                </button>
            </div>
        ),
    }),
    {virtual: true}
)

beforeEach(() => mockExport.mockClear())
const mount = () =>
    render(
        <ExportLogsDialog
            electionEventId="event"
            open
            onClose={jest.fn()}
            zones={["America/New_York"]}
        />
    )
const enter = (name: string, value: string) =>
    fireEvent.change(screen.getByLabelText(name), {target: {value}})

it("refuses a non-existent bound instead of shifting the requested export range", () => {
    mount()
    enter("From", "2028-03-12T02:30")
    expect(screen.getByRole("button", {name: "Export"})).toBeDisabled()
    expect(screen.getByText(/does not exist/)).toBeVisible()
    fireEvent.click(screen.getByRole("button", {name: "Export"}))
    expect(mockExport).not.toHaveBeenCalled()
})

it("explains an overlap, exports its earlier occurrence, and preserves an empty other bound", async () => {
    mount()
    enter("From", "2028-11-05T01:30")
    expect(screen.getByText(/happens twice/)).toBeVisible()
    fireEvent.click(screen.getByRole("button", {name: "Export"}))
    await waitFor(() =>
        expect(mockExport).toHaveBeenCalledWith({
            variables: {
                electionEventId: "event",
                format: "CSV",
                createdFrom: "2028-11-05T05:30:00Z",
                createdTo: null,
                timeZone: "America/New_York",
            },
        })
    )
})
