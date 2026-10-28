/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import {ResultsDataLoader} from "./ResultsDataLoader"
const mockSetData = jest.fn()
let mockQueryError = true
jest.mock("jotai", () => ({useSetAtom: () => mockSetData}))
jest.mock("@/atoms/tally-candidates", () => ({tallyQueryData: {}}))
jest.mock("react-admin", () => ({useTranslate: () => (key: string) => key}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/SettingsContextProvider", () => ({SettingsContext: {}}))
jest.mock("@sequentech/ui-core", () => ({isString: (value: unknown) => typeof value === "string"}))
jest.mock("@/hooks/useSQLiteDatabase", () => ({
    useManagedDatabase: () => ({isReady: true, error: null}),
    useSQLQuery: (sql: string) => {
        const error = mockQueryError && sql.includes("FROM results_area_contest_candidate ")
        return {data: [], isReady: !error, error: error ? "missing candidate results" : null}
    },
}))

test("surfaces a candidate-query failure and publishes only after recovery", () => {
    const props = {
        resultsEventId: "result",
        electionEventId: "event",
        isTallyCompleted: false,
        contests: [{id: "contest"}] as any,
        electionIds: ["election"],
        databaseName: "document",
    }
    const {rerender} = render(React.createElement(ResultsDataLoader, props))
    expect(screen.getByRole("alert").textContent).toBe("ra.message.error")
    expect(mockSetData).toHaveBeenCalledWith(null)
    expect(mockSetData.mock.calls.every(([value]) => value === null)).toBe(true)
    mockQueryError = false
    rerender(React.createElement(ResultsDataLoader, props))
    expect(screen.queryByRole("alert")).toBeNull()
    expect(mockSetData).toHaveBeenLastCalledWith(
        expect.objectContaining({sqliteDocumentId: "document"})
    )
    mockQueryError = true
    rerender(React.createElement(ResultsDataLoader, props))
    expect(screen.getByRole("alert").textContent).toBe("ra.message.error")
    expect(mockSetData).toHaveBeenLastCalledWith(null)
})
