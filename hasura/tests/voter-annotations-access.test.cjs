// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("../../packages/node_modules/js-yaml")

const VOTER_ROLE = "user"

const voterSelectColumns = (table) => {
    const metadata = yaml.load(
        readFileSync(
            resolve(
                __dirname,
                `../metadata/databases/backend-db/tables/${table}.yaml`
            ),
            "utf8"
        )
    )
    const permission = metadata.select_permissions.find(
        ({role}) => role === VOTER_ROLE
    )
    assert.ok(
        permission,
        `${table} has no select permission for role ${VOTER_ROLE}`
    )
    return permission.permission.columns
}

for (const table of [
    "sequent_backend_election_event",
    "sequent_backend_election",
    "sequent_backend_area",
]) {
    test(`voter select on ${table} excludes annotations`, () => {
        const columns = voterSelectColumns(table)
        assert.ok(columns.includes("id"))
        assert.ok(columns.includes("presentation"))
        assert.ok(!columns.includes("annotations"))
    })
}
