// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("../../packages/node_modules/js-yaml")

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
            "admin-user",
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
