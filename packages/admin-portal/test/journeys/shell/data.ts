// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {TENANT_ID} from "../fixtures"

export type Row = Record<string, unknown>

export const SHELL_IDS = {
    event: IDS.event,
    election: IDS.election,
    contest: IDS.contest,
    area: IDS.area,
    ballotStyle: IDS.style,
    candidate: IDS.alice,
    secondCandidate: IDS.bob,
    user: IDS.voter,
    areaContest: "90000000-0000-4000-8000-000000000001",
    document: "90000000-0000-4000-8000-000000000002",
    template: "90000000-0000-4000-8000-000000000003",
    notification: "90000000-0000-4000-8000-000000000004",
    scheduledEvent: "90000000-0000-4000-8000-000000000005",
    report: "90000000-0000-4000-8000-000000000006",
    electionType: "90000000-0000-4000-8000-000000000007",
    trustee: "90000000-0000-4000-8000-000000000008",
    publication: "90000000-0000-4000-8000-000000000009",
} as const

const english = (name: string, extra: Row = {}) => ({i18n: {en: {name, ...extra}}})
const stamps = {created_at: FIXED_TIME, labels: {}, annotations: {}}
const inEvent = {
    tenant_id: TENANT_ID,
    election_event_id: SHELL_IDS.event,
    last_updated_at: FIXED_TIME,
    ...stamps,
}

export const electionEvent = (row: Row = {}): Row => ({
    id: SHELL_IDS.event,
    tenant_id: TENANT_ID,
    ...stamps,
    updated_at: FIXED_TIME,
    description: "Annual council election",
    encryption_protocol: "RSA256",
    is_archived: false,
    is_audit: false,
    presentation: {
        ...english("Council 2026"),
        language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
    },
    status: {voting_status: "NOT_STARTED"},
    voting_channels: {online: true, kiosk: false},
    ...row,
})

export const election = (row: Row = {}): Row => ({
    id: SHELL_IDS.election,
    ...inEvent,
    description: "Choose the next mayor",
    presentation: english("Mayor"),
    status: {voting_status: "NOT_STARTED"},
    num_allowed_revotes: 1,
    is_consolidated_ballot_encoding: false,
    spoil_ballot_option: true,
    receipts: {},
    permission_label: null,
    ...row,
})

export const contest = (row: Row = {}): Row => ({
    id: SHELL_IDS.contest,
    ...inEvent,
    election_id: SHELL_IDS.election,
    description: "Pick one candidate",
    presentation: english("Mayor contest"),
    min_votes: 0,
    max_votes: 1,
    winning_candidates_num: 1,
    voting_type: "non-preferential",
    counting_algorithm: "plurality-at-large",
    is_encrypted: true,
    is_acclaimed: false,
    ...row,
})

export const candidate = (row: Row = {}): Row => ({
    id: SHELL_IDS.candidate,
    ...inEvent,
    contest_id: SHELL_IDS.contest,
    description: "Independent",
    type: "candidate",
    presentation: english("Alice Aldana"),
    is_public: true,
    ...row,
})

export const area = (row: Row = {}): Row => ({
    id: SHELL_IDS.area,
    ...inEvent,
    name: "North district",
    description: "Northern wards",
    type: null,
    parent_id: null,
    presentation: null,
    ...row,
})

export const areaContest = (row: Row = {}): Row => ({
    id: SHELL_IDS.areaContest,
    ...inEvent,
    area_id: SHELL_IDS.area,
    contest_id: SHELL_IDS.contest,
    ...row,
})

export const ballotStyle = (row: Row = {}): Row => ({
    id: SHELL_IDS.ballotStyle,
    ...inEvent,
    election_id: SHELL_IDS.election,
    area_id: SHELL_IDS.area,
    ballot_publication_id: SHELL_IDS.publication,
    ballot_eml: null,
    status: "PUBLISHED",
    deleted_at: null,
    ...row,
})

export const document = (row: Row = {}): Row => ({
    id: SHELL_IDS.document,
    ...inEvent,
    name: "results.pdf",
    media_type: "application/pdf",
    size: 2048,
    is_public: false,
    ...row,
})

