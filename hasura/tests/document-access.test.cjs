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
            "../metadata/databases/backend-db/tables/sequent_backend_document.yaml"
        ),
        "utf8"
    )
)
const protectedFilter = {
    _and: [
        {tenant_id: {_eq: "X-Hasura-Tenant-Id"}},
        {
            _or: [
                {annotations: {_is_null: true}},
                {
                    _not: {
                        annotations: {
                            _contains: {
                                access: {voter_secret_attributes: true},
                            },
                        },
                    },
                },
            ],
        },
    ],
}
for (const operation of ["update", "delete"]) {
    test(`${operation} cannot strip or replace classified document metadata`, () => {
        const permissions = metadata[`${operation}_permissions`]
        assert.deepEqual(permissions.map(({role}) => role).sort(), [
            "document-write",
            "service-account",
        ])
        for (const {role, permission} of permissions) {
            assert.deepEqual(
                permission.filter,
                role === "service-account" ? {} : protectedFilter
            )
        }
    })
}

test("the default admin role neither reads nor writes documents", () => {
    for (const operation of ["select", "insert", "update", "delete"]) {
        assert.ok(
            !(metadata[`${operation}_permissions`] ?? []).some(
                ({role}) => role === "admin-user"
            ),
            operation
        )
    }
})

test("board document roles retain their existing event scope", () => {
    for (const role of [
        "document-read",
        "document-write",
        "trustee-ceremony",
    ]) {
        assert.deepEqual(
            metadata.select_permissions.find((entry) => entry.role === role)
                .permission.filter,
            {tenant_id: {_eq: "X-Hasura-Tenant-Id"}}
        )
    }
    assert.deepEqual(
        metadata.insert_permissions.find(({role}) => role === "document-write")
            .permission.check,
        {tenant_id: {_eq: "X-Hasura-Tenant-Id"}}
    )
})
