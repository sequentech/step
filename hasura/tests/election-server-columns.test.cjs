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
            "../metadata/databases/backend-db/tables/sequent_backend_election.yaml"
        ),
        "utf8"
    )
)
const SERVER_ROLE = "service-account"
const SERVER_COLUMNS = ["initialization_report_generated"]

for (const operation of ["insert", "update"]) {
    test(`only ${SERVER_ROLE} can ${operation} server-maintained election columns`, () => {
        for (const {role, permission} of metadata[`${operation}_permissions`]) {
            assert.deepEqual(
                SERVER_COLUMNS.filter((column) =>
                    permission.columns.includes(column)
                ),
                role === SERVER_ROLE ? SERVER_COLUMNS : [],
                role
            )
        }
    })
}

test("election readers still see server-maintained election columns", () => {
    const admin = metadata.select_permissions.find(
        ({role}) => role === "admin-user"
    )
    for (const column of SERVER_COLUMNS) {
        assert.ok(admin.permission.columns.includes(column), column)
    }
})
