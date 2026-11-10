// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {existsSync, readFileSync} from "fs"
import {resolve} from "path"
import {parse as yaml} from "yaml"
import {
    Kind,
    buildClientSchema,
    parse,
    print,
    validate,
    type DocumentNode,
    type SelectionSetNode,
    type TypeNode,
    type ValueNode,
} from "graphql"
import {administratorSettingsQuery} from "./administratorSettings"
import {getOperationRole} from "./Permissions"
import {IPermissions} from "@/types/keycloak"

const metadata = resolve(__dirname, "../../../../hasura/metadata")
const actions: Array<{name: string; permissions?: Array<{role: string}>}> = yaml(
    readFileSync(resolve(metadata, "actions.yaml"), "utf8")
).actions
const actionTypes = parse(readFileSync(resolve(metadata, "actions.graphql"), "utf8"))
const schema = buildClientSchema(
    JSON.parse(readFileSync(resolve(__dirname, "../../graphql.schema.json"), "utf8"))
)

/** The select permission of `role` on the table behind a root field, as in the metadata. */
const selectPermission = (rootField: string, role: string) => {
    const file = resolve(
        metadata,
        "databases/backend-db/tables",
        `${rootField.replace(/_by_pk$/, "")}.yaml`
    )
    if (!existsSync(file)) throw new Error(`No table metadata for ${rootField}`)
    return yaml(readFileSync(file, "utf8")).select_permissions?.find(
        (entry: {role: string}) => entry.role === role
    )?.permission as {columns: string[]} | undefined
}

const scalarFields = (set: SelectionSetNode | undefined): string[] =>
    (set?.selections ?? []).flatMap((selection) => {
        if (selection.kind !== Kind.FIELD || selection.selectionSet)
            throw new Error("Only scalar fields are checked here")
        return selection.name.value === "__typename" ? [] : [selection.name.value]
    })

/** Columns that a `where`, `order_by` or `distinct_on` argument names inline. */
const argumentColumns = (argument: string, value: ValueNode): string[] => {
    if (value.kind === Kind.LIST)
        return value.values.flatMap((entry) => argumentColumns(argument, entry))
    if (argument === "distinct_on") return value.kind === Kind.ENUM ? [value.value] : []
    if (value.kind !== Kind.OBJECT) return []
    return value.fields.flatMap((field) =>
        argument === "where" && ["_and", "_or", "_not"].includes(field.name.value)
            ? argumentColumns(argument, field.value)
            : [field.name.value]
    )
}

const namedType = (type: TypeNode): string =>
    type.kind === Kind.NAMED_TYPE ? type.name.value : namedType(type.type)

const actionOutputFields = (action: string): Set<string> => {
    const objects = actionTypes.definitions.flatMap((definition) =>
        definition.kind === Kind.OBJECT_TYPE_DEFINITION ? [definition] : []
    )
    const field = objects
        .filter((object) => ["Query", "Mutation"].includes(object.name.value))
        .flatMap((object) => object.fields ?? [])
        .find((candidate) => candidate.name.value === action)
    if (!field) throw new Error(`No action type for ${action}`)
    const output = objects.find((object) => object.name.value === namedType(field.type))
    return new Set(output?.fields?.map((entry) => entry.name.value))
}

/** What Hasura would refuse to `role` in `document`, read from the metadata: each
 * column that the role may not select (selected, filtered or sorted by), each table
 * it may not read, each action it may not call and each field an action does not return. */
const refusals = (document: DocumentNode, role: string): string[] =>
    document.definitions.flatMap((definition) => {
        if (definition.kind !== Kind.OPERATION_DEFINITION) throw new Error("Operations only")
        return definition.selectionSet.selections.flatMap((selection) => {
            if (selection.kind !== Kind.FIELD) throw new Error("Root fields only")
            const root = selection.name.value
            if (actions.some((action) => action.name === root)) {
                const action = actions.find((entry) => entry.name === root)
                const returned = actionOutputFields(root)
                return [
                    ...(action?.permissions?.some((entry) => entry.role === role)
                        ? []
                        : [`${role} may not call ${root}`]),
                    ...scalarFields(selection.selectionSet)
                        .filter((field) => !returned.has(field))
                        .map((field) => `${root} does not return ${field}`),
                ]
            }
            const permission = selectPermission(root, role)
            if (!permission) return [`${role} may not read ${root}`]
            const used = [
                ...(selection.arguments ?? []).flatMap((argument) =>
                    ["where", "order_by", "distinct_on"].includes(argument.name.value)
                        ? argumentColumns(argument.name.value, argument.value)
                        : []
                ),
                ...scalarFields(selection.selectionSet),
            ]
            return Array.from(new Set(used))
                .filter((column) => !permission.columns.includes(column))
                .map((column) => `${role} may not select ${root}.${column}`)
        })
    })

