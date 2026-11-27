// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const EXTERNAL_ID_COLUMN = "external_id"
// The column the event's Data tab edits beside the external ID.
const PRESENTATION_COLUMN = "presentation"

const metadata = yaml.load(
    readFileSync(
        resolve(
            __dirname,
            "../metadata/databases/backend-db/tables/sequent_backend_election_event.yaml"
        ),
        "utf8"
    )
)

const rolesWithout = (permissions, column) =>
    permissions
        .filter(({permission}) => !permission.columns.includes(column))
        .map(({role}) => role)

test("every role that reads election events reads their external ID", () => {
    assert.ok(metadata.select_permissions.length > 0)
    assert.deepEqual(
        rolesWithout(metadata.select_permissions, EXTERNAL_ID_COLUMN),
        []
    )
})

for (const operation of ["insert", "update"]) {
    test(`every role that can ${operation} an event's presentation can ${operation} its external ID`, () => {
        const writers = metadata[`${operation}_permissions`].filter(
            ({permission}) => permission.columns.includes(PRESENTATION_COLUMN)
        )
        assert.ok(writers.length > 0)
        assert.deepEqual(rolesWithout(writers, EXTERNAL_ID_COLUMN), [])
    })
}
