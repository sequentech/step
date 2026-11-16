// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloClient, gql, NormalizedCacheObject} from "@apollo/client"
import {
    getIntrospectionQuery,
    IntrospectionQuery,
    IntrospectionSchema,
    IntrospectionType,
} from "graphql"

const mergeSchema = (
    base: IntrospectionSchema,
    extra: IntrospectionSchema
): IntrospectionSchema => {
    const roots = new Set(
        [extra.queryType, extra.mutationType, extra.subscriptionType].map((root) => root?.name)
    )
    const types = new Map<string, IntrospectionType>(base.types.map((type) => [type.name, type]))
    extra.types.forEach((type) => {
        const known = types.get(type.name)
        if (!known) {
            types.set(type.name, type)
        } else if (roots.has(type.name) && known.kind === "OBJECT" && type.kind === "OBJECT") {
            const names = new Set(known.fields.map(({name}) => name))
            types.set(type.name, {
                ...known,
                fields: [...known.fields, ...type.fields.filter(({name}) => !names.has(name))],
            })
        }
    })
    return {
        ...base,
        mutationType: base.mutationType ?? extra.mutationType,
        subscriptionType: base.subscriptionType ?? extra.subscriptionType,
        types: Array.from(types.values()),
    }
}

/**
 * Joins the schemas of several roles. Root types gain the operations of every role. Any
 * other type keeps the fields of the first role that has it, so a query doesn't select a
 * column that the role running it can't read.
 */
export const mergeSchemas = (schemas: Array<IntrospectionSchema>): IntrospectionSchema =>
    schemas.reduce(mergeSchema)

/**
 * Hasura shows each role only its own tables, and the data provider builds its queries
 * from one schema. A user with several roles needs them all, or the provider rejects
 * resources that the role running the query can read. Returns nothing for a single role,
 * which the data provider introspects by itself.
 */
export const fetchHeldRolesSchema = async (
    client: ApolloClient<NormalizedCacheObject>,
    roles: Array<string>
): Promise<IntrospectionSchema | undefined> => {
    if (roles.length < 2) {
        return undefined
    }
    const schemas = await Promise.all(
        roles.map(async (role) => {
            const {data} = await client.query<IntrospectionQuery>({
                query: gql(getIntrospectionQuery()),
                fetchPolicy: "no-cache",
                // Apollo otherwise merges these identical queries, ignoring their headers.
                context: {headers: {"x-hasura-role": role}, queryDeduplication: false},
            })
            return data.__schema
        })
    )
    return mergeSchemas(schemas)
}
