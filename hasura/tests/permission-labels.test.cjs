// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const {isDeepStrictEqual} = require("node:util")
const yaml = require("js-yaml")

const TABLES_DIR = resolve(__dirname, "../metadata/databases/backend-db/tables")
const PERMISSION_LABELS = "X-Hasura-Permission-Labels"
const TENANT_SCOPE = {tenant_id: {_eq: "X-Hasura-Tenant-Id"}}
const ROLES_WITHOUT_LABELS = new Set([
    "service-account",
    "super-admin-user",
    "user",
])

const labelIn = (operator) => ({
    _or: [
        {permission_label: {_is_null: true}},
        {permission_label: {[operator]: PERMISSION_LABELS}},
    ],
})
const electionLabel = labelIn("_in")
const labelList = labelIn("_contained_in")
const unlessNull = (column, predicate) => ({
    _or: [{[column]: {_is_null: true}}, predicate],
})
const viaElection = {election: electionLabel}
const viaContest = unlessNull("contest_id", {contest: viaElection})
const viaTallySession = {tally_session: labelList}

const parentKey = (column) => ({
    [column]: "id",
    election_event_id: "election_event_id",
    tenant_id: "tenant_id",
})
const parents = {
    election: parentKey("election_id"),
    contest: parentKey("contest_id"),
    tally_session: parentKey("tally_session_id"),
}

const scopedTables = {
    election: {predicate: electionLabel},
    applications: {predicate: electionLabel},
    report: {predicate: labelList},
    tally_session: {predicate: labelList},
    contest: {predicate: viaElection, parent: "election"},
    candidate: {predicate: viaContest, parent: "contest"},
    area_contest: {predicate: viaContest, parent: "contest"},
    ballot_style: {predicate: viaElection, parent: "election"},
    ballot_publication: {
        predicate: unlessNull("election_id", viaElection),
        parent: "election",
    },
    cast_vote: {
        predicate: unlessNull("election_id", viaElection),
        parent: "election",
    },
    notification: {
        predicate: unlessNull("election_id", viaElection),
        parent: "election",
    },
    results_area_contest: {predicate: viaElection, parent: "election"},
    results_area_contest_candidate: {
        predicate: viaElection,
        parent: "election",
    },
    results_contest: {predicate: viaElection, parent: "election"},
    results_contest_candidate: {predicate: viaElection, parent: "election"},
    results_election: {predicate: viaElection, parent: "election"},
    results_election_area: {predicate: viaElection, parent: "election"},
    tally_session_contest: {predicate: viaElection, parent: "election"},
    tally_session_execution: {
        predicate: viaTallySession,
        parent: "tally_session",
    },
    tally_session_resolution: {
        predicate: viaTallySession,
        parent: "tally_session",
    },
    tally_sheet: {predicate: viaElection, parent: "election"},
    tally_sheet_import_item: {predicate: viaElection, parent: "election"},
}

const loadTable = (table) =>
    yaml.load(
        readFileSync(
            resolve(TABLES_DIR, `sequent_backend_${table}.yaml`),
            "utf8"
        )
    )

const conjuncts = (expression) =>
    expression && Array.isArray(expression._and)
        ? expression._and
        : [expression]

const labelledGrants = (metadata) =>
    [
        ["select_permissions", "filter"],
        ["insert_permissions", "check"],
        ["update_permissions", "filter"],
        ["update_permissions", "check"],
        ["delete_permissions", "filter"],
    ].flatMap(([kind, key]) =>
        (metadata[kind] ?? [])
            .filter(({role}) => !ROLES_WITHOUT_LABELS.has(role))
            .map(({role, permission}) => ({
                grant: `${kind} ${role} ${key}`,
                parts: conjuncts(permission[key]),
            }))
    )

for (const [table, {predicate, parent}] of Object.entries(scopedTables)) {
    test(`${table} grants are limited to the tenant and the caller's permission labels`, () => {
        const metadata = loadTable(table)
        const grants = labelledGrants(metadata)
        assert.ok(grants.length > 0)
        for (const {grant, parts} of grants) {
            assert.ok(
                parts.some((part) => isDeepStrictEqual(part, TENANT_SCOPE)),
                `${table} ${grant} is not scoped to the tenant`
            )
            assert.ok(
                parts.some((part) => isDeepStrictEqual(part, predicate)),
                `${table} ${grant} does not check permission labels`
            )
        }
        if (parent) {
            const relationship = (metadata.object_relationships ?? []).find(
                ({name}) => name === parent
            )
            assert.ok(relationship, `${table} has no ${parent} relationship`)
            const {remote_table, column_mapping} =
                relationship.using.manual_configuration
            assert.deepEqual(remote_table, {
                name: parent,
                schema: "sequent_backend",
            })
            assert.deepEqual(column_mapping, parents[parent])
        }
    })
}

test("admin-user cannot change an application's status directly", () => {
    const update = loadTable("applications").update_permissions.find(
        ({role}) => role === "admin-user"
    )
    assert.ok(!update.permission.columns.includes("status"))
})
