/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {renderHook, waitFor} from "@testing-library/react"
import {SUPPORT_REFERENCE_DEBOUNCE_MS, useSupportReferenceSearch} from "./useSupportReferenceSearch"

const APPLICATION = "7f0c2a9e-1b3d-4c5e-8f60-123456789abc"
const mockSetFilters = jest.fn()
const mockQuery = jest.fn()
const mockNotify = jest.fn()
let mockFilterValues: Record<string, unknown> = {}

jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useApolloClient: () => ({query: mockQuery}),
}))
jest.mock("react-admin", () => ({
    useListContext: () => ({filterValues: mockFilterValues, setFilters: mockSetFilters}),
    useNotify: () => mockNotify,
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

beforeEach(() => {
    mockSetFilters.mockReset()
    mockQuery.mockReset()
    mockNotify.mockReset()
    mockQuery.mockResolvedValue({
        data: {
            find_applications_by_support_reference: {
                reference: "65SR-MSSD",
                application_ids: [APPLICATION],
            },
        },
    })
})

it("asks the server for the application of a support reference searched in the queue (AB-5)", async () => {
    mockFilterValues = {q: "65sr-mssd", status: "pending"}
    renderHook(() =>
        useSupportReferenceSearch({election_event_id: "event-2", permission_label: "post-a"})
    )
    await waitFor(() =>
        expect(mockSetFilters).toHaveBeenCalledWith(
            {
                q: "65sr-mssd",
                status: "pending",
                support_reference: {reference: "65SR-MSSD", ids: [APPLICATION]},
            },
            {}
        )
    )
    expect(mockQuery).toHaveBeenCalledTimes(1)
    expect(mockQuery.mock.calls[0][0].variables).toEqual({
        electionEventId: "event-2",
        reference: "65SR-MSSD",
        permissionLabel: "post-a",
    })
    expect(mockQuery.mock.calls[0][0].context).toEqual({
        headers: {"x-hasura-role": "application-read"},
    })
})

it("looks nothing up for a search that isn't a reference, one already found, or no event", () => {
    jest.useFakeTimers()
    mockFilterValues = {q: "dela cruz"}
    renderHook(() => useSupportReferenceSearch({election_event_id: "event-2"}))
    mockFilterValues = {q: "65SR-MSSD", support_reference: {reference: "65SR-MSSD", ids: []}}
    renderHook(() => useSupportReferenceSearch({election_event_id: "event-2"}))
    mockFilterValues = {q: "65SR-MSSD"}
    renderHook(() => useSupportReferenceSearch({}))
    jest.advanceTimersByTime(SUPPORT_REFERENCE_DEBOUNCE_MS)
    expect(mockQuery).not.toHaveBeenCalled()
    jest.useRealTimers()
})

it("says so when the reference can't be looked up", async () => {
    mockQuery.mockRejectedValue(new Error("refused"))
    mockFilterValues = {q: "65SR-MSSD"}
    renderHook(() => useSupportReferenceSearch({election_event_id: "event-2"}))
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("approvalsScreen.list.referenceError", {
            type: "error",
        })
    )
    expect(mockSetFilters).not.toHaveBeenCalled()
})

it("asks once the search stops changing, not on every keystroke (R8-14)", async () => {
    jest.useFakeTimers()
    try {
        mockFilterValues = {q: "65SR-MSSD"}
        const {rerender} = renderHook(() =>
            useSupportReferenceSearch({election_event_id: "event-2"})
        )
        // Each keystroke that still reads as a reference restarts the wait.
        jest.advanceTimersByTime(SUPPORT_REFERENCE_DEBOUNCE_MS - 1)
        mockFilterValues = {q: "65SR-MSSE"}
        rerender()
        jest.advanceTimersByTime(SUPPORT_REFERENCE_DEBOUNCE_MS - 1)
        expect(mockQuery).not.toHaveBeenCalled()
        jest.advanceTimersByTime(1)
        expect(mockQuery).toHaveBeenCalledTimes(1)
        expect(mockQuery.mock.calls[0][0].variables.reference).toBe("65SR-MSSE")
    } finally {
        jest.useRealTimers()
    }
})
