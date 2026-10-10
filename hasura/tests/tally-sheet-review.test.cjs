// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const metadata = yaml.load(
    readFileSync(
        resolve(
            __dirname,
            "../metadata/databases/backend-db/tables/sequent_backend_tally_sheet.yaml"
        ),
        "utf8"
    )
)
const SERVICE_ROLE = "service-account"
const SERVER_ASSIGNED_COLUMNS = [
    "id",
    "import_id",
    "reviewed_at",
    "reviewed_by_user_id",
    "status",
]
const pendingTenantFilter = {
    status: {_eq: "PENDING"},
    tenant_id: {_eq: "X-Hasura-Tenant-Id"},
}

const clientPermissions = (operation) =>
    metadata[`${operation}_permissions`].filter(
        ({role}) => role !== SERVICE_ROLE
    )

test("client roles cannot insert row ids or review columns", () => {
    const permissions = clientPermissions("insert")
    assert.deepEqual(permissions.map(({role}) => role).sort(), [
        "admin-user",
        "tally-sheet-create",
    ])
    for (const {role, permission} of permissions) {
        assert.deepEqual(
            permission.columns.filter((column) =>
                SERVER_ASSIGNED_COLUMNS.includes(column)
            ),
            [],
            role
        )
    }
})

test("only the service role updates tally sheet rows in place", () => {
    assert.deepEqual(
        metadata.update_permissions.map(({role}) => role),
        [SERVICE_ROLE]
    )
})

test("client roles delete only pending tally sheets", () => {
    const permissions = clientPermissions("delete")
    assert.deepEqual(permissions.map(({role}) => role).sort(), [
        "admin-user",
        "tally-sheet-create",
    ])
    for (const {role, permission} of permissions) {
        assert.deepEqual(permission.filter, pendingTenantFilter, role)
    }
})