/** A document exported by a source file, and the role its requests name. */
const sentBy = (file: string, constant: string) => {
    const source = readFileSync(resolve(__dirname, "..", file), "utf8")
    const text = source.match(new RegExp(`export const ${constant} = gql\`([\\s\\S]*?)\``))?.[1]
    const role = source.match(/"x-hasura-role": IPermissions\.(\w+)/)?.[1]
    if (!text || !role) throw new Error(`No ${constant} with a role in ${file}`)
    return {document: parse(text), role: IPermissions[role as keyof typeof IPermissions]}
}
/** What the portal's Apollo link sends for an account that holds `admin-user`. */
const asSent = (document: DocumentNode, variables: Record<string, unknown>) =>
    print(administratorSettingsQuery(document, {...variables}))

const revoting = "resources/ElectionEvent/ElectionRevotingSettings.tsx"
const variables = {electionEventId: "event", electionId: "election"}

it.each(["GET_ELECTION_REVOTING", "DISABLE_ELECTION_REVOTING"])(
    "sends the re-voting section's %s as written, with only what its role may use",
    (constant) => {
        const {document, role} = sentBy(revoting, constant)
        expect(role).toBe(IPermissions.ADMIN_USER)
        expect(validate(schema, document)).toEqual([])
        expect(refusals(document, role)).toEqual([])
        expect(asSent(document, variables)).toBe(print(document))
    }
)

it("names the election from the columns the administrator's role selects", () => {
    expect(selectPermission("sequent_backend_election", IPermissions.ADMIN_USER)?.columns).toEqual(
        expect.arrayContaining(["id", "election_event_id", "administration_settings"])
    )
    expect(
        selectPermission("sequent_backend_election", IPermissions.ADMIN_USER)?.columns
    ).not.toContain("presentation")
})

it("refuses a document that selects a column the role lacks", () => {
    const election = (field: string) =>
        parse(`query Q($electionEventId: uuid!) {
            sequent_backend_election(
                where: {election_event_id: {_eq: $electionEventId}}
                order_by: {name: asc}
            ) { id ${field} num_allowed_revotes }
        }`)
    expect(refusals(election("presentation"), IPermissions.ADMIN_USER)).toEqual([
        "admin-user may not select sequent_backend_election.name",
        "admin-user may not select sequent_backend_election.presentation",
    ])
    expect(refusals(election("alias"), IPermissions.LOGS_READ)).toEqual([
        "logs-read may not read sequent_backend_election",
    ])
    expect(
        refusals(
            parse(`mutation M { disable_election_revoting(election_event_id: "e", election_id: "x")
                { election_id status } }`),
            IPermissions.PUBLISH_READ
        )
    ).toEqual([
        "publish-read may not call disable_election_revoting",
        "disable_election_revoting does not return status",
    ])
    // The settings link renames an unaliased projection, so such a document would
    // receive its rows under another name than the one it asked for.
    const unaliased = parse(`query Q { sequent_backend_election { id administration_settings } }`)
    expect(refusals(unaliased, IPermissions.ADMIN_USER)).toEqual([])
    expect(asSent(unaliased, {})).not.toBe(print(unaliased))
})

// The side menu's trustee entry, the trustee page, the scheduled events and the
// look and feel read the tenant row that react-admin loads (`sequent_backend_tenant`).
// Its fields come from the schema of the role that loads the schema.
it.each([
    ["administrator", [IPermissions.ADMIN_USER, IPermissions.TENANT_WRITE]],
    ["trustee", [IPermissions.TRUSTEE_CEREMONY, IPermissions.PUBLISH_READ]],
    ["election board member", [IPermissions.PUBLISH_READ, IPermissions.ELECTION_EVENT_READ]],
    ["account without a default role", [IPermissions.ELECTION_EVENT_READ]],
] as const)("lets the %s's tenant read select the settings and annotations", (_, held) => {
    const has = (role: IPermissions) => (held as readonly IPermissions[]).includes(role)
    const reader = getOperationRole({operationName: "sequent_backend_tenant"} as never, has)
    const schemaRole = getOperationRole({operationName: "IntrospectionQuery"} as never, has)
    const columns = selectPermission("sequent_backend_tenant", reader)?.columns ?? []
    expect(columns).toEqual(expect.arrayContaining(["id", "settings", "annotations"]))
    expect(columns).toEqual(
        expect.arrayContaining(
            selectPermission("sequent_backend_tenant", schemaRole)?.columns ?? []
        )
    )
})
