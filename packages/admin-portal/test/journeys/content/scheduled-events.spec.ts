// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    type Row,
    electionRow,
    eventPage,
    expectRole,
    iconButton,
    names,
    notification,
    openEventTab,
    table,
} from "./data"

const readerRoles = [...BASE_ROLES, "election-event-scheduled-tab"]
const writerRoles = [
    ...readerRoles,
    "scheduled-event-write",
    "scheduled-event-create",
    "scheduled-event-delete",
    "election-event-scheduled-event-columns",
]
const END_EVENT_ID = "e0000000-0000-4000-8000-000000000002"

function scheduledRow(overrides: Row = {}): Row {
    return {
        id: CONTENT_IDS.scheduledEvent,
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        created_at: FIXED_TIME,
        created_by: "synthetic-admin",
        event_processor: "START_VOTING_PERIOD",
        cron_config: {scheduled_date: "2026-02-01T09:00:00.000Z"},
        event_payload: {election_id: IDS.election, voting_channels: ["ONLINE", "KIOSK"]},
        stopped_at: null,
        archived_at: null,
        task_id: null,
        annotations: {},
        labels: {},
        ...overrides,
    }
}

function scheduled(portal: PortalServices, initial = [scheduledRow()]) {
    eventPage(portal)
    table(portal, "sequent_backend_election", [
        electionRow({presentation: names("Mayor election")}),
    ])
    const rows = table(portal, "sequent_backend_scheduled_event", initial)
    portal.graphql.on("ManageElectionDates", ({variables}) => {
        const existing = rows.find(
            (row) =>
                row.event_processor === variables.eventProcessor &&
                (row.event_payload as Row).election_id === (variables.electionId ?? null)
        )
        if (!variables.scheduledDate && existing) existing.archived_at = FIXED_TIME
        else if (existing) existing.cron_config = {scheduled_date: variables.scheduledDate}
        else
            rows.push(
                scheduledRow({
                    id: END_EVENT_ID,
                    event_processor: variables.eventProcessor,
                    cron_config: {scheduled_date: variables.scheduledDate},
                    event_payload: {election_id: variables.electionId ?? null},
                })
            )
        return {data: {manage_election_dates: {error_msg: null}}}
    })
    return rows
}

async function openSchedule(page: Page, portal: PortalServices) {
    await openEventTab(page, portal, "Scheduled Events")
    await expect(page.getByRole("cell", {name: "Start Voting Period", exact: true})).toBeVisible()
}

