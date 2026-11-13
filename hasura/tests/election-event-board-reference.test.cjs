// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const BOARD_REFERENCE_COLUMN = "bulletin_board_reference"

const metadata = yaml.load(
    readFileSync(
        resolve(
            __dirname,
            "../metadata/databases/backend-db/tables/sequent_backend_election_event.yaml"
        ),
        "utf8"
    )
)

test("only the service account updates the bulletin board reference", () => {
    const roles = metadata.update_permissions
        .filter(({permission}) =>
            permission.columns.includes(BOARD_REFERENCE_COLUMN)
        )
        .map(({role}) => role)
    assert.deepEqual(roles, ["service-account"])
})
