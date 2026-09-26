// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Locator, Page} from "@playwright/test"
import {test, expect, TENANT_ID} from "../fixtures"
import type {AdminPortal} from "../fixtures"
import {
    ALICE_ID,
    BOB_ID,
    CONTEST_ID,
    EVENT_ID,
    FIXED_TIME,
    TALLY_ID,
    TALLY_ROLES,
    TRUSTEES,
    serveResults,
    tallyWorld,
} from "./data"

test.use({roles: TALLY_ROLES})

const rowAction = (row: Locator, label: string) =>
    row.locator(`button:has(svg[aria-label="${label}"])`)

async function openTallyList(page: Page, portal: AdminPortal) {
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    await page.goto(`${portal.origin}/sequent_backend_election_event/${EVENT_ID}?lang=en`)
    await page.getByRole("tab", {name: "Tally", exact: true}).click()
    const row = page.getByRole("row").filter({hasText: TALLY_ID})
    await expect(row).toBeVisible()
    return row
}

test("a manual tally ceremony starts once trustees restore their keys and resolves a tie", async ({
    page,
    portal,
}) => {
    const world = tallyWorld(portal)
    portal.graphql.on("UpdateTallyCeremony", ({variables}) => {
        world.session.execution_status = String(variables.status)
        return {data: {update_tally_ceremony: {tally_session_id: TALLY_ID}}}
    })

    const row = await openTallyList(page, portal)
    await rowAction(row, "View Tally Ceremony").click()
    const start = page.getByRole("button", {name: "Start Tally", exact: true})
    await expect(page.getByRole("gridcell", {name: TRUSTEES[0], exact: true})).toBeVisible()
    await expect(page.getByText("0/2 trustees imported the key", {exact: true})).toBeVisible()
    await expect(page.getByRole("alert")).toContainText(
        "You cannot continue the ceremony because the tally session is not connected"
    )
    await expect(start).toBeDisabled()

    world.execution.status.trustees = TRUSTEES.map((name) => ({name, status: "KEY_RESTORED"}))
    world.session.execution_status = "CONNECTED"
    await page.clock.runFor(101)
    await expect(page.getByText("2/2 trustees imported the key", {exact: true})).toBeVisible()
    await expect(start).toBeEnabled()
    await start.click()
    const dialog = page.getByRole("dialog")
    await expect(dialog).toContainText("All required trustees have verified their key fragments.")
    expect(portal.graphql.callsTo("UpdateTallyCeremony")).toEqual([])
    await dialog.getByRole("button", {name: "Start Tally", exact: true}).click()
    await expect(page.getByText("Tally started", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("UpdateTallyCeremony").map(({variables}) => variables)).toEqual([
        {election_event_id: EVENT_ID, tally_session_id: TALLY_ID, status: "IN_PROGRESS"},
    ])

    await serveResults(portal, world)
    world.session.execution_status = "AWAITING_INPUT"
    world.resolutions = [tie()]
    portal.graphql.on("SubmitTallyResolution", () => {
        world.resolutions = [
            tie({
                status: "resolved",
                resolved_at: FIXED_TIME,
                resolved_by_user: "harbour-admin",
                resolution_data: {...tieData, resolved_by_candidate_id: ALICE_ID},
            }),
        ]
        world.session.execution_status = "IN_PROGRESS"
        return {
            data: {
                submit_tally_resolution: {
                    success: true,
                    tally_session_id: TALLY_ID,
                    resolved_count: 1,
                },
            },
        }
    })
    await page.clock.runFor(101)
    const item = page.getByText("Tie Resolution Required", {exact: true})
    await expect(item).toBeVisible()
    await expect(page.getByRole("heading", {name: "Pending resolutions (1)"})).toBeVisible()
    await expect(
        page.getByText("Tie Resolution Required: Harbour election | Mayor | Round 1")
    ).toBeVisible()
    await item.click()
    await expect(page.getByRole("alert").filter({hasText: "Tally paused"})).toHaveText(
        "Tally paused due to unresolved tie (Round 1)" +
            "Candidates tied (30 votes, 50.0%): Alice Example, Bob Example. " +
            "Manual tie-break required to continue tally."
    )
    const apply = page.getByRole("button", {name: "Apply Resolutions and Recalculate"})
    await expect(apply).toBeDisabled()
    await page.getByRole("combobox").last().click()
    await page.getByRole("option", {name: "Alice Example", exact: true}).click()
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await expect(page.getByText("Pending calculation", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("SubmitTallyResolution")).toEqual([])
    await apply.click()
    await expect(
        page.getByText("Resolutions submitted. Tally is resuming...", {exact: true})
    ).toBeVisible()
    const submitted = portal.graphql.callsTo("SubmitTallyResolution")
    expect(submitted.map(({variables}) => variables)).toEqual([
        {
            election_event_id: EVENT_ID,
            tally_session_id: TALLY_ID,
            resolutions: [{contest_id: CONTEST_ID, selected_candidate_id: ALICE_ID}],
        },
    ])
    expect(submitted[0].headers["x-hasura-role"]).toBe("tally-resolution-submit")
})

const tieData = {
    round_number: 1,
    tied_candidate_ids: [ALICE_ID, BOB_ID],
    vote_count: 30,
    method_used: "manual",
}

function tie(overrides: Record<string, unknown> = {}) {
    return {
        id: "63000000-0000-4000-8000-000000000001",
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        tally_session_id: TALLY_ID,
        contest_id: CONTEST_ID,
        area_id: null,
        resolution_type: "tie_break",
        status: "pending",
        resolution_data: tieData,
        resolved_at: null,
        resolved_by_user: null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        ...overrides,
    }
}