test.describe("schedule administrator", () => {
    test.use({roles: writerRoles})

    test("lists scheduled events of the event's elections with readable dates", async ({
        page,
        portal,
    }) => {
        scheduled(portal, [
            scheduledRow(),
            scheduledRow({
                id: END_EVENT_ID,
                event_processor: "START_ENROLLMENT_PERIOD",
                event_payload: {election_id: null},
                stopped_at: "2026-01-20T10:00:00.000Z",
            }),
        ])
        await openSchedule(page, portal)
        const start = page.getByRole("row").filter({hasText: "Start Voting Period"})
        await expect(start.getByRole("cell", {name: "Mayor election", exact: true})).toBeVisible()
        await expect(start.getByText(/Sun Feb 01 2026 09:00:00/)).toBeVisible()
        const enrollment = page.getByRole("row").filter({hasText: "Start Enrollment Period"})
        await expect(enrollment.getByText(/Tue Jan 20 2026 10:00:00/)).toBeVisible()
        // The list first asks for event-wide schedules, then adds the loaded elections.
        const list = portal.graphql
            .callsTo("sequent_backend_scheduled_event")
            .filter((call) => call.variables.limit)
            .at(-1)
        expect(list?.variables.where).toEqual({
            _and: [
                {election_event_id: {_eq: IDS.event}},
                {tenant_id: {_eq: TENANT_ID}},
                {archived_at: {_is_null: true}},
                {
                    _or: [
                        {event_payload: {_contains: {election_id: IDS.election}}},
                        {event_payload: {_contains: {election_id: null}}},
                    ],
                },
            ],
        })
    })

    test("schedules the end of voting for an election and its channels", async ({page, portal}) => {
        const rows = scheduled(portal)
        await openSchedule(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create Scheduled Event"})
        await drawer.getByRole("combobox", {name: "Type"}).click()
        await page.getByRole("option", {name: "End Voting Period", exact: true}).click()
        await drawer.getByRole("combobox", {name: "Election"}).fill("Mayor")
        await page.clock.runFor(500)
        await page.getByRole("option", {name: "Mayor election", exact: true}).click()
        await drawer.getByRole("checkbox", {name: "Early voting"}).check()
        await drawer.getByRole("checkbox", {name: "Kiosk"}).uncheck()
        await drawer.getByLabel("End Date and Time (UTC)").fill("2026-02-02T18:30")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(drawer).not.toBeVisible()
        expect(portal.graphql.callsTo("ManageElectionDates")[0].variables).toEqual({
            electionEventId: IDS.event,
            electionId: IDS.election,
            scheduledDate: "2026-02-02T18:30:00.000Z",
            eventProcessor: "END_VOTING_PERIOD",
            votingChannels: ["ONLINE", "EARLY_VOTING"],
        })
        expectRole(portal, "ManageElectionDates", "scheduled-event-write")
        await expect(page.getByRole("cell", {name: "End Voting Period", exact: true})).toBeVisible()
        expect(rows).toHaveLength(2)
    })

    test("confirms a new schedule as created", async ({page, portal}) => {
        scheduled(portal)
        await openSchedule(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create Scheduled Event"})
        await drawer.getByLabel("Start Date and Time (UTC)").fill("2026-02-01T09:00")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("ManageElectionDates").length).toBe(1)

        await expect(notification(page, "Scheduled Event created successfully")).toBeVisible({
            timeout: 3000,
        })
    })

    test("opens a single create drawer", async ({page, portal}) => {
        scheduled(portal)
        await openSchedule(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        await expect(
            page.getByRole("dialog").filter({hasText: "Create Scheduled Event"})
        ).toBeVisible()

        await expect(page.getByRole("dialog", {includeHidden: true})).toHaveCount(1, {
            timeout: 1000,
        })
    })

    test("blocks a start schedule that opens online and early voting together", async ({
        page,
        portal,
    }) => {
        scheduled(portal)
        await openSchedule(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create Scheduled Event"})
        await drawer.getByRole("checkbox", {name: "Early voting"}).check()
        await expect(
            drawer.getByText(
                "A start schedule cannot open Online and Early voting together: early voting has to start before online voting."
            )
        ).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
        await drawer.getByRole("checkbox", {name: "Online"}).uncheck()
        await expect(drawer.getByRole("button", {name: "Save", exact: true})).toBeEnabled()
        expect(portal.graphql.callsTo("ManageElectionDates")).toHaveLength(0)
    })

    test("reschedules an event and archives another after confirmation", async ({page, portal}) => {
        const rows = scheduled(portal, [
            scheduledRow(),
            scheduledRow({
                id: END_EVENT_ID,
                event_processor: "ALLOW_TALLY",
                cron_config: {scheduled_date: "2026-02-03T08:00:00.000Z"},
            }),
        ])
        await openSchedule(page, portal)
        const start = page.getByRole("row").filter({hasText: "Start Voting Period"})
        await iconButton(start, "edit-voter-icon").click()
        const drawer = page.getByRole("dialog").filter({hasText: "Edit Scheduled Event"})
        await expect(drawer.getByRole("combobox", {name: "Type"})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
        await expect(drawer.getByRole("textbox", {name: "Election"})).toHaveValue("Mayor election")
        await drawer.getByLabel("Start Date and Time (UTC)").fill("2026-02-01T10:15")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Scheduled Event edited successfully")).toBeVisible()
        expect(portal.graphql.callsTo("ManageElectionDates")[0].variables).toEqual({
            electionEventId: IDS.event,
            electionId: IDS.election,
            scheduledDate: "2026-02-01T10:15:00.000Z",
            eventProcessor: "START_VOTING_PERIOD",
            votingChannels: ["ONLINE", "KIOSK"],
        })

        const tally = page.getByRole("row").filter({hasText: "Allow Tally"})
        await iconButton(tally, "delete-voter-icon").click()
        const confirm = page
            .getByRole("dialog")
            .filter({hasText: "Are you sure you want delete this Scheduled Event?"})
        await confirm.getByRole("button", {name: "Delete", exact: true}).click()
        await expect(page.getByRole("cell", {name: "Allow Tally", exact: true})).not.toBeVisible()
        expect(portal.graphql.callsTo("ManageElectionDates")[1].variables).toEqual({
            electionEventId: IDS.event,
            electionId: IDS.election,
            eventProcessor: "ALLOW_TALLY",
        })
        expect(rows.find((row) => row.id === END_EVENT_ID)?.archived_at).toBe(FIXED_TIME)
    })

    test("reports a schedule the API rejects", async ({page, portal}) => {
        scheduled(portal)
        portal.graphql.on("ManageElectionDates", () => ({
            data: {manage_election_dates: {error_msg: "Synthetic overlapping schedule"}},
        }))
        await openSchedule(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog").filter({hasText: "Create Scheduled Event"})
        await drawer.getByLabel("Start Date and Time (UTC)").fill("2026-02-01T09:00")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(notification(page, "Error creating Scheduled Event")).toBeVisible()
        expect(portal.graphql.callsTo("ManageElectionDates")[0].variables).toMatchObject({
            electionId: null,
            eventProcessor: "START_VOTING_PERIOD",
            votingChannels: ["ONLINE", "KIOSK"],
        })
    })
})

test.describe("schedule reader", () => {
    test.use({roles: readerRoles})

    test("sees scheduled events without create, edit or delete actions", async ({page, portal}) => {
        scheduled(portal)
        await openSchedule(page, portal)
        await expect(page.getByRole("button", {name: "Add", exact: true})).not.toBeVisible()
        const start = page.getByRole("row").filter({hasText: "Start Voting Period"})
        await expect(start.getByRole("button")).toHaveCount(0)
    })

    test("shows the empty state without the create button", async ({page, portal}) => {
        scheduled(portal, [])
        await openEventTab(page, portal, "Scheduled Events")
        await expect(page.getByText("No Scheduled Events yet.", {exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Create Scheduled Event"})).not.toBeVisible()
    })
})
