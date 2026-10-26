// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
// Explicit opt-in: use only a fresh disposable Hasura with an empty document table.
// This replaces that instance's metadata and inserts synthetic test rows.
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")
const origin = process.env.DOC_SCOPE_TEST_URL
assert.ok(
    origin && new URL(origin).hostname === "127.0.0.1",
    "Set DOC_SCOPE_TEST_URL to the disposable local Hasura"
)
const secret = process.env.DOC_SCOPE_TEST_ADMIN_SECRET
assert.ok(secret, "Set the disposable Hasura admin secret")
const tenant = "11111111-1111-4111-8111-111111111111"
const other = "22222222-2222-4222-8222-222222222222"
const event = "33333333-3333-4333-8333-333333333333"
const id = (suffix) =>
    `44444444-4444-4444-8444-${String(suffix).padStart(12, "0")}`
const metadata = yaml.load(
    readFileSync(
        resolve(
            __dirname,
            "../metadata/databases/backend-db/tables/sequent_backend_document.yaml"
        ),
        "utf8"
    )
)
async function request(path, body, role) {
    const response = await fetch(origin + path, {
        method: "POST",
        headers: {
            "content-type": "application/json",
            "x-hasura-admin-secret": secret,
            ...(role
                ? {"x-hasura-role": role, "x-hasura-tenant-id": tenant}
                : {}),
        },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(5000),
    })
    assert.ok(response.ok, await response.clone().text())
    return response.json()
}
async function query(query, variables, role = "admin-user") {
    return request("/v1/graphql", {query, variables}, role)
}
async function main() {
    await request("/v1/metadata", {
        type: "replace_metadata",
        args: {
            version: 3,
            sources: [
                {
                    name: "default",
                    kind: "postgres",
                    configuration: {
                        connection_info: {
                            database_url: {
                                from_env: "HASURA_GRAPHQL_DATABASE_URL",
                            },
                        },
                    },
                    tables: [metadata],
                },
            ],
        },
    })
    const insert = `mutation($objects:[sequent_backend_document_insert_input!]!){insert_sequent_backend_document(objects:$objects){affected_rows}}`
    const fixtures = [
        {
            id: id(1),
            tenant_id: tenant,
            election_event_id: null,
            name: `export-tenant-config-${tenant}.zip`,
            media_type: "application/zip",
            is_public: false,
        },
        {
            id: id(2),
            tenant_id: tenant,
            election_event_id: event,
            name: "election-results.pdf",
            is_public: false,
        },
        {
            id: id(3),
            tenant_id: other,
            election_event_id: null,
            name: "other-tenant.zip",
            is_public: false,
        },
        {
            id: id(4),
            tenant_id: tenant,
            election_event_id: null,
            name: "classified.zip",
            annotations: {access: {voter_secret_attributes: true}},
            is_public: false,
        },
    ]
    assert.equal(
        (await query(insert, {objects: fixtures}, "service-account")).data
            .insert_sequent_backend_document.affected_rows,
        4
    )
    const visible = await query(
        `{sequent_backend_document(order_by:{id:asc}){id}}`
    )
    assert.deepEqual(
        visible.data.sequent_backend_document.map((row) => row.id),
        [id(1), id(4)]
    )
    const update = `mutation($id:uuid!,$set:sequent_backend_document_set_input!){update_sequent_backend_document(where:{id:{_eq:$id}},_set:$set){affected_rows}}`
    assert.equal(
        (await query(update, {id: id(1), set: {labels: {reviewed: true}}})).data
            .update_sequent_backend_document.affected_rows,
        1
    )
    for (const blocked of [id(2), id(3), id(4)])
        assert.equal(
            (await query(update, {id: blocked, set: {name: "changed"}})).data
                .update_sequent_backend_document.affected_rows,
            0
        )
    for (const set of [{election_event_id: event}, {tenant_id: other}])
        assert.ok((await query(update, {id: id(1), set})).errors)
    for (const row of [
        {...fixtures[1], id: id(5)},
        {...fixtures[2], id: id(6)},
    ])
        assert.ok((await query(insert, {objects: [row]})).errors)
    assert.equal(
        (await query(insert, {objects: [{...fixtures[0], id: id(7)}]})).data
            .insert_sequent_backend_document.affected_rows,
        1
    )
    const remove = `mutation($id:uuid!){delete_sequent_backend_document(where:{id:{_eq:$id}}){affected_rows}}`
    for (const blocked of [id(2), id(3), id(4)])
        assert.equal(
            (await query(remove, {id: blocked})).data
                .delete_sequent_backend_document.affected_rows,
            0
        )
    assert.equal(
        (await query(remove, {id: id(7)})).data.delete_sequent_backend_document
            .affected_rows,
        1
    )
    const board = await query(
        `{sequent_backend_document(order_by:{id:asc}){id}}`,
        {},
        "document-read"
    )
    assert.deepEqual(
        board.data.sequent_backend_document.map((row) => row.id),
        [id(1), id(2), id(4)]
    )
    assert.equal(
        (
            await query(
                update,
                {id: id(2), set: {labels: {reviewed: true}}},
                "document-write"
            )
        ).data.update_sequent_backend_document.affected_rows,
        1
    )
    const retained = await query(
        `{sequent_backend_document_by_pk(id:"${id(1)}"){name election_event_id tenant_id}}`
    )
    assert.deepEqual(retained.data.sequent_backend_document_by_pk, {
        name: fixtures[0].name,
        election_event_id: null,
        tenant_id: tenant,
    })
    console.log(
        "PASS: actual Hasura row authorization preserves tenant backup scope, blocks event/cross-tenant/default-role writes and classified mutations, and retains board access"
    )
}
main().catch((error) => {
    console.error(error)
    process.exitCode = 1
})
