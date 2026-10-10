/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {webcrypto} from "crypto"
import {TextEncoder} from "util"
import {renderHook, waitFor} from "@testing-library/react"
import {useSupportReferenceSearch} from "./useSupportReferenceSearch"

const APPLICATION = "7f0c2a9e-1b3d-4c5e-8f60-123456789abc"
const mockSetFilters = jest.fn()
const mockQuery = jest.fn()
let mockFilterValues: Record<string, unknown> = {}

jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useApolloClient: () => ({query: mockQuery}),
}))
jest.mock("react-admin", () => ({
    useListContext: () => ({filterValues: mockFilterValues, setFilters: mockSetFilters}),
    useNotify: () => jest.fn(),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

beforeAll(() => {
    Object.defineProperty(globalThis, "crypto", {value: webcrypto, configurable: true})
    Object.defineProperty(globalThis, "TextEncoder", {value: TextEncoder, configurable: true})
})

beforeEach(() => {
    mockSetFilters.mockReset()
    mockQuery.mockReset()
    mockQuery.mockResolvedValue({
        data: {
            sequent_backend_applications: [
                {id: "7f0c2a9e-1b3d-4c5e-8f60-123456789abd"},
                {id: APPLICATION},
            ],
        },
    })
})

it("finds the application of a support reference searched in the queue (EF-1)", async () => {
    mockFilterValues = {q: "65sr-mssd", status: "pending"}
    renderHook(() => useSupportReferenceSearch({election_event_id: "event-2"}))
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
    expect(mockQuery.mock.calls[0][0].variables).toEqual({
        where: {_and: [{election_event_id: {_eq: "event-2"}}]},
        offset: 0,
    })
})

it("looks nothing up for a search that isn't a reference, or one already found", () => {
    mockFilterValues = {q: "dela cruz"}
    renderHook(() => useSupportReferenceSearch({election_event_id: "event-2"}))
    mockFilterValues = {q: "65SR-MSSD", support_reference: {reference: "65SR-MSSD", ids: []}}
    renderHook(() => useSupportReferenceSearch({election_event_id: "event-2"}))
    expect(mockQuery).not.toHaveBeenCalled()
})