export const template = (row: Row = {}): Row => ({
    id: SHELL_IDS.template,
    tenant_id: TENANT_ID,
    ...stamps,
    updated_at: FIXED_TIME,
    alias: "ballot-receipt",
    type: "BALLOT_RECEIPT",
    communication_method: "EMAIL",
    created_by: "synthetic-admin",
    template: {
        alias: "ballot-receipt",
        name: "Ballot receipt",
        type: "BALLOT_RECEIPT",
        email: {subject: "Your receipt", plaintext_body: "Receipt", html_body: "<p>Receipt</p>"},
        sms: {message: "Receipt"},
        document: "<p>Receipt</p>",
        selected_methods: {email: true, sms: false, document: false},
    },
    ...row,
})

export const notification = (row: Row = {}): Row => ({
    id: SHELL_IDS.notification,
    ...inEvent,
    updated_at: FIXED_TIME,
    type: "ELECTION_OPENED",
    ...row,
})

export const scheduledEvent = (row: Row = {}): Row => ({
    id: SHELL_IDS.scheduledEvent,
    tenant_id: TENANT_ID,
    election_event_id: SHELL_IDS.event,
    created_at: FIXED_TIME,
    stopped_at: null,
    archived_at: null,
    labels: {},
    annotations: {},
    event_processor: "START_VOTING_PERIOD",
    cron_config: {cron: null, scheduled_date: "2026-02-01T09:00:00.000Z"},
    event_payload: {election_id: SHELL_IDS.election},
    task_id: null,
    ...row,
})

export const report = (row: Row = {}): Row => ({
    id: SHELL_IDS.report,
    tenant_id: TENANT_ID,
    election_event_id: SHELL_IDS.event,
    election_id: SHELL_IDS.election,
    created_at: FIXED_TIME,
    report_type: "BALLOT_RECEIPT",
    template_alias: "ballot-receipt",
    encryption_policy: "unencrypted",
    cron_config: null,
    permission_label: null,
    ...row,
})

export const electionType = (row: Row = {}): Row => ({
    id: SHELL_IDS.electionType,
    tenant_id: TENANT_ID,
    name: "Municipal",
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
    labels: {},
    annotations: {},
    ...row,
})

/** The tables a fully configured tenant returns for its React-admin reads. */
export const adminTables = (): Record<string, Row[]> => ({
    sequent_backend_election_event: [electionEvent()],
    sequent_backend_election: [election()],
    sequent_backend_contest: [contest()],
    sequent_backend_candidate: [
        candidate(),
        candidate({
            id: SHELL_IDS.secondCandidate,
            presentation: english("Bruno Blanco"),
        }),
    ],
    sequent_backend_area: [area()],
    sequent_backend_area_contest: [areaContest()],
    sequent_backend_ballot_style: [ballotStyle()],
    sequent_backend_document: [document()],
    sequent_backend_template: [template()],
    sequent_backend_notification: [notification()],
    sequent_backend_scheduled_event: [scheduledEvent()],
    sequent_backend_report: [report()],
    sequent_backend_election_type: [electionType()],
})

type Where = Record<string, unknown>
const compare = (value: unknown, operand: unknown) =>
    String(value).localeCompare(String(operand), "en", {numeric: true})
const like = (value: unknown, pattern: unknown, flags: string) =>
    typeof value === "string" &&
    new RegExp(
        `^${String(pattern)
            .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
            .replace(/%/g, ".*")
            .replace(/_/g, ".")}$`,
        flags
    ).test(value)
const contains = (value: unknown, operand: unknown): boolean => {
    if (Array.isArray(operand))
        return (
            Array.isArray(value) &&
            operand.every((item) => value.some((candidate) => contains(candidate, item)))
        )
    if (operand && typeof operand === "object")
        return (
            !!value &&
            typeof value === "object" &&
            Object.entries(operand).every(([key, item]) => contains((value as Row)[key], item))
        )
    return value === operand
}
const OPERATORS: Record<string, (value: unknown, operand: unknown) => boolean> = {
    _eq: (value, operand) => value === operand,
    _neq: (value, operand) => value !== operand,
    _in: (value, operand) => (operand as unknown[]).includes(value),
    _nin: (value, operand) => !(operand as unknown[]).includes(value),
    _is_null: (value, operand) => (value == null) === operand,
    _gt: (value, operand) => value != null && compare(value, operand) > 0,
    _gte: (value, operand) => value != null && compare(value, operand) >= 0,
    _lt: (value, operand) => value != null && compare(value, operand) < 0,
    _lte: (value, operand) => value != null && compare(value, operand) <= 0,
    _like: (value, operand) => like(value, operand, ""),
    _ilike: (value, operand) => like(value, operand, "i"),
    _contains: contains,
}

