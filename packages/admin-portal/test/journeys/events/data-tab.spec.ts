// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {
    electionEvent,
    EVENT_ID,
    EVENT_ROLES,
    FIXED_TIME,
    mockEvent,
    openEvent,
    TENANT_ID,
    type Row,
} from "./data"

const TASK_ID = "44444444-4444-4444-8444-444444444444"
const DOCUMENT_ID = "33333333-3333-4333-8333-333333333333"
test.use({roles: [...EVENT_ROLES, "election-event-data-tab"]})

/** Serves an event that absorbs `update_sequent_backend_election_event`, like Hasura would. */
function editableEvent(portal: AdminPortal, presentation: Row = {}) {
    let event = electionEvent(
        {alias: "Council"},
        {
            i18n: {
                en: {
                    name: "Council election",
                    alias: "Council",
                    description: "Annual council election",
                },
            },
            ...presentation,
        }
    )
    mockEvent(portal, () => event)
    portal.graphql.on("SetCustomUrls", () => ({
        data: {set_custom_urls: {success: true, message: "Custom URL updated"}},
    }))
    portal.graphql.on("SetVoterAuthentication", () => ({
        data: {set_voter_authentication: {success: true, message: "Updated"}},
    }))
    portal.graphql.on("update_sequent_backend_election_event", ({variables}) => {
        event = {...event, ...(variables._set as Row)}
        return {
            data: {update_sequent_backend_election_event: {affected_rows: 1, returning: [event]}},
        }
    })
}

function customUrlCalls(portal: AdminPortal, prefixes = {login: "", enrollment: "", saml: ""}) {
    const voting = `${portal.origin}/voting/tenant/${TENANT_ID}/event/${EVENT_ID}`
    return [
        {
            origin: `https://${prefixes.login}.vote.example`,
            redirect_to: `${voting}/login`,
            dns_prefix: prefixes.login,
            election_id: EVENT_ID,
            key: "login",
        },
        {
            origin: `https://${prefixes.enrollment}.vote.example`,
            redirect_to: `${voting}/enroll`,
            dns_prefix: prefixes.enrollment,
            election_id: EVENT_ID,
            key: "enrollment",
        },
        {
            origin: `https://${prefixes.saml}.vote.example`,
            redirect_to: `${portal.origin}/keycloak/realms/tenant-${TENANT_ID}-event-${EVENT_ID}/broker/simplesamlphp/endpoint`,
            dns_prefix: prefixes.saml,
            election_id: EVENT_ID,
            key: "saml",
        },
    ]
}

/** The presentation defaults the form writes back on the first save of a bare event. */
const SAVED_PRESENTATION_DEFAULTS = {
    language_conf: {
        enabled_language_codes: ["en"],
        default_language_code: "en",
        language_detection_policy: "browser-detect",
    },
    elections_order: "alphabetical",
    voting_portal_countdown_policy: {policy: "NO_COUNTDOWN"},
    custom_urls: {},
    skip_election_list: false,
    show_user_profile: false,
    show_cast_vote_logs: "hide-logs-tab",
    automatic_recount_policy: "disabled",
    materials: {policy: "off"},
    contest_encryption_policy: "single-contest",
    locked_down: "not-locked-down",
    decoded_ballot_inclusion_policy: "not-included",
    ceremonies_policy: "manual-ceremonies",
    weighted_voting_policy: "disabled-weighted-voting",
    delegated_voting_policy: "disabled",
    voting_portal_datetime_format: "legacy-gb-24h",
    voter_signing_policy: "no-signature",
    voter_certificate_policy: "disabled",
}

async function save(page: Page, portal: AdminPortal) {
    const before = portal.graphql.callsTo("update_sequent_backend_election_event").length
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_election_event").length)
        .toBe(before + 1)
    return portal.graphql.callsTo("update_sequent_backend_election_event")[before].variables
}

test.beforeEach(({portal}) => {
    portal.settings.CUSTOM_URLS_DOMAIN_NAME = "vote.example"
})

