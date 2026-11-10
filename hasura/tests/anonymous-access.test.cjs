// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const METADATA = resolve(__dirname, "../metadata")
const DATABASE = resolve(METADATA, "databases/backend-db")
const ANONYMOUS = "unauthorized"

const load = (path) => yaml.load(readFileSync(path, "utf8"))
const included = (dir, list) =>
    load(resolve(dir, list)).map((entry) =>
        load(resolve(dir, entry.replace("!include ", "")))
    )

const tables = included(resolve(DATABASE, "tables"), "tables.yaml")
const functions = included(resolve(DATABASE, "functions"), "functions.yaml")

test("the anonymous role reads only the sign-in projection of one tenant", () => {
    const grants = tables.flatMap(({table, ...metadata}) =>
        ["insert", "select", "update", "delete"]
            .filter((operation) =>
                (metadata[`${operation}_permissions`] ?? []).some(
                    ({role}) => role === ANONYMOUS
                )
            )
            .map((operation) => `${table.name} ${operation}`)
    )
    assert.deepEqual(grants, ["sign_in_tenant select"])
    const {permission} = tables
        .find(({table}) => table.name === "sign_in_tenant")
        .select_permissions.find(({role}) => role === ANONYMOUS)
    assert.deepEqual([...permission.columns].sort(), ["annotations", "id"])
    assert.equal(permission.limit, 1)
    assert.deepEqual(permission.query_root_fields, [])
    assert.deepEqual(permission.subscription_root_fields, [])
})

test("the tenant lookup function is the anonymous role's only root field", () => {
    assert.match(
        readFileSync(resolve(METADATA, "databases/databases.yaml"), "utf8"),
        /functions: "!include backend-db\/functions\/functions.yaml"/
    )
    assert.deepEqual(
        functions.map(({function: fn, permissions}) => [
            fn.name,
            permissions.map(({role}) => role),
        ]),
        [["tenant_for_sign_in", [ANONYMOUS]]]
    )
    const actions = load(resolve(METADATA, "actions.yaml")).actions
    assert.deepEqual(
        actions.filter(({permissions}) =>
            (permissions ?? []).some(({role}) => role === ANONYMOUS)
        ),
        []
    )
})

test("actions send no admin secret header", () => {
    const actions = load(resolve(METADATA, "actions.yaml")).actions
    const secretHeaders = actions.filter(({definition}) =>
        (definition.headers ?? []).some(
            ({name}) => name.toLowerCase() === "x-hasura-admin-secret"
        )
    )
    assert.deepEqual(
        secretHeaders.map(({name}) => name),
        []
    )
})
