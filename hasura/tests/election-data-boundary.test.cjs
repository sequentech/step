// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")
const table = name => yaml.load(readFileSync(resolve(__dirname,
    `../metadata/databases/backend-db/tables/sequent_backend_${name}.yaml`), "utf8"))

// Execution-data deletion is a quorum-authorised server operation. A direct
// GraphQL mutation must not bypass either the export prerequisite or signing.
for (const name of ["election_event", "election", "contest", "candidate", "area", "area_contest", "ballot_publication", "ballot_style", "election_result", "keys_ceremony", "protocol_board", "tally_session", "tally_session_contest", "tally_session_execution", "tally_session_resolution", "tally_sheet", "tally_sheet_import", "tally_sheet_import_item", "electoral_log_checkpoint", "ballot_box_seal"]) {
    test(`no human GraphQL role directly deletes ${name}`, () => {
        assert.deepEqual((table(name).delete_permissions || []).filter(p => p.role !== "service-account"), [])
    })
}
test("archive state is changed only through the board's server action", () => {
    for (const operation of ["insert", "update"]) {
        for (const {role, permission} of table("election_event")[`${operation}_permissions`] || []) {
            if (role !== "service-account") assert.ok(!permission.columns.includes("is_archived"), `${role} ${operation}`)
        }
    }
})

for (const name of ["area", "area_contest", "ballot_publication", "ballot_style", "candidate", "contest", "document", "election", "election_event", "election_result", "support_material"]) {
    test(`voter reads of ${name} require a live safe execution phase`, () => {
        const permission = table(name).select_permissions.find(p => p.role === "user").permission
        const serialized = JSON.stringify(permission.filter)
        assert.ok(serialized.includes('"voter_allowed":{"_eq":true}'))
        assert.ok(serialized.includes('X-Hasura-Election-Event-Id'))
        const relationship = table(name).object_relationships.find(p => p.name === "lifecycle_access")
        assert.equal(relationship.using.manual_configuration.remote_table.name, "election_lifecycle_access")
    })
}

for (const name of ["election", "contest", "candidate", "area", "area_contest", "ballot_publication", "ballot_style", "scheduled_event"]) {
    test(`direct GraphQL writes of ${name} cannot bypass import authorisation`, () => {
        for (const kind of ["insert", "update"]) {
            assert.deepEqual((table(name)[`${kind}_permissions`] || []).filter(p => p.role !== "service-account"), [])
        }
    })
}
for (const name of ["area", "area_contest", "ballot_publication", "ballot_style", "candidate", "contest", "election_result", "keys_ceremony", "protocol_board", "tally_session", "tally_session_execution", "tally_session_resolution", "tally_sheet", "electoral_log_checkpoint", "ballot_box_seal", "tasks_execution", "event_execution"]) {
    test(`administrator role cannot read ${name}`, () => {
        assert.ok(!(table(name).select_permissions || []).some(p => p.role === "admin-user"))
    })
}
for (const name of ["election_event", "election"]) {
    test(`administrator ${name} navigation contains only the security-settings projection`, () => {
        const grant=table(name).select_permissions.find(p => p.role === "admin-user")
        assert.ok(grant.permission.columns.includes("administration_settings"))
        for (const column of ["presentation","status","statistics","public_key","annotations","eml","receipts"])
            assert.ok(!grant.permission.columns.includes(column))
    })
}
