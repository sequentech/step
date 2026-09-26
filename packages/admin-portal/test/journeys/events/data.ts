// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {GraphQLHandler} from "@sequentech/ui-test-kit/mocks/graphql"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {TENANT_ID, type AdminPortal} from "../fixtures"

export type Row = Record<string, unknown>
export const EVENT_ID = IDS.event
export const EVENT_URL = `/sequent_backend_election_event/${EVENT_ID}`
/** Roles every event page needs; specs add the tab and action permissions they exercise. */
export const EVENT_ROLES = [
    "admin-user",
    "election-event-read",
    "election-event-write",
    "election-read",
]

/** A synthetic election event with every non-null column the portal selects. */
export function electionEvent(overrides: Row = {}, presentation: Row = {}): Row {
    const name = typeof overrides.name === "string" ? overrides.name : "Council election"
    return {
        id: EVENT_ID,
        tenant_id: TENANT_ID,
        name,
        alias: null,
        description: "Annual council election",
        encryption_protocol: "RSA256",
        is_archived: false,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        elections: [],
        elections_aggregate: {aggregate: {count: 0}, nodes: []},
        presentation: {
            i18n: {en: {name}},
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
            ...presentation,
        },
        status: {},
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: false},
        ...overrides,
    }
}

/** Answers a react-admin list, getOne or getMany query for `resource`. */
export function listOf(resource: string, rows: Row[] | (() => Row[])): GraphQLHandler {
    return () => {
        const items = typeof rows === "function" ? rows() : rows
        return {
            data: {
                [resource]: items,
                [`${resource}_aggregate`]: {aggregate: {count: items.length}},
            },
        }
    }
}

export const UNCONFIGURED_PASSWORD_POLICY = {
    configured: false,
    minimum_length: null,
    maximum_length: null,
    include_uppercase: null,
    include_lowercase: null,
    include_digits: null,
    include_special_characters: null,
}

/** Registers the queries every election event page makes, answered from `event`. */
export function mockEvent(portal: AdminPortal, event: Row | (() => Row) = electionEvent()) {
    const current = typeof event === "function" ? event : () => event
    portal.graphql.on(
        "sequent_backend_election_event",
        listOf("sequent_backend_election_event", () => [current()])
    )
    portal.graphql.on("election_events_tree", () => ({
        data: {sequent_backend_election_event: [current()]},
    }))
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: []}}))
    portal.graphql.on("sequent_backend_election", listOf("sequent_backend_election", []))
    portal.graphql.on(
        "sequent_backend_support_material",
        listOf("sequent_backend_support_material", [])
    )
    portal.graphql.on("GetRealmPasswordPolicy", () => ({
        data: {get_realm_password_policy: UNCONFIGURED_PASSWORD_POLICY},
    }))
}

/** Opens the event page, optionally switching to the tab labelled `tab`. */
export async function openEvent(page: Page, portal: AdminPortal, tab?: string) {
    await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
    if (tab) await page.getByRole("tab", {name: tab, exact: true}).click()
}

export {FIXED_TIME, TENANT_ID}
