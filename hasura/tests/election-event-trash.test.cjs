// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const metadata = (file) =>
    readFileSync(resolve(__dirname, "../metadata", file), "utf8")
const electionEvent = yaml.load(
    metadata("databases/backend-db/tables/sequent_backend_election_event.yaml")
)
const actions = yaml.load(metadata("actions.yaml")).actions
const trashColumns = [
    "trashed_at",
    "purge_after",
    "archived_before_trash",
    "purge_started_at",
]

const clientPermissions = (operation) =>
    (electionEvent[`${operation}_permissions`] ?? []).filter(
        ({role}) => role !== "service-account"
    )

// Whether a permission filter requires the row not to be in Trash for every
// row it lets through.
const excludesTrash = (filter) =>
    filter?.trashed_at?._is_null === true ||
    (Array.isArray(filter?._and) && filter._and.some(excludesTrash))

test("no client role can delete an election event directly; deletion moves it to Trash", () => {
    assert.deepEqual(electionEvent.delete_permissions ?? [], [])
})

for (const operation of ["select", "update"]) {
    test(`every client role's ${operation} excludes election events in Trash`, () => {
        const permissions = clientPermissions(operation)
        assert.ok(permissions.length > 0)
        for (const {role, permission} of permissions) {
            assert.ok(excludesTrash(permission.filter), `${role} ${operation}`)
        }
    })
}

for (const operation of ["select", "insert", "update"]) {
    test(`no client role can ${operation} the Trash columns`, () => {
        for (const {role, permission} of clientPermissions(operation)) {
            const exposed = (permission.columns ?? []).filter((column) =>
                trashColumns.includes(column)
            )
            assert.deepEqual(exposed, [], `${role} ${operation}`)
        }
    })
}

test("voters only see their election event while it is not in Trash", () => {
    const voter = clientPermissions("select").find(({role}) => role === "user")
    assert.ok(excludesTrash(voter.permission.filter))
})

test("Trash listing and restore are actions of the delete permission only", () => {
    for (const name of [
        "list_trashed_election_events",
        "restore_election_event",
    ]) {
        const action = actions.find((candidate) => candidate.name === name)
        assert.ok(action, name)
        assert.deepEqual(action.permissions.map(({role}) => role).sort(), [
            "admin-user",
            "election-event-delete",
        ])
    }
})