test("saves general edits through custom URLs, voter authentication and the event update", async ({
    page,
    portal,
}) => {
    editableEvent(portal)
    await openEvent(page, portal)
    const name = page.getByRole("textbox", {name: "Name", exact: true})
    await expect(name).toHaveValue("Council election")
    await expect(page.getByRole("textbox", {name: "Alias", exact: true})).toHaveValue("Council")
    await expect(page.getByRole("textbox", {name: "Description", exact: true})).toHaveValue(
        "Annual council election"
    )
    await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    await name.fill("City council 2026")
    await page.getByRole("textbox", {name: "Description", exact: true}).fill("Renewed council")

    const update = await save(page, portal)
    expect(update).toEqual({
        _set: {
            description: "Renewed council",
            presentation: {
                ...SAVED_PRESENTATION_DEFAULTS,
                i18n: {
                    en: {
                        name: "City council 2026",
                        alias: "Council",
                        description: "Renewed council",
                    },
                },
            },
        },
        where: {id: {_eq: EVENT_ID}},
    })
    // Custom URLs and voter authentication are always sent before the event update.
    expect(portal.graphql.callsTo("SetCustomUrls").map(({variables}) => variables)).toEqual(
        customUrlCalls(portal)
    )
    expect(
        portal.graphql.callsTo("SetVoterAuthentication").map(({variables}) => variables)
    ).toEqual([{electionEventId: EVENT_ID, enrollment: "", otp: ""}])
    for (const call of portal.graphql.callsTo("SetCustomUrls"))
        expect(call.headers["x-hasura-role"]).toBe("election-event-write")
    // The edit refreshes instead of notifying: the saved values come back and Save disables.
    await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    await expect(name).toHaveValue("City council 2026")
})

test("exports an encrypted archive, tracks its task and reveals the generated password", async ({
    context,
    page,
    portal,
}) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"], {origin: portal.origin})
    editableEvent(portal)
    let status = "IN_PROGRESS"
    const task = () => ({
        id: TASK_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: "Export Election Event",
        type: "EXPORT_ELECTION_EVENT",
        execution_status: status,
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: status === "IN_PROGRESS" ? null : FIXED_TIME,
        executed_by_user: "synthetic-admin",
        annotations: {},
        labels: {},
        logs: [],
    })
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    portal.graphql.on("ExportElectionEvent", () => ({
        data: {
            export_election_event: {
                password: "Generated-Archive-Pass",
                document_id: DOCUMENT_ID,
                task_execution: task(),
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({data: {sequent_backend_tasks_execution: [task()]}}))
    await openEvent(page, portal)
    await page.getByRole("button", {name: "Export", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(dialog.getByText("Export Election Event", {exact: true})).toBeVisible()
    await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(dialog).toHaveCount(0)
    expect(portal.graphql.callsTo("ExportElectionEvent")).toHaveLength(0)

    await page.getByRole("button", {name: "Export", exact: true}).click()
    await dialog.getByRole("checkbox", {name: "Encrypt with Password"}).check()
    await dialog.getByRole("checkbox", {name: "Include Voters"}).check()
    // Tally needs the bulletin board, so ticking it ticks the board as well.
    await dialog.getByRole("checkbox", {name: "Tally", exact: true}).check()
    await expect(dialog.getByRole("checkbox", {name: "Bulletin Board"})).toBeChecked()
    await dialog.getByRole("button", {name: "Export", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("ExportElectionEvent").length).toBe(1)
    const call = portal.graphql.callsTo("ExportElectionEvent")[0]
    expect(call.variables).toEqual({
        electionEventId: EVENT_ID,
        exportConfigurations: {
            is_encrypted: true,
            encrypt_with_password: true,
            include_voters: true,
            activity_logs: false,
            bulletin_board: true,
            publications: false,
            s3_files: false,
            scheduled_events: false,
            reports: false,
            applications: false,
            tally: true,
            include_certificates: false,
        },
    })
    expect(call.headers["x-hasura-role"]).toBe("election-event-read")
    const password = page.getByRole("dialog", {name: "Password"})
    await expect(password.getByRole("textbox").first()).toHaveValue("Generated-Archive-Pass")
    await password.getByRole("button", {name: "Copy Password"}).first().click()
    await expect(page.getByText("Password copied to clipboard", {exact: true})).toBeVisible()
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("Generated-Archive-Pass")
    await expect(page.getByText("Task: Export Election Event", {exact: true})).toBeVisible()
    await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
    expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: TASK_ID})
    await password.getByRole("button", {name: "Ok", exact: true}).click()
    await expect(password).toHaveCount(0)
    status = "SUCCESS"
    await page.clock.runFor(250)
    await expect(page.getByText("SUCCESS", {exact: true})).toBeVisible()
})
