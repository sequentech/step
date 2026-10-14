// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "node:fs"
import {resolve} from "node:path"
import {print, type DocumentNode, type OperationDefinitionNode} from "graphql"
import {introspectSchema} from "ra-data-graphql/dist/cjs/introspection"
import {customBuildQuery} from "./customBuildQuery"
import {electionSearchFilter} from "../services/ElectionSearch"
import {ACTIVE_SCHEDULED_EVENTS} from "../services/ScheduledEvent"
import {Order_By} from "@/gql/graphql"
import {ELogScope} from "@/components/logs/logScope"

// The log range filter reads its wall times with the timezone service.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/services/timeZones"),
}))

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

    it("keeps an operator filter on a whitelisted column", () => {
        const built = list("sequent_backend_area", {
            sort: undefined,
            filter: {"name@_ilike": "Nor", "election_event_id": EVENT, "contests@_ilike": "Mayor"},
        })
        expect(built.variables.where).toEqual({
            _and: [{name: {_ilike: "%Nor%"}}, {election_event_id: {_eq: EVENT}}],
        })
    })

    it.each(["sequent_backend_election", "sequent_backend_contest"])(
        "keeps the name search's boolean expression on %s",
        (resource) => {
            const built = list(resource, {
                sort: undefined,
                filter: {election_event_id: EVENT, ...electionSearchFilter("Mayor")},
            })
            expect(built.variables.where).toEqual({
                _and: [
                    {election_event_id: {_eq: EVENT}},
                    {
                        _or: [
                            {external_id: {_ilike: "%Mayor%"}},
                            {presentation: {_cast: {String: {_ilike: "%Mayor%"}}}},
                        ],
                    },
                ],
            })
        }
    )

    it("drops a boolean expression that is not a raw Hasura query", () => {
        const built = list("sequent_backend_election", {
            sort: undefined,
            filter: {election_event_id: EVENT, _or: "Mayor"},
        })
        expect(built.variables.where).toEqual({_and: [{election_event_id: {_eq: EVENT}}]})
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

    it("filters tally sheets by a complete import id", () => {
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

describe("latest tally session execution per session", () => {
    const executions = (meta?: Params, params: Params = {}) =>
        list("sequent_backend_tally_session_execution", {
            filter: {
                tally_session_id: {format: "hasura-raw-query", value: {_in: ["session-1"]}},
                tenant_id: "tenant-1",
            },
            meta,
            ...params,
        })
    const orderBy = (built: Built) => built.variables.order_by as Array<Record<string, string>>

    it("orders by tally_session_id first so distinct_on is valid in Postgres", () => {
        const built = executions({latestPerTallySession: true})
        expect(built.variables.distinct_on).toEqual(["tally_session_id"])
        expect(orderBy(built)[0]).toEqual({tally_session_id: "asc"})
    })

    it("orders nullable created_at with nulls last so a null-dated execution never wins", () => {
        expect(executions({latestPerTallySession: true}).variables).toEqual({
            where: {
                _and: [{tally_session_id: {_in: ["session-1"]}}, {tenant_id: {_eq: "tenant-1"}}],
            },
            limit: 10,
            offset: 0,
            distinct_on: ["tally_session_id"],
            order_by: [{tally_session_id: "asc"}, {created_at: "desc_nulls_last"}, {id: "desc"}],
        })
    })

    it("uses an ordering direction the Hasura schema actually accepts", () => {
        const directions = Object.values(Order_By) as string[]
        orderBy(executions({latestPerTallySession: true})).forEach((entry) =>
            Object.values(entry).forEach((direction) => expect(directions).toContain(direction))
        )
    })

    it("does not order created_at with a nulls-first direction", () => {
        const createdAtOrder = orderBy(executions({latestPerTallySession: true})).find(
            (entry) => "created_at" in entry
        )?.created_at
        expect(["desc", "desc_nulls_first"]).not.toContain(createdAtOrder)
    })

    it("leaves the query untouched when the latestPerTallySession flag is absent", () => {
        const built = executions()
        expect(built.variables).not.toHaveProperty("distinct_on")
        expect(built.variables.order_by).toEqual({created_at: "desc"})
    })

    it("adds no order to an unsorted list", () => {
        const built = executions({latestPerTallySession: true}, {sort: undefined})
        expect(built.variables.distinct_on).toEqual(["tally_session_id"])
        expect(built.variables).not.toHaveProperty("order_by")
    })
})

describe("scheduled events", () => {
    it("lists only active rows: a sent notification, stored archived, never shows (Follow-up 2)", () => {
        const built = list("sequent_backend_scheduled_event", {
            sort: {field: "created_at", order: "ASC"},
            filter: {election_event_id: EVENT, ...ACTIVE_SCHEDULED_EVENTS},
        })
        expect((built.variables.where as {_and: unknown[]})._and).toContainEqual({
            archived_at: {_is_null: true},
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

    it("searches the applicant data of applications as text", () => {
        const built = list("sequent_backend_applications", {
            sort: {field: "created_at", order: "ASC"},
            filter: {election_event_id: EVENT, q: " dela cruz "},
        })
        expect(built.variables.where).toEqual({
            _and: [
                {election_event_id: {_eq: EVENT}},
                {applicant_data: {_cast: {String: {_ilike: "%dela cruz%"}}}},
            ],
        })
    })

    it("finds an application by the support reference the voter was given (EF-1)", () => {
        const built = list("sequent_backend_applications", {
            sort: {field: "created_at", order: "ASC"},
            filter: {
                election_event_id: EVENT,
                q: "65sr mssd",
                support_reference: {reference: "65SR-MSSD", ids: ["app-1"]},
            },
        })
        expect(built.variables.where).toEqual({
            _and: [
                {election_event_id: {_eq: EVENT}},
                {
                    _or: [
                        {applicant_data: {_cast: {String: {_ilike: "%65sr mssd%"}}}},
                        {id: {_in: ["app-1"]}},
                    ],
                },
            ],
        })
    })

    it("ignores a support reference resolved for another search text", () => {
        const built = list("sequent_backend_applications", {
            sort: {field: "created_at", order: "ASC"},
            filter: {
                election_event_id: EVENT,
                q: "Q5KW-8RFX",
                support_reference: {reference: "65SR-MSSD", ids: ["app-1"]},
            },
        })
        expect(built.variables.where).toEqual({
            _and: [
                {election_event_id: {_eq: EVENT}},
                {applicant_data: {_cast: {String: {_ilike: "%Q5KW-8RFX%"}}}},
            ],
        })
    })

    it("does not search applications for a blank text", () => {
        const built = list("sequent_backend_applications", {
            sort: {field: "created_at", order: "ASC"},
            filter: {election_event_id: EVENT, q: "  "},
        })
        expect(built.variables.where).toEqual({_and: [{election_event_id: {_eq: EVENT}}]})
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

    it("sends the Created range as instants of the wall times in the chosen zone", () => {
        const built = list("electoral_log", {
            filter: {
                election_event_id: EVENT,
                created_from: "2028-04-08T00:00",
                created_to: "2028-04-09T23:59",
                time_zone: "Asia/Manila",
            },
        })
        const filter = built.variables.filter as Record<string, string>
        expect(new Date(filter.created_from).getTime()).toBe(Date.parse("2028-04-07T16:00:00Z"))
        expect(new Date(filter.created_to).getTime()).toBe(Date.parse("2028-04-09T15:59:00Z"))
        expect(JSON.stringify(built.variables)).not.toContain("time_zone")
    })

    it("filters either log by event type and log type", () => {
        const built = list("electoral_log", {
            filter: {election_event_id: EVENT, event_type: "SYSTEM", log_type: "ERROR"},
        })
        expect(built.variables.filter).toEqual({event_type: "SYSTEM", log_type: "ERROR"})
    })

    it("reads an event's log as the default role, naming the event", () => {
        const built = list("electoral_log", {filter: {election_event_id: EVENT}}) as Built & {
            context?: unknown
        }
        expect(built.context).toBeUndefined()
        expect(print(built.query)).toContain(`$election_event_id: String = "${EVENT}"`)
    })

    it("reads the tenant log without an election event, as tenant-logs-read", () => {
        const built = list("electoral_log", {
            filter: {election_event_id: EVENT, username: "rosa.marin", event_type: "USER"},
            meta: {logScope: ELogScope.TENANT},
        }) as Built & {context?: unknown}
        expect(operationName(built.query)).toBe("listTenantLog")
        expect(print(built.query)).not.toContain("election_event_id")
        expect(JSON.stringify(built.variables)).not.toContain(EVENT)
        expect(built.variables.filter).toEqual({username: "rosa.marin", event_type: "USER"})
        expect(built.context).toEqual({headers: {"x-hasura-role": "tenant-logs-read"}})
        expect(built.parseResponse({data: {listElectoralLog: page}})).toEqual({
            data: page.items,
            total: 7,
        })
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