/**
 * Hasura's boolean expression semantics for the operators the portal sends.
 * An operator outside this set is reported, so a filter never silently matches.
 */
export function matches(row: Row, where: Where, unsupported: (operator: string) => void): boolean {
    return Object.entries(where).every(([key, condition]) => {
        if (key === "_and")
            return (Array.isArray(condition) ? condition : [condition]).every((part: Where) =>
                matches(row, part, unsupported)
            )
        if (key === "_or")
            return (condition as Where[]).some((part) => matches(row, part, unsupported))
        if (key === "_not") return !matches(row, condition as Where, unsupported)
        const value = row[key]
        const entries = Object.entries(condition as Where)
        if (entries.length && entries.every(([name]) => name.startsWith("_")))
            return entries.every(([operator, operand]) => {
                const check = OPERATORS[operator]
                if (!check) unsupported(operator)
                return check ? check(value, operand) : true
            })
        if (Array.isArray(value))
            return value.some((item: Row) => matches(item, condition as Where, unsupported))
        return !!value && matches(value as Row, condition as Where, unsupported)
    })
}

/**
 * Answers the React-admin reads of each table (list, one, many and references,
 * all named after the table) from in-memory rows, applying the request's
 * filter and page.
 */
export function serveTables(portal: PortalServices, tables: Record<string, Row[]>) {
    for (const [table, rows] of Object.entries(tables)) {
        portal.graphql.on(table, ({variables}) => {
            const unsupported = (operator: string) =>
                portal.violations.add(`Unsupported Hasura operator ${operator} on ${table}`)
            const where = (variables.where ?? {}) as Where
            const found = rows.filter((row) => matches(row, where, unsupported))
            const offset = Number(variables.offset ?? 0)
            const limit = variables.limit == null ? found.length : Number(variables.limit)
            return {
                data: {
                    [table]: found.slice(offset, offset + limit),
                    [`${table}_aggregate`]: {aggregate: {count: found.length}},
                    [`${table}_by_pk`]: rows.find((row) => row.id === variables.id) ?? null,
                },
            }
        })
    }
}

/** The sidebar's event, election, contest and candidate tree over the same rows. */
export function serveTree(portal: PortalServices, tables: Record<string, Row[]>) {
    const select = (table: string, where: Row) =>
        (tables[table] ?? []).filter((row) =>
            Object.entries(where).every(([key, value]) => row[key] === value)
        )
    portal.graphql.on("election_events_tree", ({variables}) => ({
        data: {
            sequent_backend_election_event: select("sequent_backend_election_event", {
                tenant_id: variables.tenantId,
                is_archived: variables.isArchived,
            }),
        },
    }))
    portal.graphql.on("election_tree", ({variables}) => ({
        data: {
            sequent_backend_election: select("sequent_backend_election", {
                tenant_id: variables.tenantId,
                election_event_id: variables.electionEventId,
            }),
        },
    }))
    portal.graphql.on("contest_tree", ({variables}) => ({
        data: {
            sequent_backend_contest: select("sequent_backend_contest", {
                tenant_id: variables.tenantId,
                election_id: variables.electionId,
            }),
        },
    }))
    portal.graphql.on("candidate_tree", ({variables}) => ({
        data: {
            sequent_backend_candidate: select("sequent_backend_candidate", {
                tenant_id: variables.tenantId,
                contest_id: variables.contestId,
            }),
        },
    }))
}

