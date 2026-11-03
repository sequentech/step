/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, renderHook, waitFor} from "@testing-library/react"
import {QueryClient} from "@tanstack/react-query"
import {TextEncoder, TextDecoder} from "util"

Object.assign(globalThis, {TextEncoder, TextDecoder})
const {CoreAdminContext, useGetList} = require("ra-core")

const mockProvider = {getList: jest.fn()}

test("real useGetList flattens hook data but refetch retains the provider envelope", async () => {
    const rows = [{id: "execution", results_event_id: "final-results"}]
    mockProvider.getList.mockResolvedValue({data: rows, total: 1})
    const client = new QueryClient({defaultOptions: {queries: {retry: false}}})
    const wrapper = ({children}: {children: React.ReactNode}) =>
        React.createElement(
            CoreAdminContext,
            {queryClient: client, dataProvider: mockProvider as any},
            children
        )
    const {result, unmount} = renderHook(
        () => useGetList("sequent_backend_tally_session_execution"),
        {wrapper}
    )
    await waitFor(() => expect(result.current.data).toEqual(rows))
    let refreshed: unknown
    await act(async () => {
        refreshed = await result.current.refetch({throwOnError: true})
    })
    expect(refreshed).toMatchObject({data: {data: rows, total: 1}})
    unmount()
    client.clear()
})
