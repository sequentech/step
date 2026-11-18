// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {IntrospectionObjectType, IntrospectionSchema} from "graphql"
import {fetchHeldRolesSchema, mergeSchemas} from "./heldRolesSchema"

const objectType = (name: string, fields: Array<string>) =>
    ({
        kind: "OBJECT",
        name,
        fields: fields.map((field) => ({name: field})),
    } as unknown as IntrospectionObjectType)

const schemaWith = (
    queries: Array<string>,
    types: Array<IntrospectionObjectType> = [],
    mutations?: Array<string>
) =>
    ({
        queryType: {kind: "OBJECT", name: "query_root"},
        mutationType: mutations ? {kind: "OBJECT", name: "mutation_root"} : null,
        subscriptionType: null,
        types: [
            objectType("query_root", queries),
            ...(mutations ? [objectType("mutation_root", mutations)] : []),
            ...types,
        ],
    } as unknown as IntrospectionSchema)

const fieldsOf = (schema: IntrospectionSchema, name: string) =>
    (schema.types.find((type) => type.name === name) as IntrospectionObjectType).fields.map(
        (field) => field.name
    )

describe("mergeSchemas", () => {
    it("adds the operations and types only a later role has", () => {
        const merged = mergeSchemas([
            schemaWith(["keys_ceremony"], [objectType("keys_ceremony", ["id"])]),
            schemaWith(["ballot_publication"], [objectType("ballot_publication", ["id"])]),
        ])

        expect(fieldsOf(merged, "query_root")).toEqual(["keys_ceremony", "ballot_publication"])
        expect(fieldsOf(merged, "ballot_publication")).toEqual(["id"])
    })

    it("keeps the columns of the first role for a type both roles have", () => {
        const merged = mergeSchemas([
            schemaWith(["election"], [objectType("election", ["id"])]),
            schemaWith(["election"], [objectType("election", ["id", "annotations"])]),
        ])

        expect(fieldsOf(merged, "election")).toEqual(["id"])
        expect(fieldsOf(merged, "query_root")).toEqual(["election"])
    })

    it("keeps the mutations of a later role when the first has none", () => {
        const merged = mergeSchemas([
            schemaWith(["election"]),
            schemaWith(["election"], [], ["publish"]),
        ])

        expect(merged.mutationType?.name).toBe("mutation_root")
        expect(fieldsOf(merged, "mutation_root")).toEqual(["publish"])
    })
})

describe("fetchHeldRolesSchema", () => {
    // A real client, because Apollo merges identical queries in flight whatever their headers.
    const clientFor = (schemas: Record<string, IntrospectionSchema>) => {
        const roles: Array<string> = []
        const link = new ApolloLink((operation) => {
            const role = operation.getContext().headers["x-hasura-role"]
            roles.push(role)
            return Observable.of({data: {__schema: schemas[role]}})
        })
        return {client: new ApolloClient({link, cache: new InMemoryCache()}), roles}
    }

    it("leaves a single role to the data provider", async () => {
        const {client, roles} = clientFor({})

        expect(await fetchHeldRolesSchema(client, ["publish-read"])).toBeUndefined()
        expect(roles).toEqual([])
    })

    it("reads each role's schema as that role", async () => {
        const {client, roles} = clientFor({
            "trustee-ceremony": schemaWith(["keys_ceremony"]),
            "publish-read": schemaWith(["ballot_publication"]),
        })

        const schema = await fetchHeldRolesSchema(client, ["trustee-ceremony", "publish-read"])

        expect(roles).toEqual(["trustee-ceremony", "publish-read"])
        expect(fieldsOf(schema as IntrospectionSchema, "query_root")).toEqual([
            "keys_ceremony",
            "ballot_publication",
        ])
    })
})
