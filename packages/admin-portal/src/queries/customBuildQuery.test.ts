// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "node:fs"
import {resolve} from "node:path"
import {type DocumentNode, type OperationDefinitionNode} from "graphql"
import {introspectSchema} from "ra-data-graphql/dist/cjs/introspection"
import {customBuildQuery} from "./customBuildQuery"

const EVENT = "20000000-0000-4000-8000-000000000001"
const ELECTION = "30000000-0000-4000-8000-000000000001"
const OTHER_ELECTION = "30000000-0000-4000-8000-000000000002"
const IMPORT = "90000000-0000-4000-8000-00000000000a"

type Params = Record<string, unknown>
interface Built {
    query: DocumentNode
    variables: Record<string, unknown>
    parseResponse: (response: {data: Record<string, unknown>}) => unknown
}
let build: (fetchType: string, resource: string, params: Params) => Built

beforeAll(async () => {
    // The introspection React-admin builds at startup, from the portal's schema.
    const json = JSON.parse(readFileSync(resolve(__dirname, "../../graphql.schema.json"), "utf8"))
    const named = ({name}: {name: string}) => name
    const options = {
        schema: json.__schema ?? json.data.__schema,
        operationNames: {
            GET_LIST: named,
            GET_ONE: named,
            GET_MANY: named,
            GET_MANY_REFERENCE: named,
            CREATE: ({name}: {name: string}) => `insert_${name}`,
            UPDATE: ({name}: {name: string}) => `update_${name}`,
            DELETE: ({name}: {name: string}) => `delete_${name}`,
        },
    }
    const introspection = await introspectSchema(
        undefined as unknown as Parameters<typeof introspectSchema>[0],
        options as unknown as Parameters<typeof introspectSchema>[1]
    )
    build = customBuildQuery(introspection)
})

// The builder logs each filter it drops.
beforeEach(() => jest.spyOn(console, "log").mockImplementation(() => undefined))
afterEach(() => jest.restoreAllMocks())

const list = (resource: string, params: Params = {}) =>
    build("GET_LIST", resource, {
        pagination: {page: 1, perPage: 10},
        sort: {field: "created_at", order: "DESC"},
        filter: {},
        ...params,
    })
const operationName = (query: DocumentNode) =>
    (query.definitions[0] as OperationDefinitionNode).name?.value

describe("resource list variables", () => {
    it("orders reports by the chosen column and then by id for stable pages", () => {
        const built = list("sequent_backend_report", {
            pagination: {page: 2, perPage: 10},
            filter: {election_event_id: EVENT},
        })
        expect(operationName(built.query)).toBe("sequent_backend_report")
        expect(built.variables).toEqual({
            where: {_and: [{election_event_id: {_eq: EVENT}}]},
            limit: 10,
            offset: 10,
            order_by: [{created_at: "desc"}, {id: "asc"}],
        })
    })

    it("drops filters and sorts on columns a whitelisted table does not have", () => {
        const built = list("sequent_backend_area", {
            sort: {field: "contests", order: "ASC"},
            filter: {election_event_id: EVENT, contests: "Mayor"},
        })
        expect(built.variables).toEqual({
            where: {_and: [{election_event_id: {_eq: EVENT}}]},
            limit: 10,
            offset: 0,
        })
    })

    it("keeps a sort on a whitelisted column", () => {
        const built = list("sequent_backend_area", {sort: {field: "name", order: "ASC"}})
        expect(built.variables.order_by).toEqual({name: "asc"})
    })

    it("matches scheduled events of the chosen elections and of the whole event", () => {
        const built = list("sequent_backend_scheduled_event", {
            sort: {field: "created_at", order: "ASC"},
            filter: {
                election_event_id: EVENT,
                event_payload: {
                    format: "hasura-raw-query",
                    value: {_contains: {election_id: [ELECTION, OTHER_ELECTION]}},
                },
            },
        })
        expect(built.variables).toEqual({
            where: {
                _and: [
                    {election_event_id: {_eq: EVENT}},
                    {
                        _or: [
                            {event_payload: {_contains: {election_id: ELECTION}}},
                            {event_payload: {_contains: {election_id: OTHER_ELECTION}}},
                            {event_payload: {_contains: {election_id: null}}},
                        ],
                    },
                ],
            },
            limit: 10,
            offset: 0,
            order_by: [{created_at: "asc"}, {id: "asc"}],
        })
    })

    it("lists only event-wide publications when no election is chosen", () => {
        const built = list("sequent_backend_ballot_publication", {
            filter: {election_event_id: EVENT},
        })
        expect(built.variables.where).toEqual({
            _and: [{election_event_id: {_eq: EVENT}}, {election_id: {_is_null: true}}],
        })
    })

    it("lists the publications that include the chosen election", () => {
        const built = list("sequent_backend_ballot_publication", {
            filter: {election_event_id: EVENT, election_id: ELECTION},
        })
        expect(built.variables.where).toEqual({
            _and: [{election_event_id: {_eq: EVENT}}, {election_ids: {_contains: [ELECTION]}}],
        })
    })

    it("searches tally sheet labels and annotations as text", () => {
        const built = list("sequent_backend_tally_sheet", {
            filter: {labels: "recount", annotations: "box 7"},
        })
        expect(built.variables.where).toEqual({
            _and: [
                {labels: {_cast: {String: {_ilike: "%recount%"}}}},
                {annotations: {_cast: {String: {_ilike: "%box 7%"}}}},
            ],
        })
    })

    it("does not send a partially typed import id", () => {
        const built = list("sequent_backend_tally_sheet", {filter: {import_id: IMPORT.slice(0, 8)}})
        expect(built.variables.where).toEqual({_and: []})
    })

    // The import_id column is missing from the tally sheet column whitelist, which
    // strips the filter before the uuid check can forward it.
    it.failing("filters tally sheets by a complete import id", () => {
        const built = list("sequent_backend_tally_sheet", {filter: {import_id: IMPORT}})
        expect(built.variables.where).toEqual({_and: [{import_id: {_eq: IMPORT}}]})
    })

    it("shows one row per ballot box, latest version first", () => {
        const built = list("sequent_backend_tally_sheet", {
            filter: {election_event_id: EVENT},
            meta: {distinctBallotBoxes: true},
        })
        expect(built.variables).toEqual({
            where: {_and: [{election_event_id: {_eq: EVENT}}]},
            limit: 10,
            offset: 0,
            distinct_on: ["area_id", "contest_id", "channel"],
            order_by: [{area_id: "asc"}, {contest_id: "asc"}, {channel: "asc"}, {version: "desc"}],
        })
    })
})

