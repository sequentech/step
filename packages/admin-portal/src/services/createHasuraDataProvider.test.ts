// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {ApolloClient, ApolloLink, InMemoryCache, Observable} from "@apollo/client"
import {
    GraphQLError,
    type OperationDefinitionNode,
    type FieldNode,
    type IntrospectionSchema,
} from "graphql"
import {readFileSync} from "node:fs"
import {resolve} from "node:path"
import {createHasuraDataProvider, supplementTallyReadSchema} from "./createHasuraDataProvider"
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
            : type.name === "sequent_backend_tenant"
              ? {...type, fields: type.fields?.filter((field) => field.name !== "test")}
              : type.name === "sequent_backend_trustee"
                ? {
                      ...type,
                      fields: type.fields?.filter((field) =>
                          ["id", "name", "tenant_id"].includes(field.name)
                      ),
                  }
                : type
    ),
}

// Apollo must identify introspection fragments just as it does with the server's
// __typename response fields. Omitting these would erase FullType selections.
function introspectionResponse(value: unknown, kind = "__Schema"): unknown {
    if (Array.isArray(value)) return value.map((entry) => introspectionResponse(entry, kind))
    if (!value || typeof value !== "object") return value
    const kinds: Record<string, string> = {
        types: "__Type",
        queryType: "__Type",
        mutationType: "__Type",
        subscriptionType: "__Type",
        type: "__Type",
        ofType: "__Type",
        interfaces: "__Type",
        possibleTypes: "__Type",
        fields: "__Field",
        args: "__InputValue",
        inputFields: "__InputValue",
        enumValues: "__EnumValue",
        directives: "__Directive",
    }
    return {
        __typename: kind,
        ...Object.fromEntries(
            Object.entries(value).map(([key, entry]) => [
                key,
                introspectionResponse(entry, kinds[key] ?? kind),
            ])
        ),
    }
}

function fixture(deny = false, trustee = false, live = defaultRoleSchema) {
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
                    observer.next({data: {__schema: introspectionResponse(live)}})
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
        expect(requests).toHaveLength(2)
        expect(requests[0].name).toBe("IntrospectionQuery")
        expect(requests[1]).toMatchObject({name: resource, role: IPermissions.ADMIN_CEREMONY})
        expect(requests[1].variables).toMatchObject({
            where: {_and: [{tenant_id: {_eq: TENANT}}, {election_event_id: {_eq: EVENT}}]},
        })
        expect(requests[1].fields).toContain("id")
        client.stop()
    }
)

it.each(resources)("preserves the exact ID in %s detail reads", async (resource) => {
    const {client, requests} = fixture()
    const provider = await createHasuraDataProvider(client)
    await expect(provider.getOne(resource, {id: ID})).resolves.toMatchObject({data: {id: ID}})
    expect(requests).toHaveLength(2)
    expect(requests[0].name).toBe("IntrospectionQuery")
    expect(requests[1]).toMatchObject({
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
    expect(requests).toHaveLength(2)
    expect(requests[0].name).toBe("IntrospectionQuery")
    expect(requests[1].role).toBe(IPermissions.ADMIN_CEREMONY)
    client.stop()
})

it("retains trustee role routing for execution reads without changing the schema", async () => {
    const {client, requests} = fixture(false, true)
    const provider = await createHasuraDataProvider(client)
    await provider.getOne(resources[1], {id: ID})
    expect(requests).toHaveLength(2)
    expect(requests[0].name).toBe("IntrospectionQuery")
    expect(requests[1].role).toBe(IPermissions.TRUSTEE_CEREMONY)
    client.stop()
})

it("uses the live tenant columns without resurrecting the stale test field", async () => {
    const {client, requests} = fixture()
    const provider = await createHasuraDataProvider(client)
    await provider.getOne("sequent_backend_tenant", {id: ID})
    const request = requests.find((value) => value.name === "sequent_backend_tenant")!
    expect(request.fields).toContain("slug")
    expect(request.fields).not.toContain("test")
    client.stop()
})

it("preserves a restricted live resource instead of adding bundled private columns", async () => {
    const {client, requests} = fixture()
    const provider = await createHasuraDataProvider(client)
    await provider.getOne("sequent_backend_trustee", {id: ID})
    const request = requests.find((value) => value.name === "sequent_backend_trustee")!
    expect(request.fields.sort()).toEqual(["id", "name", "tenant_id"])
    expect(request.fields).not.toContain("public_key")
    client.stop()
})

it("supplements only missing tally reads and never changes live fields or mutation roots", () => {
    const live = defaultRoleSchema as IntrospectionSchema
    const combined = supplementTallyReadSchema(live)
    for (const type of live.types) {
        const result = combined.types.find((value) => value.name === type.name)!
        if (type.name !== live.queryType?.name) expect(result).toBe(type)
    }
    const before = live.types.find((type) => type.name === live.queryType?.name)!
    const after = combined.types.find((type) => type.name === live.queryType?.name)!
    if (before.kind !== "OBJECT" || after.kind !== "OBJECT") throw new Error("Missing queries")
    expect(after.fields.slice(0, before.fields.length)).toEqual(before.fields)
    expect(
        after.fields
            .slice(before.fields.length)
            .map((field) => field.name)
            .sort()
    ).toEqual(resources.flatMap((resource) => [resource, `${resource}_aggregate`]).sort())
})

it("does not broaden tally columns when the live role already exposes the resource", () => {
    const live = {
        ...schema,
        types: schema.types.map((type: {name: string; fields?: {name: string}[]}) =>
            resources.includes(type.name)
                ? {...type, fields: type.fields?.filter((field) => field.name === "id")}
                : type
        ),
    } as IntrospectionSchema
    expect(supplementTallyReadSchema(live)).toBe(live)
})

it("imports missing tally type dependencies while keeping tenant reads live", async () => {
    const live = {
        ...defaultRoleSchema,
        types: defaultRoleSchema.types.filter(
            (type: {name: string}) => !type.name.startsWith("sequent_backend_tally_session")
        ),
    }
    const {client, requests} = fixture(false, false, live)
    const provider = await createHasuraDataProvider(client)
    await expect(provider.getOne(resources[0], {id: ID})).resolves.toMatchObject({data: {id: ID}})
    await provider.getOne("sequent_backend_tenant", {id: ID})
    expect(
        requests.find((request) => request.name === "sequent_backend_tenant")!.fields
    ).not.toContain("test")
    client.stop()
})

it("fails closed when live introspection is denied instead of falling back to the bundled schema", async () => {
    const {client, requests} = fixture()
    const query = jest.spyOn(client, "query").mockRejectedValue(new Error("introspection denied"))
    await expect(createHasuraDataProvider(client)).rejects.toThrow("introspection denied")
    expect(requests).toHaveLength(0)
    query.mockRestore()
    client.stop()
})
