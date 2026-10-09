// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync, readdirSync} = require("node:fs")
const {resolve, join} = require("node:path")
const yaml = require("js-yaml")

const databases = resolve(__dirname, "../metadata/databases")
const tables = []
for (const database of readdirSync(databases, {withFileTypes: true})) {
    if (!database.isDirectory()) continue
    const directory = join(databases, database.name, "tables")
    for (const file of readdirSync(directory)) {
        if (!file.endsWith(".yaml") || file === "tables.yaml") continue
        tables.push(yaml.load(readFileSync(join(directory, file), "utf8")))
    }
}

const document = tables.find(
    ({table}) => table.schema === "sequent_backend" && table.name === "document"
)

// Column lists on tracked table permissions identify tenant-bearing tables.
const hasTenantColumn = (metadata) =>
    Object.entries(metadata)
        .filter(([key]) => key.endsWith("_permissions"))
        .flatMap(([, permissions]) => permissions)
        .some(
            ({permission}) =>
                Array.isArray(permission.columns) &&
                permission.columns.includes("tenant_id")
        )

test("user selects on tenant tables cannot have an empty row filter", () => {
    const unfiltered = tables
        .filter(hasTenantColumn)
        .flatMap((metadata) =>
            (metadata.select_permissions || [])
                .filter(
                    ({role, permission}) =>
                        role === "user" &&
                        (!permission.filter ||
                            Object.keys(permission.filter).length === 0)
                )
                .map(() => `${metadata.table.schema}.${metadata.table.name}`)
        )
    assert.deepEqual(
        unfiltered,
        [],
        "Unrestricted user selects: " + unfiltered.join(", ")
    )
})

test("voters can select only public documents in their own tenant", () => {
    const permission = document.select_permissions.find(
        ({role}) => role === "user"
    ).permission
    assert.deepEqual(permission.filter, {
        tenant_id: {_eq: "X-Hasura-Tenant-Id"},
        is_public: {_eq: true},
    })
    assert.equal(permission.allow_aggregations, false)
    // Keep public tenant-level documents available: no mandatory event filter.
    assert.equal(Object.hasOwn(permission.filter, "election_event_id"), false)
})

test("existing administrative document access remains tenant scoped", () => {
    for (const role of ["admin-user", "document-read", "document-write"]) {
        const permission = document.select_permissions.find(
            (entry) => entry.role === role
        ).permission
        assert.deepEqual(permission.filter, {
            tenant_id: {_eq: "X-Hasura-Tenant-Id"},
        })
    }
    const service = document.select_permissions.find(
        ({role}) => role === "service-account"
    ).permission
    assert.deepEqual(service.filter, {})
})