export const keycloakUser = (row: Row = {}): Row => ({
    id: SHELL_IDS.user,
    username: "maria.lopez",
    email: "maria.lopez@example.test",
    email_verified: true,
    enabled: true,
    first_name: "María",
    last_name: "López",
    attributes: {"area-id": [SHELL_IDS.area]},
    groups: [],
    area: {id: SHELL_IDS.area, name: "North district"},
    votes_info: [],
    ...row,
})

export const keycloakRole = (row: Row = {}): Row => ({
    id: "a0000000-0000-4000-8000-000000000001",
    name: "election-manager",
    permissions: {},
    access: {},
    attributes: {},
    client_roles: {},
    ...row,
})

export const profileAttribute = (name: string, display_name: string): Row => ({
    name,
    display_name,
    multivalued: false,
    annotations: {},
    validations: {},
    group: null,
    required: null,
    permissions: null,
    selector: null,
})

/** Seven days of the event's statistics, as the stats actions return them. */
export const eventStats = {
    total_eligible_voters: 120,
    total_distinct_voters: 45,
    total_elections: 1,
    total_areas: 1,
    voters_by_channel: [
        {channel: "online", count: 40},
        {channel: "kiosk", count: 5},
    ],
    votes_per_day: [
        {day: "2026-01-14", bucket: "2026-01-14", channel: "online", day_count: 25},
        {day: "2026-01-15", bucket: "2026-01-15", channel: "online", day_count: 15},
        {day: "2026-01-15", bucket: "2026-01-15", channel: "kiosk", day_count: 5},
    ],
}
export const eventStatistics = {num_emails_sent: 12, num_sms_sent: 3}

/**
 * Every read a tenant administrator's session makes on the shared shell:
 * the React-admin tables, the sidebar tree, dashboards, users and roles.
 */
export function serveAdminTenant(portal: PortalServices, tables = adminTables()) {
    const all: Record<string, Row[]> = {
        sequent_backend_tally_session: [],
        sequent_backend_keys_ceremony: [],
        ...tables,
    }
    serveTables(portal, all)
    serveTree(portal, all)
    const {
        total_elections: _elections,
        total_eligible_voters: _eligible,
        ...electionStats
    } = eventStats
    // The mock resolves root fields by name, so replies are keyed by field, not by alias.
    portal.graphql.on("GetElectionEventStats", () => ({
        data: {
            getElectionEventStats: eventStats,
            sequent_backend_election_event: [{statistics: eventStatistics}],
        },
    }))
    portal.graphql.on("GetElectionStats", () => ({
        data: {
            getElectionStats: electionStats,
            count_users: {count: 120},
            sequent_backend_election: [{statistics: eventStatistics}],
        },
    }))
    portal.graphql.on("GetCastVotesByIp", () => ({
        data: {get_top_votes_by_ip: {items: [], total: {aggregate: {count: 0}}}},
    }))
    portal.graphql.on("ListKeysCeremony", () => ({
        data: {list_keys_ceremony: {items: [], total: {aggregate: {count: 0}}}},
    }))
    portal.graphql.on("getUsers", () => ({
        data: {get_users: {items: [keycloakUser()], total: {aggregate: {count: 1}}}},
    }))
    portal.graphql.on("getRoles", () => ({
        data: {get_roles: {items: [keycloakRole()], total: {aggregate: {count: 1}}}},
    }))
    portal.graphql.on("ListUserRoles", () => ({data: {list_user_roles: []}}))
    portal.graphql.on("GetUserProfileConfiguration", () => ({
        data: {
            get_user_profile_configuration: {
                attributes: [
                    profileAttribute("username", "Username"),
                    profileAttribute("email", "Email"),
                    profileAttribute("firstName", "First name"),
                    profileAttribute("lastName", "Last name"),
                ],
                groups: [],
            },
        },
    }))
    portal.graphql.on("get_area_with_area_contests", ({variables}) => ({
        data: {
            sequent_backend_area_contest: (all.sequent_backend_area_contest ?? [])
                .filter((row) => row.area_id === variables.areaId)
                .map((row) => ({
                    id: row.id,
                    contest: all.sequent_backend_contest?.find(
                        (item) => item.id === row.contest_id
                    ),
                })),
        },
    }))
}
