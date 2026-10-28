// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const loadTable = (table) =>
    yaml.load(
        readFileSync(
            resolve(
                __dirname,
                `../metadata/databases/backend-db/tables/${table}.yaml`
            ),
            "utf8"
        )
    )

const columnsFor = (metadata, operation, role) =>
    metadata[`${operation}_permissions`].find((entry) => entry.role === role)
        .permission.columns

const windmillOnlyColumns = [
    {table: "sequent_backend_tally_session", column: "annotations"},
    {table: "sequent_backend_results_event", column: "documents"},
]

for (const {table, column} of windmillOnlyColumns) {
    const metadata = loadTable(table)
    for (const operation of ["insert", "update"]) {
        test(`${table}.${column} is not writable by admin-user on ${operation}`, () => {
            assert.ok(
                !columnsFor(metadata, operation, "admin-user").includes(column)
            )
        })
        test(`${table}.${column} stays writable by service-account on ${operation}`, () => {
            assert.ok(
                columnsFor(metadata, operation, "service-account").includes(
                    column
                )
            )
        })
    }
}
