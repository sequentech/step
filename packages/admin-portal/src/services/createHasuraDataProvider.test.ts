// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {GraphQLError, type OperationDefinitionNode, type FieldNode} from "graphql"
import {readFileSync} from "node:fs"
import {resolve} from "node:path"
import {createHasuraDataProvider} from "./createHasuraDataProvider"
import {getOperationRole} from "./Permissions"
import {IPermissions} from "../types/keycloak"

const schema = JSON.parse(
    readFileSync(resolve(__dirname, "../../graphql.schema.json"), "utf8")
).__schema
const resources = ["sequent_backend_tally_session", "sequent_backend_tally_session_execution"]
const ID = "30000000-0000-4000-8000-000000000001"
const EVENT = "40000000-0000-4000-8000-000000000001"
const TENANT = "50000000-0000-4000-8000-000000000001"

// Reproduce startup under publish-read: its schema has no tally resources.
const defaultRoleSchema = {
    ...schema,
    types: schema.types.map((type: {name: string; fields?: {name: string}[]}) =>
        type.name === schema.queryType.name || type.name === schema.mutationType.name
            ? {
                  ...type,
                  fields: type.fields?.filter((field) => !field.name.includes("tally_session")),
              }
            : type
    ),
}

function fixture(deny = false, trustee = false) {
    const requests: {
        name: string
        role: string
        fields: string[]
        variables: Record<string, unknown>
    }[] = []
    const transport = new ApolloLink(
        (operation) =>
            new Observable((observer) => {
                const role = getOperationRole(
                    operation,
                    (permission) =>
                        permission === IPermissions.ADMIN_CEREMONY ||
                        permission === IPermissions.PUBLISH_READ ||
                        (trustee && permission === IPermissions.TRUSTEE_CEREMONY)
                )
                if (operation.operationName === "IntrospectionQuery") {
                    requests.push({
                        name: operation.operationName,
                        role,
                        fields: [],
                        variables: operation.variables,
                    })
                    observer.next({data: {__schema: defaultRoleSchema}})
                } else {
                    const definition = operation.query.definitions[0] as OperationDefinitionNode
                    const rows = definition.selectionSet.selections.find(
                        (selection) =>
                            selection.kind === "Field" &&
                            ["items", "returning"].includes(selection.alias?.value ?? "")
                    ) as FieldNode
                    requests.push({
                        name: operation.operationName,
                        role,
                        fields: rows.selectionSet!.selections.map(
                            (selection) => (selection as FieldNode).name.value
                        ),
                        variables: operation.variables,
                    })
                    if (deny) observer.next({errors: [new GraphQLError("permission denied")]})
                    else {
                        const row = Object.fromEntries(
                            requests[requests.length - 1].fields.map((field) => [
                                field,
                                field === "id" ? ID : null,
                            ])
                        )
                        observer.next({
                            data:
                                rows.alias?.value === "returning"
                                    ? {returning: [row]}
                                    : {items: [], total: {aggregate: {count: 0}}},
                        })
                    }
                }
                observer.complete()
            })
    )
    const client = new ApolloClient({
        link: transport,
        cache: new InMemoryCache({addTypename: false}),
    })
    return {client, requests}
}

it.each(resources)(
    "constructs %s lists independently of the default role's schema",
    async (resource) => {
        const {client, requests} = fixture()
        const provider = await createHasuraDataProvider(client)
        await expect(
            provider.getList(resource, {
                pagination: {page: 1, perPage: 10},
                sort: {field: "created_at", order: "DESC"},
                filter: {tenant_id: TENANT, election_event_id: EVENT},
            })
        ).resolves.toEqual({data: [], total: 0})
        expect(requests).toHaveLength(1)
        expect(requests[0]).toMatchObject({name: resource, role: IPermissions.ADMIN_CEREMONY})
        expect(requests[0].variables).toMatchObject({
            where: {_and: [{tenant_id: {_eq: TENANT}}, {election_event_id: {_eq: EVENT}}]},
        })
        expect(requests[0].fields).toContain("id")
        client.stop()
    }
)

it.each(resources)("preserves the exact ID in %s detail reads", async (resource) => {
    const {client, requests} = fixture()
    const provider = await createHasuraDataProvider(client)
    await expect(provider.getOne(resource, {id: ID})).resolves.toMatchObject({data: {id: ID}})
    expect(requests).toHaveLength(1)
    expect(requests[0]).toMatchObject({
        name: resource,
        role: IPermissions.ADMIN_CEREMONY,
        variables: {where: {id: {_eq: ID}}},
    })
    client.stop()
})

it("propagates server authorization refusal without retrying with a stronger role", async () => {
    const {client, requests} = fixture(true)
    const provider = await createHasuraDataProvider(client)
    await expect(provider.getOne(resources[0], {id: ID})).rejects.toThrow("permission denied")
    expect(requests).toHaveLength(1)
    expect(requests[0].role).toBe(IPermissions.ADMIN_CEREMONY)
    client.stop()
})

it("retains trustee role routing for execution reads without changing the schema", async () => {
    const {client, requests} = fixture(false, true)
    const provider = await createHasuraDataProvider(client)
    await provider.getOne(resources[1], {id: ID})
    expect(requests).toHaveLength(1)
    expect(requests[0].role).toBe(IPermissions.TRUSTEE_CEREMONY)
    client.stop()
})
