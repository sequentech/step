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

const selectGrant = (metadata, role) =>
    (metadata.select_permissions ?? []).find((grant) => grant.role === role)

const SHELL_READS = [
    ["tenant", "admin-user"],
    ["election_event", "election-event-read"],
    ["election", "election-read"],
    ["contest", "election-event-read"],
    ["candidate", "election-event-read"],
]

const PROFILE_READS = {
    "trustee-ceremony": [
        ...SHELL_READS,
        ["trustee", "trustee-read"],
        ["document", "document-read"],
    ],
    "publish-read": SHELL_READS,
}

for (const [role, reads] of Object.entries(PROFILE_READS)) {
    for (const [table, sourceRole] of reads) {
        test(`${role} reads ${table} as ${sourceRole} does`, () => {
            const metadata = loadTable(table)
            const grant = selectGrant(metadata, role)
            const source = selectGrant(metadata, sourceRole)
            assert.ok(grant, `${role} has no select on ${table}`)
            assert.deepEqual(
                [...grant.permission.columns].sort(),
                [...source.permission.columns].sort()
            )
            assert.deepEqual(grant.permission.filter, source.permission.filter)
            assert.equal(
                grant.permission.allow_aggregations,
                source.permission.allow_aggregations
            )
        })

        test(`${role} cannot write ${table}`, () => {
            const metadata = loadTable(table)
            for (const operation of ["insert", "update", "delete"]) {
                const roles = (metadata[`${operation}_permissions`] ?? []).map(
                    (grant) => grant.role
                )
                assert.ok(!roles.includes(role), `${role} can ${operation}`)
            }
        })
    }
}

test("voters cannot read election event annotations", () => {
    const grant = selectGrant(loadTable("election_event"), "user")
    assert.ok(!grant.permission.columns.includes("annotations"))
})

test("publish-read reads the ballot generation tasks of its tenant only", () => {
    const grant = selectGrant(loadTable("tasks_execution"), "publish-read")
    assert.deepEqual(grant.permission.filter, {
        _and: [
            {tenant_id: {_eq: "X-Hasura-Tenant-Id"}},
            {
                type: {
                    _in: [
                        "EXPORT_BALLOT_PUBLICATION",
                        "GENERATE_BALLOT_PUBLICATION",
                    ],
                },
            },
        ],
    })
})

test("publish-results-read follows only its tenant's results publications", () => {
    const grant = selectGrant(
        loadTable("tasks_execution"),
        "publish-results-read"
    )
    assert.deepEqual(grant.permission.filter, {
        _and: [
            {tenant_id: {_eq: "X-Hasura-Tenant-Id"}},
            {type: {_eq: "PUBLISH_RESULTS_WEBSITE"}},
        ],
    })
})

test("fetchDocument accepts the role Harvest requires", () => {
    const {actions} = yaml.load(
        readFileSync(resolve(__dirname, "../metadata/actions.yaml"), "utf8")
    )
    const action = actions.find(({name}) => name === "fetchDocument")
    assert.deepEqual(action.permissions.map(({role}) => role).sort(), [
        "admin-user",
        "document-download",
    ])
})
