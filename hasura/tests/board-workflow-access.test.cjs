// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")
const load = (path) => yaml.load(readFileSync(resolve(__dirname, "../metadata", path), "utf8"))
const table = (name) => load(`databases/backend-db/tables/sequent_backend_${name}.yaml`)
const select = (name, role) => table(name).select_permissions.find(p => p.role === role)?.permission
const tenant = {tenant_id: {_eq: "X-Hasura-Tenant-Id"}}

test("board document and ceremony actions use their existing backend permissions", () => {
    const actions = load("actions.yaml").actions
    for (const [name, roles] of [["fetchDocument", ["admin-user", "document-download"]], ["update_tally_ceremony", ["trustee-read", "admin-user", "admin-ceremony"]]]) {
        const action = actions.find(a => a.name === name)
        assert.deepEqual(action.permissions.map(p => p.role).sort(), roles.sort())
        assert.equal(action.definition.forward_client_headers, true)
    }
})

test("the actual GetTallyData selection is readable with tally-results-read", () => {
    const source = readFileSync(resolve(__dirname, "../../packages/admin-portal/src/queries/GetTallyData.ts"), "utf8")
    const fields = [...source.matchAll(/sequent_backend_(\w+)\(([\s\S]*?)\)\s*\{([^{}]*)\}/g)]
    assert.equal(fields.length, 13)
    for (const [, name, where, selection] of fields) {
        const permission = select(name, "tally-results-read")
        assert.ok(permission, name)
        const columns = new Set(selection.trim().split(/\s+/))
        for (const [, column] of where.matchAll(/(\w+)\s*:\s*\{_/g)) columns.add(column)
        for (const column of columns) assert.ok(permission.columns.includes(column), `${name}.${column}`)
        assert.ok(JSON.stringify(permission.filter).includes("X-Hasura-Tenant-Id"))
        for (const kind of ["insert_permissions", "update_permissions", "delete_permissions"]) assert.ok(!(table(name)[kind] ?? []).some(p => p.role === "tally-results-read"))
    }
})

test("new result projections retain their existing tenant and election-label filters", () => {
    for (const [name, base] of [["election_event", "election-event-read"], ["area", "area-read"], ["area_contest", "area-read"], ["election", "election-read"], ["candidate", "candidate-read"], ["contest", "contest-read"]]) assert.deepEqual(select(name, "tally-results-read")?.filter, select(name, base).filter, name)
})

test("board trustee choices expose only public identity columns in the tenant", () => {
    const permission = select("trustee", "admin-ceremony")
    assert.ok(permission)
    assert.deepEqual(permission.columns.slice().sort(), ["id", "name", "tenant_id"])
    assert.deepEqual(permission.filter, tenant)
    for (const kind of ["insert_permissions", "update_permissions", "delete_permissions"]) assert.ok(!(table("trustee")[kind] ?? []).some(p => p.role === "admin-ceremony"))
})

test("independent signed public anchors are board-only and tenant scoped", () => {
    const permission = select("protocol_board", "admin-ceremony")
    assert.ok(permission.columns.includes("manager_message"))
    assert.deepEqual(permission.filter, tenant)
    assert.ok(!select("protocol_board", "admin-user")?.columns.includes("manager_message"))
})

test("transcript export accepts only a selected event and board under admin-ceremony", () => {
    const action = load("actions.yaml").actions.find(a => a.name === "export_verification_transcript")
    assert.deepEqual(action.permissions, [{role: "admin-ceremony"}])
    assert.equal(action.definition.handler, "http://{{HARVEST_DOMAIN}}/export-verification-transcript")
    assert.deepEqual(action.definition.headers, [{name: "X-Hasura-Admin-Secret", value_from_env: "ACTIONS_ADMIN_SECRET"}])
    assert.ok(load("actions.yaml").custom_types.objects.some(t => t.name === "ExportVerificationTranscriptOutput"))
    assert.equal(action.definition.forward_client_headers, true)
    const template = action.definition.request_transform.body.template
    assert.match(template, /election_event_id/)
    assert.match(template, /protocol_board_id/)
    assert.doesNotMatch(template, /board_url|tenant_id|document_id/)
})


test("publishers can read event support material without gaining editing or administration grants", () => {
    const permission = select("support_material", "publish-read")
    assert.ok(permission, "publication dependency is readable")
    assert.deepEqual(permission.filter, select("election_event", "election-event-read").filter)
    assert.deepEqual(permission.columns, select("support_material", "support-material-read").columns)
    assert.equal(permission.allow_aggregations, true)
    assert.equal(select("support_material", "admin-user"), undefined)
    for (const kind of ["insert_permissions", "update_permissions", "delete_permissions"])
        assert.ok(!(table("support_material")[kind] ?? []).some(p => p.role === "publish-read"))
})
