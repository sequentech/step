// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql, type ApolloClient, type NormalizedCacheObject} from "@apollo/client"
import {getIntrospectionQuery, type IntrospectionSchema} from "graphql"
import schema from "../../graphql.schema.json"
import type {DataProvider} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import {customBuildQuery} from "../queries/customBuildQuery"

const bundled = schema.__schema as unknown as IntrospectionSchema
const tallyReads = ["sequent_backend_tally_session", "sequent_backend_tally_session_execution"]

/** Keep the current role's schema; supplement only absent, separately routed tally reads. */
export function supplementTallyReadSchema(live: IntrospectionSchema): IntrospectionSchema {
    const liveQuery = live.types.find((type) => type.name === live.queryType?.name)
    const bundledQuery = bundled.types.find((type) => type.name === bundled.queryType?.name)
    if (liveQuery?.kind !== "OBJECT" || bundledQuery?.kind !== "OBJECT") {
        throw new Error("Query schema is unavailable")
    }
    const names = new Set(liveQuery.fields.map((field) => field.name))
    const missing = tallyReads.filter((resource) => !names.has(resource))
    const additions = bundledQuery.fields.filter(
        (field) =>
            missing.some((resource) => [resource, `${resource}_aggregate`].includes(field.name)) &&
            !names.has(field.name)
    )
    if (!additions.length) return live

    // Import only dependencies of these read fields. Existing live definitions,
    // including restricted column sets and every mutation, remain untouched.
    const types = [...live.types]
    const known = new Set(types.map((type) => type.name))
    const addReferences = (value: unknown): void => {
        if (!value || typeof value !== "object") return
        if (Array.isArray(value)) {
            value.forEach(addReferences)
            return
        }
        const reference = value as {kind?: string; name?: string}
        if (reference.kind && reference.name && !known.has(reference.name)) {
            const dependency = bundled.types.find((type) => type.name === reference.name)
            if (!dependency) throw new Error("Tally schema dependency is unavailable")
            known.add(dependency.name)
            types.push(dependency)
            addReferences(dependency)
        }
        Object.values(value).forEach(addReferences)
    }
    additions.forEach(addReferences)
    return {
        ...live,
        types: types.map((type) =>
            type.name === liveQuery.name
                ? {...liveQuery, fields: [...liveQuery.fields, ...additions]}
                : type
        ),
    }
}

export const createHasuraDataProvider = async (
    client: ApolloClient<NormalizedCacheObject>
): Promise<DataProvider> => {
    const {data} = await client.query<{__schema: IntrospectionSchema}>({
        query: gql(getIntrospectionQuery()),
        fetchPolicy: "network-only",
    })
    return buildHasuraProvider(
        {
            // ra-data-graphql resolves a newer Apollo type than the portal. Keep
            // the supplied client and its links; integration tests cover this API.
            client: client as unknown as NonNullable<
                Parameters<typeof buildHasuraProvider>[0]
            >["client"],
            buildQuery: customBuildQuery,
            // This constructs queries only. Existing per-operation role routing
            // and server-side authorization still govern every request.
            introspection: {schema: supplementTallyReadSchema(data.__schema)},
        },
        {}
    )
}
