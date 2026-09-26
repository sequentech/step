// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {buildGetListVariables} from "ra-data-hasura/dist/buildVariables/buildGetListVariables"
import type {FetchType, IntrospectedResource, IntrospectionResult} from "ra-data-hasura/dist/types"
import {electionSearchFilter, electionSearchQuery, electionSearchText} from "./ElectionSearch"

describe("electionSearchFilter", () => {
    it.each(["", "   "])("does not filter an empty search %p", (text) => {
        expect(electionSearchFilter(text)).toEqual({})
    })

    it("matches the external ID or the presentation text", () => {
        expect(electionSearchFilter(" Deputy ")).toEqual({
            _or: {
                format: "hasura-raw-query",
                text: " Deputy ",
                value: [
                    {external_id: {_ilike: "%Deputy%"}},
                    {presentation: {_cast: {String: {_ilike: "%Deputy%"}}}},
                ],
            },
        })
    })
})

describe("electionSearchText", () => {
    it("gives back the text typed into the search", () => {
        expect(electionSearchText(electionSearchQuery("Mayoral "))).toBe("Mayoral ")
        expect(electionSearchText(undefined)).toBe("")
    })
})

describe("electionSearchFilter through ra-data-hasura", () => {
    it("sends Hasura an _or over external_id and the presentation text", () => {
        // Without schema fields the builder passes raw queries and plain values through.
        const resource = {type: {fields: []}} as unknown as IntrospectedResource
        const variables = buildGetListVariables({} as IntrospectionResult)(
            resource,
            "GET_LIST" as FetchType,
            {filter: {tenant_id: "tenant", ...electionSearchFilter("Deputy")}}
        )
        expect(variables.where._and).toContainEqual({
            _or: [
                {external_id: {_ilike: "%Deputy%"}},
                {presentation: {_cast: {String: {_ilike: "%Deputy%"}}}},
            ],
        })
        expect(JSON.stringify(variables.where)).not.toMatch(/"(name|alias)"/)
    })
})
