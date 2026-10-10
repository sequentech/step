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
            "../metadata/databases/backend-db/tables/sequent_backend_ballot_style.yaml"
        ),
        "utf8"
    )
)

test("voter ballot reads require a live published style within every JWT scope", () => {
    const {permission} = metadata.select_permissions.find(
        ({role}) => role === "user"
    )
    assert.deepEqual(permission.filter, {
        _and: [
            {election_event_id: {_eq: "X-Hasura-Election-Event-Id"}},
            {tenant_id: {_eq: "X-Hasura-Tenant-Id"}},
            {area_id: {_eq: "X-Hasura-Area-Id"}},
            {election_id: {_in: "X-Hasura-Authorized-Election-Ids"}},
            {deleted_at: {_is_null: true}},
            {
                ballot_publication: {
                    published_at: {_is_null: false},
                    deleted_at: {_is_null: true},
                },
            },
        ],
    })
    // The verifier still needs the signed payload for eligible published ballots.
    assert.ok(permission.columns.includes("ballot_eml"))
    assert.ok(permission.columns.includes("ballot_signature"))
})

test("publication eligibility cannot be borrowed from another tenant or event", () => {
    const relationship = metadata.object_relationships.find(
        ({name}) => name === "ballot_publication"
    )
    assert.deepEqual(relationship?.using.manual_configuration, {
        column_mapping: {
            tenant_id: "tenant_id",
            election_event_id: "election_event_id",
            ballot_publication_id: "id",
        },
        remote_table: {name: "ballot_publication", schema: "sequent_backend"},
    })
})

test("publication managers retain tenant-scoped access to drafts and deleted styles", () => {
    for (const role of ["admin-user", "publish-read", "publish-write"]) {
        const {permission} = metadata.select_permissions.find(
            (item) => item.role === role
        )
        assert.deepEqual(permission.filter, {
            tenant_id: {_eq: "X-Hasura-Tenant-Id"},
        })
    }
})
