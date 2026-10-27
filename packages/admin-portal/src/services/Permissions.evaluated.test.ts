// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "fs"
import {resolve} from "path"
import {parse as yaml} from "yaml"
import {Kind, parse} from "graphql"
import {getOperationRole} from "./Permissions"
import {IPermissions} from "@/types/keycloak"

const root = resolve(__dirname, "../../../../")
const actions = yaml(readFileSync(resolve(root, "hasura/metadata/actions.yaml"), "utf8")).actions
const cases = [
    ["InsertElectionEvent.ts", "CreateElectionEvent", IPermissions.ELECTION_EVENT_CREATE],
    ["Lifecycle.ts", "PreviewScheduledOutcomeChange", IPermissions.ELECTION_EVENT_READ],
    ["Lifecycle.ts", "GetLifecycleSnapshots", IPermissions.ELECTION_EVENT_READ],
    ["ImportElectionEvent.ts", "ImportElectionEvent", IPermissions.ELECTION_EVENT_CREATE],
    ["CreateTallyCeremony.ts", "CreateTallyCeremony", IPermissions.ADMIN_CEREMONY],
    ["UpdateTallyCeremony.ts", "UpdateTallyCeremony", IPermissions.ADMIN_CEREMONY],
    ["PublishBallot.ts", "PublishBallot", IPermissions.PUBLISH_WRITE],
    ["GenerateBallotPublication.ts", "GenerateBallotPublication", IPermissions.PUBLISH_CREATE],
    ["UpdateEventVotingStatus.ts", "UpdateEventVotingStatus", IPermissions.ELECTION_STATE_WRITE],
    ["ExportElectionEvent.ts", "ExportElectionEvent", IPermissions.ELECTION_EXECUTION_EXPORT],
    ["DeleteElectionEvent.ts", "DeleteElectionEvent", IPermissions.ELECTION_EVENT_DELETE],
] as const

it("reads the signed electoral log with the board's log permission", () => {
    const selected = getOperationRole(
        {operationName: "listElectoralLog"} as never,
        (role) => role === IPermissions.PUBLISH_READ || role === IPermissions.LOGS_READ
    )
    expect(selected).toBe(IPermissions.LOGS_READ)
    expect(
        actions.find((value: {name: string}) => value.name === "listElectoralLog").permissions
    ).toContainEqual({role: selected})
})

it.each(cases)(
    "routes the actual %s document's %s through its held action grant",
    (file, name, required) => {
        const source = readFileSync(resolve(__dirname, "../queries", file), "utf8")
        const definitions = Array.from(source.matchAll(/(?:gql|graphql)`([\s\S]*?)`/g)).flatMap(
            (match) => parse(match[1]).definitions
        )
        const definition = definitions.find(
            (value) => value.kind === Kind.OPERATION_DEFINITION && value.name?.value === name
        )
        expect(definition?.kind).toBe(Kind.OPERATION_DEFINITION)
        if (definition?.kind !== Kind.OPERATION_DEFINITION)
            throw new Error("Missing real operation")
        const roles = new Set<IPermissions>([IPermissions.PUBLISH_READ, required])
        const selected = getOperationRole(
            {operationName: name, query: {kind: Kind.DOCUMENT, definitions: [definition]}},
            (role) => roles.has(role)
        )
        expect(selected).toBe(required)
        for (const selection of definition.selectionSet.selections) {
            if (selection.kind !== Kind.FIELD) throw new Error("Expected direct action selection")
            const action = actions.find(
                (value: {name: string}) => value.name === selection.name.value
            )
            expect(action?.permissions.map((value: {role: string}) => value.role)).toContain(
                selected
            )
        }
        expect(
            getOperationRole(
                {operationName: name} as never,
                (role) => role === IPermissions.PUBLISH_READ
            )
        ).toBe(IPermissions.PUBLISH_READ)
    }
)

it("does not grant election creation to the administrator projection", () => {
    const create = actions.find((value: {name: string}) => value.name === "insertElectionEvent")
    expect(create.permissions).toEqual([{role: "election-event-create"}])
})

it.each([
    "import-election-data",
    "start-tally",
    "export-execution-data",
    "delete-execution-data",
    "read-voters-register",
])(
    "lets a %s signer read only its own action requests and scoped approval identities",
    (action) => {
        for (const table of ["signing_request", "signing_approval"]) {
            const metadata = yaml(
                readFileSync(
                    resolve(
                        root,
                        "hasura/metadata/databases/backend-db/tables/sequent_backend_" +
                            table +
                            ".yaml"
                    ),
                    "utf8"
                )
            )
            const permission = metadata.select_permissions.find(
                (value: {role: string}) => value.role === "sign-" + action
            )?.permission
            expect(permission).toBeDefined()
            expect(permission.filter._and[0]).toEqual({tenant_id: {_eq: "X-Hasura-Tenant-Id"}})
            const scope =
                table === "signing_request"
                    ? permission.filter._and.slice(1)
                    : permission.filter._and[1].request._and
            expect(scope[0]).toEqual({action: {_eq: action}})
            expect(scope[1]).toEqual({
                _or: [
                    {permission_label: {_is_null: true}},
                    {permission_label: {_in: "X-Hasura-Permission-Labels"}},
                ],
            })
            expect(permission.columns).not.toContain("signature")
            expect(permission.columns).not.toContain("payload")
            expect(permission.columns).not.toContain("certificate_pem")
        }
    }
)
