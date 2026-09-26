// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    candidateRow,
    contestRow,
    electionRow,
    eventPage,
    expectRole,
    notification,
    table,
} from "./data"

const roles = [
    ...BASE_ROLES,
    "election-event-create",
    "election-write",
    "election-delete",
    "election-data-tab",
    "contest-read",
    "contest-write",
    "contest-delete",
    "candidate-read",
    "candidate-write",
    "candidate-delete",
]
test.use({roles})

const items = [
    {
        kind: "election",
        id: CONTENT_IDS.election,
        name: "Mayor election",
        parent: "election_event",
        parentId: CONTENT_IDS.event,
    },
    {
        kind: "contest",
        id: CONTENT_IDS.contest,
        name: "Mayor",
        parent: "election",
        parentId: CONTENT_IDS.election,
    },
    {
        kind: "candidate",
        id: CONTENT_IDS.candidate,
        name: "Alice Adams",
        parent: "contest",
        parentId: CONTENT_IDS.contest,
    },
] as const

function ballot(portal: AdminPortal) {
    eventPage(portal)
    const elections = table(portal, "sequent_backend_election", [electionRow()])
    const contests = table(portal, "sequent_backend_contest", [contestRow()])
    const candidates = table(portal, "sequent_backend_candidate", [candidateRow()])
    table(portal, "sequent_backend_document", [])
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: elections}}))
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: contests}}))
    portal.graphql.on("candidate_tree", () => ({data: {sequent_backend_candidate: candidates}}))
    return {election: elections, contest: contests, candidate: candidates}
}

async function removeDialog(page: Page, item: (typeof items)[number]) {
    await page.getByRole("link", {name: item.name, exact: true}).hover()
    // The menu exposes this icon hook but no accessible button/name.
    await page.locator(`.menu-actions-sequent_backend_${item.kind} #MoreHorizIcon`).click()
    const label = `Remove this ${item.kind[0].toUpperCase()}${item.kind.slice(1)}`
    const action = page.getByRole("menuitem", {name: label, exact: true})
    await expect(action).toBeVisible()
    await action.click()
    return page.getByRole("dialog", {name: "Warning", exact: true})
}

for (const item of items) {
    test(`sidebar cancels then deletes only the selected ${item.kind}`, async ({page, portal}) => {
        const records = ballot(portal)
        const operation = `delete_sequent_backend_${item.kind}`
        await page.goto(`${portal.origin}/sequent_backend_${item.kind}/${item.id}?lang=en`)
        let dialog = await removeDialog(page, item)
        await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
        expect(portal.graphql.callsTo(operation)).toEqual([])
        dialog = await removeDialog(page, item)
        await dialog.getByRole("button", {name: "Delete", exact: true}).click()
        await expect(notification(page, "The item has been deleted")).toBeVisible()
        expect(portal.graphql.callsTo(operation).map(({variables}) => variables)).toEqual([
            {where: {id: {_eq: item.id}}},
        ])
        expectRole(portal, operation, `${item.kind}-delete`)
        expect(records[item.kind]).toEqual([])
        await expect(page.getByRole("link", {name: item.name, exact: true})).toHaveCount(0)

        await expect(page).toHaveURL(
            (url) => url.pathname === `/sequent_backend_${item.parent}/${item.parentId}`
        )
    })
}

test("a rejected sidebar deletion keeps the candidate and reports the failure", async ({
    page,
    portal,
}) => {
    const records = ballot(portal)
    const item = items[2]
    const operation = "delete_sequent_backend_candidate"
    portal.graphql.on(operation, () => ({errors: [{message: "Synthetic deletion rejected"}]}))
    await page.goto(`${portal.origin}/sequent_backend_candidate/${item.id}?lang=en`)
    const dialog = await removeDialog(page, item)
    await dialog.getByRole("button", {name: "Delete", exact: true}).click()
    await expect(notification(page, "Error while trying to delete this item")).toBeVisible()
    expect(portal.graphql.callsTo(operation).map(({variables}) => variables)).toEqual([
        {where: {id: {_eq: item.id}}},
    ])
    expectRole(portal, operation, "candidate-delete")
    expect(records.candidate.map(({id}) => id)).toEqual([item.id])
    await expect(page).toHaveURL((url) => url.pathname === `/sequent_backend_candidate/${item.id}`)
    await expect(page.getByRole("link", {name: item.name, exact: true})).toBeVisible()
})

test.describe("sidebar deletion permissions", () => {
    test.use({roles: roles.filter((role) => role !== "election-event-create")})
    test("candidate deletion does not require the unrelated event-create permission", async ({
        page,
        portal,
    }) => {
        ballot(portal)
        await page.goto(
            `${portal.origin}/sequent_backend_candidate/${CONTENT_IDS.candidate}?lang=en`
        )
        await expect(page.getByRole("textbox", {name: "Name", exact: true})).toHaveValue(
            "Alice Adams"
        )
        await page.getByRole("link", {name: "Alice Adams", exact: true}).hover()
        expect(portal.graphql.callsTo("delete_sequent_backend_candidate")).toEqual([])

        await expect(
            page.locator(".menu-actions-sequent_backend_candidate #MoreHorizIcon")
        ).toBeVisible()
    })
})

test.describe("sidebar sibling creation", () => {
    test.use({roles: [...roles, "election-create", "contest-create", "candidate-create"]})
    for (const item of items.filter((item) => item.kind !== "election")) {
        test(`opens a sibling ${item.kind} form in the same event and parent`, async ({
            page,
            portal,
        }) => {
            ballot(portal)
            await page.goto(`${portal.origin}/sequent_backend_${item.kind}/${item.id}?lang=en`)
            await page.getByRole("link", {name: item.name, exact: true}).hover()
            await page.locator(`.menu-actions-sequent_backend_${item.kind} #MoreHorizIcon`).click()
            await page.locator(`.menu-action-add-sequent_backend_${item.kind}`).click()
            await expect(page).toHaveURL(
                (url) =>
                    url.pathname === `/sequent_backend_${item.kind}/create` &&
                    url.searchParams.get("electionEventId") === CONTENT_IDS.event &&
                    url.searchParams.get(item.kind === "contest" ? "electionId" : "contestId") ===
                        item.parentId
            )
            await expect(page.getByRole("textbox", {name: "Name", exact: true})).toHaveValue("")
        })
    }
})
