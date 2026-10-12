// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const mockBuildQuery = jest.fn()

jest.mock("ra-data-hasura", () => ({
    buildQuery: () => mockBuildQuery,
    buildVariables: () => jest.fn(),
}))

import {customBuildQuery} from "./customBuildQuery"
import {Order_By} from "@/gql/graphql"

const RESOURCE = "sequent_backend_tally_session_execution"

const buildParams = (meta?: Record<string, unknown>) => ({
    pagination: {page: 1, perPage: 10},
    sort: {field: "created_at", order: "DESC"},
    filter: {
        tally_session_id: {format: "hasura-raw-query", value: {_in: ["session-1"]}},
        tenant_id: "tenant-1",
    },
    meta,
})

describe("customBuildQuery: latest tally session execution per session", () => {
    beforeEach(() => {
        mockBuildQuery.mockReset()
        mockBuildQuery.mockImplementation(() => ({
            query: "QUERY",
            variables: {where: {_and: []}, order_by: [{created_at: "desc"}]},
        }))
    })

    it("orders by tally_session_id first so distinct_on is valid in Postgres", () => {
        const params = buildParams({latestPerTallySession: true})

        const ret = customBuildQuery({})("GET_LIST", RESOURCE, params)

        expect(params.filter).toMatchObject({distinct_on: ["tally_session_id"]})
        expect(ret.variables.order_by[0]).toEqual({tally_session_id: "asc"})
    })

    it("orders nullable created_at with nulls last so a null-dated execution never wins", () => {
        const ret = customBuildQuery({})(
            "GET_LIST",
            RESOURCE,
            buildParams({latestPerTallySession: true})
        )

        expect(ret.variables.order_by).toEqual([
            {tally_session_id: "asc"},
            {created_at: "desc_nulls_last"},
            {id: "desc"},
        ])
    })

    it("uses an ordering direction the Hasura schema actually accepts", () => {
        const ret = customBuildQuery({})(
            "GET_LIST",
            RESOURCE,
            buildParams({latestPerTallySession: true})
        )

        const directions = Object.values(Order_By) as string[]
        ret.variables.order_by.forEach((entry: Record<string, string>) =>
            Object.values(entry).forEach((direction) => expect(directions).toContain(direction))
        )
    })

    it("does not order created_at with a nulls-first direction", () => {
        const ret = customBuildQuery({})(
            "GET_LIST",
            RESOURCE,
            buildParams({latestPerTallySession: true})
        )

        const createdAtOrder = ret.variables.order_by.find(
            (entry: Record<string, string>) => "created_at" in entry
        )?.created_at

        expect(["desc", "desc_nulls_first"]).not.toContain(createdAtOrder)
    })

    it("leaves the query untouched when the latestPerTallySession flag is absent", () => {
        const params = buildParams()

        const ret = customBuildQuery({})("GET_LIST", RESOURCE, params)

        expect(params.filter).not.toHaveProperty("distinct_on")
        expect(ret.variables.order_by).toEqual([{created_at: "desc"}])
    })

    it("keeps the default order_by when the delegate returns none", () => {
        mockBuildQuery.mockImplementation(() => ({query: "QUERY", variables: {where: {_and: []}}}))

        const ret = customBuildQuery({})(
            "GET_LIST",
            RESOURCE,
            buildParams({latestPerTallySession: true})
        )

        expect(ret.variables).not.toHaveProperty("order_by")
    })
})
