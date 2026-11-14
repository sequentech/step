// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const VOTER_ROLE = "user"

const metadata = yaml.load(
    readFileSync(
        resolve(
            __dirname,
            "../metadata/databases/backend-db/tables/sequent_backend_tenant.yaml"
        ),
        "utf8"
    )
)

test("voter select on sequent_backend_tenant excludes settings", () => {
    const permission = metadata.select_permissions.find(
        ({role}) => role === VOTER_ROLE
    )
    assert.ok(
        permission,
        `sequent_backend_tenant has no select permission for role ${VOTER_ROLE}`
    )
    assert.ok(permission.permission.columns.includes("id"))
    assert.ok(!permission.permission.columns.includes("settings"))
})
