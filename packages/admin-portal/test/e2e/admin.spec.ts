// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../../../e2e/fixtures"
import {login} from "../../../voting-portal/test/load/flow"
import {randomUUID} from "node:crypto"

test("administrator can inspect the freshly provisioned election @smoke @probe", async ({
    page,
    fixture,
}) => {
    await page.goto(fixture.adminUrl)
    await login(page, {username: fixture.adminUsername, password: fixture.adminPassword})
    await expect(page).toHaveURL(/sequent_backend_election_event/)
    await expect(page.getByRole("main")).toBeVisible()
    await expect(page.locator(`a[href*="${fixture.eventId}"]`).first()).toBeVisible()
})

test("administrator can create, archive, restore and delete an owned event", async ({
    page,
    fixture,
}) => {
    // This local-only scenario owns a separate event; voting/probes retain their fixture.
    // A failed run still removes its entire isolated database during stack cleanup.
    const name = `E2E administration ${randomUUID()}`
    await page.goto(fixture.adminUrl)
    await login(page, {username: fixture.adminUsername, password: fixture.adminPassword})
    await page.locator(".election-event-create-button").click()
    await page.getByRole("menuitem", {name: "Create an Election Event", exact: true}).click()
    await page.locator('input[name="name"]').fill(name)
    await page.locator('input[name="description"]').fill("Owned by the isolated E2E run")
    const created = page.waitForResponse((response) =>
        response.url().includes("/v1/graphql") &&
        Boolean(response.request().postData()?.includes("mutation CreateElectionEvent"))
    )
    await page.locator(".election-event-save-button").click()
    const response = await created
    expect(response.ok()).toBe(true)
    const body = await response.json()
    expect(body.errors).toBeUndefined()
    expect(body.data?.insertElectionEvent?.error).toBeFalsy()
    const eventId = body.data?.insertElectionEvent?.id
    expect(eventId).toMatch(/^[0-9a-f-]{36}$/)
    expect(eventId).not.toBe(fixture.eventId)
    await expect(page).toHaveURL(new RegExp(`/sequent_backend_election_event/${eventId}`))

    const event = page.locator(`a[title="${name}"]`)
    await page.reload()
    await expect(event).toBeVisible()
    const openActions = async () => {
        await event.hover()
        await event.locator("..").locator(".menu-actions-sequent_backend_election_event #MoreHorizIcon").click()
    }

    await openActions()
    await page.getByRole("menuitem", {name: "Archive this Election Event", exact: true}).click()
    await page.getByRole("dialog").getByRole("button", {name: "Archive", exact: true}).click()
    await expect(event).toHaveCount(0)
    await page.reload()
    await expect(page.getByRole("main")).toBeVisible()
    await expect(event).toHaveCount(0)
    await page.getByText("Archived", {exact: true}).click()
    await expect(event).toBeVisible()

    await openActions()
    await page.getByRole("menuitem", {name: "Unarchive this Election Event", exact: true}).click()
    await page.getByRole("dialog").getByRole("button", {name: "Unarchive", exact: true}).click()
    await expect(event).toHaveCount(0)
    await page.getByText("Active", {exact: true}).click()
    await expect(event).toBeVisible()

    await openActions()
    await page.getByRole("menuitem", {name: "Remove this Election Event", exact: true}).click()
    await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
    await expect(event).toHaveCount(0)
    await page.reload()
    await expect(page.locator(`a[href*="${fixture.eventId}"]`).first()).toBeVisible()
    await expect(event).toHaveCount(0)
})
