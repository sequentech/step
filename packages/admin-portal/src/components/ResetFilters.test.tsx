/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render} from "@testing-library/react"
import {ResetFilters} from "./ResetFilters"
import {logsFilterSearch} from "./logs/messagingLogFilters"
import {EMessageChannel} from "@/types/messaging"

const mockSetFilters = jest.fn()
let mockSearch = ""
jest.mock("react-admin", () => ({useListContext: () => ({setFilters: mockSetFilters})}))
jest.mock("react-router-dom", () => ({useLocation: () => ({search: mockSearch})}))

beforeEach(() => {
    mockSetFilters.mockReset()
    mockSearch = ""
})

it("clears the filters when the list opens", () => {
    render(<ResetFilters />)
    expect(mockSetFilters).toHaveBeenCalledWith({}, {})
})

it("keeps the filter a link opened the list with (R4-02: Failed opens Logs filtered)", () => {
    mockSearch = logsFilterSearch(EMessageChannel.SMS)
    render(<ResetFilters keepLinkedFilter />)
    expect(mockSetFilters).not.toHaveBeenCalled()
})

it("still clears when the location carries no filter", () => {
    mockSearch = "?tabId=logs"
    render(<ResetFilters keepLinkedFilter />)
    expect(mockSetFilters).toHaveBeenCalledWith({}, {})
})
