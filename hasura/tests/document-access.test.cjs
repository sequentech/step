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
const excludeBallotReceipts = {
    _and: [
        {
            _or: [
                {name: {_is_null: true}},
                {name: {_nilike: "ballot\\_receipt\\_%"}},
            ],
        },
        {
            _or: [
                {annotations: {_is_null: true}},
                {_not: {annotations: {_contains: {purpose: "ballot-receipt"}}}},
            ],
        },
    ],
}
// Participation and voter exports remain immutable even for service accounts.
const immutableExportFilters = [
    {
        _or: [
            {annotations: {_is_null: true}},
            {_not: {annotations: {_has_key: "participation_scope"}}},
        ],
    },
    {
        _or: [{name: {_is_null: true}}, {name: {_nilike: "users-export-%"}}],
    },
    {
        _or: [
            {name: {_is_null: true}},
            {name: {_nilike: "election-event-%-export-sha256-%"}},
        ],
    },
    {
        _or: [
            {annotations: {_is_null: true}},
            {_not: {annotations: {_contains: {purpose: "voter-export"}}}},
        ],
    },
]
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
                {
                    _and:
                        role === "service-account"
                            ? immutableExportFilters
                            : [
                                  protectedFilter,
                                  excludeBallotReceipts,
                                  ...immutableExportFilters,
                              ],
                },
                `${role} ${operation} must preserve document privacy restrictions`
            )
        }
    })
}
