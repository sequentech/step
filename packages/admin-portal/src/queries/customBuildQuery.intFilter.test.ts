// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    buildClientSchema,
    getVariableValues,
    IntrospectionQuery,
    OperationDefinitionNode,
    DocumentNode,
} from "graphql"
import {introspectSchema} from "ra-data-graphql"
import schemaJson from "../../graphql.schema.json"
import {customBuildQuery} from "./customBuildQuery"

const introspection = schemaJson as unknown as IntrospectionQuery
const schema = buildClientSchema(introspection)

const operationNames = {
    GET_LIST: (resource: {name: string}) => resource.name,
    GET_ONE: (resource: {name: string}) => resource.name,
    GET_MANY: (resource: {name: string}) => resource.name,
    GET_MANY_REFERENCE: (resource: {name: string}) => resource.name,
    CREATE: (resource: {name: string}) => `insert_${resource.name}`,
    UPDATE: (resource: {name: string}) => `update_${resource.name}`,
    UPDATE_MANY: (resource: {name: string}) => `update_${resource.name}`,
    DELETE: (resource: {name: string}) => `delete_${resource.name}`,
    DELETE_MANY: (resource: {name: string}) => `delete_${resource.name}`,
}

let introspectionResults: Awaited<ReturnType<typeof introspectSchema>>

beforeAll(async () => {
    introspectionResults = await introspectSchema(undefined as never, {
        schema: introspection.__schema,
        operationNames,
        exclude: undefined,
        include: undefined,
    })
})

const getList = (resource: string, filter: Record<string, unknown>) =>
    customBuildQuery(introspectionResults)("GET_LIST", resource, {
        pagination: {page: 1, perPage: 10},
        sort: {field: "id", order: "ASC"},
        filter,
    })

const variableErrors = (query: DocumentNode, variables: Record<string, unknown>) => {
    const operation = query.definitions[0] as OperationDefinitionNode
    const result = getVariableValues(schema, operation.variableDefinitions ?? [], variables)
    return result.errors ?? []
}

const whereFor = (variables: {where: {_and: Array<Record<string, unknown>>}}, field: string) =>
    variables.where._and.find((condition) => field in condition)?.[field]

describe("customBuildQuery: Int column filters", () => {
    it.each(["42", "0", "-1", "-2147483648", "2147483647"])(
        "sends the nullable Int filter %s as a number Hasura accepts",
        (value) => {
            const {query, variables} = getList("sequent_backend_contest", {min_votes: value})

            expect(whereFor(variables, "min_votes")).toEqual({_eq: Number(value)})
            expect(variableErrors(query, variables)).toEqual([])
        }
    )

    it("sends a non-null Int filter typed as text as a number", () => {
        const {query, variables} = getList("sequent_backend_keys_ceremony", {threshold: "3"})

        expect(whereFor(variables, "threshold")).toEqual({_eq: 3})
        expect(variableErrors(query, variables)).toEqual([])
    })

    it("keeps a filter that is already a number", () => {
        const {query, variables} = getList("sequent_backend_contest", {max_votes: 7})

        expect(whereFor(variables, "max_votes")).toEqual({_eq: 7})
        expect(variableErrors(query, variables)).toEqual([])
    })

    it("keeps String column filters as text, including leading zeros", () => {
        const {query, variables} = getList("sequent_backend_contest", {
            counting_algorithm: "00123",
            voting_type: "42",
        })

        expect(whereFor(variables, "counting_algorithm")).toEqual({_ilike: "%00123%"})
        expect(whereFor(variables, "voting_type")).toEqual({_ilike: "%42%"})
        expect(variableErrors(query, variables)).toEqual([])
    })

    it.each(["4.2", "1.0", "1e3", "", " 42", "42abc", "0x10", "2147483648", "-2147483649"])(
        "does not turn the invalid Int filter %p into another number",
        (value) => {
            const {query, variables} = getList("sequent_backend_contest", {min_votes: value})

            expect(whereFor(variables, "min_votes")).toEqual({_eq: value})
            expect(variableErrors(query, variables)).not.toEqual([])
        }
    )

    it("leaves raw Hasura filters on Int columns unchanged", () => {
        const {query, variables} = getList("sequent_backend_contest", {
            min_votes: {format: "hasura-raw-query", value: {_gte: 1}},
        })

        expect(whereFor(variables, "min_votes")).toEqual({_gte: 1})
        expect(variableErrors(query, variables)).toEqual([])
    })

    it("keeps voter attribute filters as text", () => {
        const {variables} = getList("user", {
            tenant_id: "tenant-1",
            username: "42",
            attributes: {worker_id: "00042", employee_number: "42"},
        })

        expect(variables.username).toBe("42")
        expect(variables.attributes).toEqual({"'worker_id'": "00042", "'employee_number'": "42"})
    })
})
