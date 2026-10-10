// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {existsSync, readFileSync, readdirSync} = require("node:fs")
const {resolve, join} = require("node:path")
const yaml = require("js-yaml")

const databases = resolve(__dirname, "../metadata/databases")
const tables = []
for (const database of readdirSync(databases, {withFileTypes: true})) {
    const directory = join(databases, database.name, "tables")
    if (!database.isDirectory() || !existsSync(directory)) continue
    for (const file of readdirSync(directory)) {
        if (!file.endsWith(".yaml") || file === "tables.yaml") continue
        const metadata = yaml.load(readFileSync(join(directory, file), "utf8"))
        assert.ok(metadata?.table, `${file} does not describe a table`)
        tables.push(metadata)
    }
}

const document = tables.find(
    ({table}) => table.schema === "sequent_backend" && table.name === "document"
)
assert.ok(document, "sequent_backend.document metadata not found")

const documentSelect = (role) => {
    const entry = document.select_permissions.find(
        (permission) => permission.role === role
    )
    assert.ok(entry, `No ${role} select permission on sequent_backend.document`)
    return entry.permission
}

// Column lists on tracked table permissions identify tenant-bearing tables.
const hasTenantColumn = (metadata) =>
    Object.entries(metadata)
        .filter(([key]) => key.endsWith("_permissions"))
        .flatMap(([, permissions]) => permissions || [])
        .some(
            ({permission}) =>
                permission.columns === "*" ||
                (Array.isArray(permission.columns) &&
                    permission.columns.includes("tenant_id"))
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

test("user role can select only public documents in its own tenant", () => {
    const permission = documentSelect("user")
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
        const permission = documentSelect(role)
        assert.deepEqual(permission.filter, {
            tenant_id: {_eq: "X-Hasura-Tenant-Id"},
        })
    }
    assert.deepEqual(documentSelect("service-account").filter, {})
})