describe("task and application lists", () => {
    it("orders tasks by a task column", () => {
        const built = list("sequent_backend_tasks_execution", {
            sort: {field: "start_at", order: "ASC"},
            filter: {election_event_id: EVENT},
        })
        expect(built.variables).toEqual({
            where: {_and: [{election_event_id: {_eq: EVENT}}]},
            limit: 10,
            offset: 0,
            order_by: {start_at: "asc"},
        })
    })

    it("matches applicant data attributes, nested ones by dotted key, by containment", () => {
        const built = list("sequent_backend_applications", {
            sort: {field: "status", order: "ASC"},
            filter: {
                election_event_id: EVENT,
                applicant_data: {first_name: {_ilike: "Ana"}, address: {city: {_ilike: "Rome"}}},
            },
        })
        expect(built.variables).toEqual({
            where: {
                _and: [
                    {election_event_id: {_eq: EVENT}},
                    {},
                    {applicant_data: {_contains: {first_name: "Ana"}}},
                    {applicant_data: {_contains: {"address.city": "Rome"}}},
                ],
            },
            limit: 10,
            offset: 0,
            order_by: {status: "asc"},
        })
    })

    it("does not order applications by their applicant data", () => {
        const built = list("sequent_backend_applications", {
            sort: {field: "applicant_data", order: "ASC"},
        })
        expect(built.variables.order_by).toEqual({})
    })
})

describe("action-backed lists", () => {
    const page = {items: [{id: "a"}, {id: "b"}], total: {aggregate: {count: 7}}}

    it.each([
        ["user", "getUsers", "get_users"],
        ["role", "getRoles", "get_roles"],
        ["permission", "getPermissions", "get_permissions"],
        ["ip_address", "GetCastVotesByIp", "get_top_votes_by_ip"],
        ["electoral_log", "listElectoralLog", "listElectoralLog"],
        ["pgaudit", "listPgaudit", "listPgaudit"],
    ])("%s reads its page from the %s action", (resource, operation, field) => {
        const built = list(resource, {filter: {election_event_id: EVENT, tenant_id: EVENT}})
        expect(operationName(built.query)).toBe(operation)
        expect(built.parseResponse({data: {[field]: page}})).toEqual({
            data: page.items,
            total: 7,
        })
    })

    it("keeps only the electoral log's searchable filters", () => {
        const built = list("electoral_log", {
            filter: {election_event_id: EVENT, username: "maria", statement_head: "x"},
        })
        const sent = JSON.stringify(built.variables)
        expect(sent).toContain("maria")
        expect(sent).not.toContain("statement_head")
    })
})

describe("elections by external id", () => {
    it("asks for the external ids of one event and keys the rows by them", () => {
        const built = build("GET_MANY", "sequent_backend_election_by_external_id", {
            ids: [101, "102"],
            meta: {filter: {election_event_id: EVENT}},
        })
        expect(operationName(built.query)).toBe("GetElectionsByExternalId")
        expect(built.variables).toEqual({external_ids: ["101", "102"], election_event_id: EVENT})
        const rows = [{id: ELECTION, external_id: "101", alias: "mayor"}]
        expect(built.parseResponse({data: {sequent_backend_election: rows}})).toEqual({
            data: [{id: "101", external_id: "101", alias: "mayor"}],
        })
    })

    it("accepts the camel-case event filter and an empty id list", () => {
        const built = build("GET_MANY", "sequent_backend_election_by_external_id", {
            meta: {filter: {electionEventId: EVENT}},
        })
        expect(built.variables).toEqual({external_ids: [], election_event_id: EVENT})
        expect(built.parseResponse({data: {}})).toEqual({data: []})
    })
})
