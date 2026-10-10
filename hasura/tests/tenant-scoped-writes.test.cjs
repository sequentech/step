// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readdirSync, readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const TABLES_DIR = resolve(__dirname, "../metadata/databases/backend-db/tables")
const TENANT_SESSION_VARIABLE = "X-Hasura-Tenant-Id"
const CROSS_TENANT_ROLES = new Set(["service-account", "super-admin-user"])

const tables = readdirSync(TABLES_DIR)
    .filter((file) => file.startsWith("sequent_backend_"))
    .map((file) => yaml.load(readFileSync(resolve(TABLES_DIR, file), "utf8")))

const tenantColumn = ({table}) => (table.name === "tenant" ? "id" : "tenant_id")

const scopesToTenant = (expression, column) =>
    expression?.[column]?._eq === TENANT_SESSION_VARIABLE ||
    (expression?._and ?? []).some((term) => scopesToTenant(term, column))

const tenantRoleGrants = (operation) =>
    tables.flatMap((metadata) =>
        (metadata[`${operation}_permissions`] ?? [])
            .filter(({role}) => !CROSS_TENANT_ROLES.has(role))
            .map(({role, permission}) => ({
                grant: `${metadata.table.name} ${operation} ${role}`,
                column: tenantColumn(metadata),
                permission,
            }))
    )

const unscopedGrants = (grants, key) =>
    grants
        .filter(
            ({column, permission}) => !scopesToTenant(permission[key], column)
        )
        .map(({grant}) => grant)

for (const operation of ["update", "delete"]) {
    test(`${operation} grants only match rows of the caller's tenant`, () => {
        assert.deepEqual(
            unscopedGrants(tenantRoleGrants(operation), "filter"),
            []
        )
    })
}

for (const operation of ["insert", "update"]) {
    test(`${operation} grants keep written rows in the caller's tenant`, () => {
        const writingGrants = tenantRoleGrants(operation).filter(
            ({permission}) => permission.columns?.length > 0
        )
        assert.deepEqual(unscopedGrants(writingGrants, "check"), [])
    })
}
