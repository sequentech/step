// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const EXTERNAL_ID_COLUMN = "external_id"

// The roles release/9.0 granted the column under its former name, alias,
// before the rename to external_id dropped it from every permission.
const EXPECTED_ROLES = {
    select: [
        "admin-user",
        "election-event-archive",
        "election-event-create",
        "election-event-delete",
        "election-event-read",
        "election-event-write",
        "localization-create",
        "localization-delete",
        "localization-read",
        "localization-write",
        "publish-read",
        "service-account",
        "trustee-ceremony",
        "user",
    ],
    insert: [
        "admin-user",
        "election-event-create",
        "localization-create",
        "service-account",
    ],
    update: [
        "admin-user",
        "election-event-write",
        "localization-write",
        "service-account",
    ],
}

const metadata = yaml.load(
    readFileSync(
        resolve(
            __dirname,
            "../metadata/databases/backend-db/tables/sequent_backend_election_event.yaml"
        ),
        "utf8"
    )
)

const rolesWith = (operation) =>
    metadata[`${operation}_permissions`]
        .filter(({permission}) =>
            permission.columns.includes(EXTERNAL_ID_COLUMN)
        )
        .map(({role}) => role)
        .sort()

for (const [operation, roles] of Object.entries(EXPECTED_ROLES)) {
    test(`exactly release/9.0's roles can ${operation} an event's external ID`, () => {
        assert.deepEqual(rolesWith(operation), roles)
    })
}

test("archiving an event cannot change its external ID", () => {
    assert.ok(!rolesWith("update").includes("election-event-archive"))
})
