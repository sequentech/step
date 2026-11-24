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
            "../metadata/databases/backend-db/tables/sequent_backend_tally_session_resolution.yaml"
        ),
        "utf8"
    )
)

for (const operation of ["insert", "update"]) {
    test(`${operation} of tally session resolutions is limited to super-admin-user`, () => {
        const roles = metadata[`${operation}_permissions`].map(({role}) => role)
        assert.deepEqual(roles, ["super-admin-user"])
    })
}

test("tenant roles can still read tally session resolutions", () => {
    const roles = metadata.select_permissions.map(({role}) => role).sort()
    assert.deepEqual(roles, [
        "admin-user",
        "super-admin-user",
        "tally-resolution-submit",
    ])
})
