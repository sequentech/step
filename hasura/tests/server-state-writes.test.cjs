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
                `../metadata/databases/backend-db/tables/sequent_backend_${table}.yaml`
            ),
            "utf8"
        )
    )

const roles = (metadata, operation) =>
    (metadata[`${operation}_permissions`] ?? []).map(({role}) => role)

const serverWrittenTables = [
    "ballot_style",
    "results_area_contest",
    "results_area_contest_candidate",
    "results_contest",
    "results_contest_candidate",
    "results_election",
    "results_election_area",
    "results_event",
]

for (const table of serverWrittenTables) {
    for (const operation of ["insert", "update", "delete"]) {
        test(`only service-account can ${operation} ${table}`, () => {
            assert.deepEqual(
                roles(loadTable(table), operation).filter(
                    (role) => role !== "service-account"
                ),
                []
            )
        })
    }
}

// Rows that only server operations write.
const serverWrittenOperations = {
    applications: ["insert", "update", "delete"],
    certificate_authority: ["delete"],
    preview: ["insert"],
    scheduled_event: ["delete"],
    tally_session_resolution: ["insert", "update"],
    tenant: ["insert", "delete"],
}

for (const [table, operations] of Object.entries(serverWrittenOperations)) {
    for (const operation of operations) {
        test(`only service-account can ${operation} ${table}`, () => {
            assert.deepEqual(
                roles(loadTable(table), operation).filter(
                    (role) => role !== "service-account"
                ),
                []
            )
        })
    }
}

for (const operation of ["insert", "update", "delete"]) {
    test(`admin-user cannot ${operation} scheduled events`, () => {
        assert.ok(
            !roles(loadTable("scheduled_event"), operation).includes(
                "admin-user"
            )
        )
    })
}

test("admin-user reads neither scheduled events nor ballot styles", () => {
    assert.ok(
        !roles(loadTable("scheduled_event"), "select").includes("admin-user")
    )
    assert.ok(
        !roles(loadTable("ballot_style"), "select").includes("admin-user")
    )
})
