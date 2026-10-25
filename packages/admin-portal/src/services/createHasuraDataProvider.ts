// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {ApolloClient, NormalizedCacheObject} from "@apollo/client"
import type {IntrospectionSchema} from "graphql"
import schema from "../../graphql.schema.json"
import type {DataProvider} from "react-admin"
import buildHasuraProvider from "ra-data-hasura"
import {customBuildQuery} from "../queries/customBuildQuery"

export const createHasuraDataProvider = (
    client: ApolloClient<NormalizedCacheObject>
): Promise<DataProvider> =>
    buildHasuraProvider(
        {
            // ra-data-graphql resolves a newer Apollo type than the portal. Keep
            // the supplied client and its links; the shared query API is covered by integration tests.
            client: client as unknown as NonNullable<
                Parameters<typeof buildHasuraProvider>[0]
            >["client"],
            buildQuery: customBuildQuery,
            // Query construction must not depend on the login's default role: each
            // operation selects its own role through the existing Apollo auth link.
            // This public schema grants no access; Hasura still authorizes every request.
            introspection: {schema: schema.__schema as unknown as IntrospectionSchema},
        },
        {}
    )
