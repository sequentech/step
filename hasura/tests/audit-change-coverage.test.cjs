// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const {test} = require("node:test")
const assert = require("node:assert/strict")
const {readdirSync, readFileSync} = require("node:fs")
const {resolve} = require("node:path")
const yaml = require("js-yaml")

const TABLES_DIR = resolve(__dirname, "../metadata/databases/backend-db/tables")
const MIGRATIONS_DIR = resolve(__dirname, "../migrations/backend-db")
const LOGGED_ELSEWHERE_ROLE = "service-account"

// Tables a person can write through Hasura whose rows are not logged, each
// with its reason.
const EXEMPTIONS = {
    "sequent_backend.tasks_execution":
        "task bookkeeping rows the portal's widgets create, not configuration",
}

const qualifiedName = ({schema, name}) => `${schema}.${name}`

const trackedTables = () =>
    yaml
        .load(readFileSync(resolve(TABLES_DIR, "tables.yaml"), "utf8"))
        .map((include) => include.replace(/^!include\s+/, ""))
        .map((file) =>
            yaml.load(readFileSync(resolve(TABLES_DIR, file), "utf8"))
        )

const writableTables = (tables) =>
    tables
        .filter((metadata) =>
            ["insert", "update", "delete"].some((operation) =>
                (metadata[`${operation}_permissions`] ?? []).some(
                    ({role}) => role !== LOGGED_ELSEWHERE_ROLE
                )
            )
        )
        .map(({table}) => qualifiedName(table))

const migrationsInOrder = () =>
    readdirSync(MIGRATIONS_DIR)
        .sort((a, b) => parseInt(a, 10) - parseInt(b, 10))
        .map((directory) =>
            readFileSync(resolve(MIGRATIONS_DIR, directory, "up.sql"), "utf8")
        )

const TABLE = String.raw`"?(\w+)"?\."?(\w+)"?`
const TRIGGER_STATEMENT = new RegExp(
    [
        String.raw`CREATE\s+TRIGGER\s+"?audit_change"?\s+AFTER\s+INSERT\s+OR\s+UPDATE\s+OR\s+DELETE\s+ON\s+${TABLE}\s+FOR\s+EACH\s+ROW\s+EXECUTE\s+FUNCTION\s+"?sequent_backend"?\."?audit_change"?\(\)`,
        String.raw`DROP\s+TRIGGER\s+(?:IF\s+EXISTS\s+)?"?audit_change"?\s+ON\s+${TABLE}`,
        String.raw`DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?${TABLE}`,
    ].join("|"),
    "gi"
)

// The tables left with the trigger after the migrations ran in order.
const auditedTables = (migrations) => {
    const audited = new Set()
    for (const sql of migrations) {
        const statements = sql.replace(/--.*$/gm, "")
        for (const match of statements.matchAll(TRIGGER_STATEMENT)) {
            if (match[1]) {
                audited.add(`${match[1]}.${match[2]}`)
            } else {
                audited.delete(
                    `${match[3] ?? match[5]}.${match[4] ?? match[6]}`
                )
            }
        }
    }
    return audited
}

const coverage = ({writable, audited, exemptions}) => ({
    unlogged: writable.filter(
        (table) => !audited.has(table) && !(table in exemptions)
    ),
    staleExemptions: Object.keys(exemptions).filter(
        (table) => !writable.includes(table) || audited.has(table)
    ),
})

const current = () =>
    coverage({
        writable: writableTables(trackedTables()),
        audited: auditedTables(migrationsInOrder()),
        exemptions: EXEMPTIONS,
    })

test("every table a person can write through Hasura has the audit_change trigger", () => {
    assert.deepEqual(current().unlogged, [])
})

test("every exemption names a writable table without the trigger", () => {
    assert.deepEqual(current().staleExemptions, [])
})

test("every exemption gives its reason", () => {
    for (const [table, reason] of Object.entries(EXEMPTIONS)) {
        assert.ok(reason.trim().length > 0, `${table} has no reason`)
    }
})

test("the metadata has tables a person can write", () => {
    const writable = writableTables(trackedTables())
    assert.ok(writable.includes("sequent_backend.tenant"))
    assert.ok(writable.includes("sequent_backend.election_event"))
    assert.ok(!writable.includes("sequent_backend.results_event"))
})

const syntheticTable = (name, permissions) =>
    yaml.load(
        [
            "table:",
            `    name: ${name}`,
            "    schema: sequent_backend",
            ...permissions.flatMap(([operation, role]) => [
                `${operation}_permissions:`,
                `    - role: ${role}`,
                "      permission:",
                "          filter: {}",
            ]),
        ].join("\n")
    )

const trigger = (table) =>
    `CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON ${table}\n` +
    "    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();"

test("a new writable table without the trigger is reported", () => {
    const tables = [
        ...trackedTables(),
        syntheticTable("polling_place", [["update", "admin-user"]]),
    ]
    assert.deepEqual(
        coverage({
            writable: writableTables(tables),
            audited: auditedTables(migrationsInOrder()),
            exemptions: EXEMPTIONS,
        }).unlogged,
        ["sequent_backend.polling_place"]
    )
})

for (const operation of ["insert", "update", "delete"]) {
    test(`a table a role can only ${operation} needs the trigger`, () => {
        const writable = writableTables([
            syntheticTable("polling_place", [[operation, "area-write"]]),
        ])
        assert.deepEqual(
            coverage({writable, audited: new Set(), exemptions: {}}).unlogged,
            ["sequent_backend.polling_place"]
        )
        assert.deepEqual(
            coverage({
                writable,
                audited: auditedTables([
                    trigger("sequent_backend.polling_place"),
                ]),
                exemptions: {},
            }).unlogged,
            []
        )
    })
}

test("a table only the service account writes needs no trigger", () => {
    const writable = writableTables([
        syntheticTable("polling_place", [
            ["insert", "service-account"],
            ["select", "admin-user"],
        ]),
    ])
    assert.deepEqual(writable, [])
})

test("an exemption for a table nobody writes, or one that has the trigger, is stale", () => {
    const writable = writableTables([
        syntheticTable("polling_place", [["insert", "admin-user"]]),
    ])
    const exemptions = {
        "sequent_backend.polling_place": "logged by its route",
        "sequent_backend.results_event": "written by the tally",
    }
    assert.deepEqual(
        coverage({writable, audited: new Set(), exemptions}).staleExemptions,
        ["sequent_backend.results_event"]
    )
    assert.deepEqual(
        coverage({
            writable,
            audited: new Set(["sequent_backend.polling_place"]),
            exemptions,
        }).staleExemptions,
        ["sequent_backend.polling_place", "sequent_backend.results_event"]
    )
})

test("a dropped trigger, a dropped table and a commented statement leave no trigger", () => {
    const polling = "sequent_backend.polling_place"
    assert.deepEqual([...auditedTables([trigger(polling)])], [polling])
    assert.deepEqual(
        [
            ...auditedTables([
                trigger(polling),
                `DROP TRIGGER IF EXISTS "audit_change" ON "sequent_backend"."polling_place";`,
            ]),
        ],
        []
    )
    assert.deepEqual(
        [...auditedTables([trigger(polling), `DROP TABLE ${polling};`])],
        []
    )
    assert.deepEqual(
        [
            ...auditedTables([
                `-- CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON ${polling} FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();`,
            ]),
        ],
        []
    )
})
