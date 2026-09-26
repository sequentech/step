// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect, TENANT_ID} from "../fixtures"
import {
    ELECTION_TYPE_ID,
    SETTINGS_ROLES,
    electionType,
    mockElectionTypes,
    mockTenant,
    openSettings,
    rowWith,
} from "./data"

const REFERENDUM_ID = "20000000-0000-4000-8000-000000000003"

test.use({roles: SETTINGS_ROLES})

test("lists election types and renames one from the edit drawer", async ({page, portal}) => {
    mockTenant(portal)
    mockElectionTypes(portal, [
        electionType(),
        electionType({id: REFERENDUM_ID, name: "Referendum"}),
    ])
    await openSettings(page, portal)
    await expect(rowWith(page, "Referendum")).toBeVisible()
    const row = rowWith(page, "General election")
    // The row actions are icon buttons without accessible names: edit first, delete last.
    await row.getByRole("button").first().click()
    const drawer = page.getByRole("dialog")
    await expect(drawer.getByText("Edit Election Type", {exact: true})).toBeVisible()
    const name = drawer.getByRole("textbox", {name: "Name", exact: true})
    await expect(name).toHaveValue("General election")
    await name.fill("Municipal election")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()

    await expect(rowWith(page, "Municipal election")).toBeVisible()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(
        portal.graphql.callsTo("update_sequent_backend_election_type").map((c) => c.variables)
    ).toEqual([{_set: {name: "Municipal election"}, where: {id: {_eq: ELECTION_TYPE_ID}}}])
    expect(
        portal.graphql.callsTo("sequent_backend_election_type").map((c) => c.variables)
    ).toContainEqual({where: {id: {_eq: ELECTION_TYPE_ID}}, limit: 1})
})

test("deletes an election type only after the warning is confirmed", async ({page, portal}) => {
    mockTenant(portal)
    mockElectionTypes(portal, [
        electionType(),
        electionType({id: REFERENDUM_ID, name: "Referendum"}),
    ])
    await openSettings(page, portal)
    const row = rowWith(page, "Referendum")
    await row.getByRole("button").last().click()
    const warning = page.getByRole("dialog")
    await expect(warning.getByText("Are you sure you want to delete this item?")).toBeVisible()
    await warning.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(portal.graphql.callsTo("delete_sequent_backend_election_type")).toEqual([])

    await row.getByRole("button").last().click()
    await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
    await expect(row).toHaveCount(0)
    await expect(rowWith(page, "General election")).toBeVisible()
    expect(
        portal.graphql.callsTo("delete_sequent_backend_election_type").map((c) => c.variables)
    ).toEqual([{where: {id: {_eq: REFERENDUM_ID}}}])
})

test("creates the first election type from the empty state", async ({page, portal}) => {
    mockTenant(portal)
    mockElectionTypes(portal)
    await openSettings(page, portal)
    await expect(page.getByText("No Election Types yet.", {exact: true})).toBeVisible()
    await expect(page.getByText("Do you want to create one?", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Create Election Type", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("Municipal election")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()

    await expect(rowWith(page, "Municipal election")).toBeVisible()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(
        portal.graphql.callsTo("insert_sequent_backend_election_type").map((c) => c.variables)
    ).toEqual([{objects: {tenant_id: TENANT_ID, name: "Municipal election"}}])
})

test("creates an election type from its own route and returns to settings", async ({
    page,
    portal,
}) => {
    mockTenant(portal)
    mockElectionTypes(portal)
    await page.goto(`${portal.origin}/sequent_backend_election_type/create?lang=en`)
    await expect(page.getByText("Create Election Type", {exact: true})).toBeVisible()
    await page.getByRole("textbox", {name: "Name", exact: true}).fill("Regional election")
    await page.getByRole("button", {name: "Save", exact: true}).click()

    await expect(page).toHaveURL(/\/settings$/)
    await expect(rowWith(page, "Regional election")).toBeVisible()
    expect(
        portal.graphql.callsTo("insert_sequent_backend_election_type").map((c) => c.variables)
    ).toEqual([{objects: {tenant_id: TENANT_ID, name: "Regional election"}}])
})

test("offers a way to add another election type once some exist", async ({page, portal}) => {
    // Defect: SettingsElectionsTypes passes the create drawer to ListActions without `withComponent`,
    // so once the list has rows there is no create button at all.
    mockTenant(portal)
    mockElectionTypes(portal, [electionType()])
    await openSettings(page, portal)
    await expect(rowWith(page, "General election")).toBeVisible()
    test.fail(true, "A populated election-type list has no create action")
    await expect(
        page.getByRole("button", {name: /^(Add|Create Election Type)$/}).first()
    ).toBeVisible({timeout: 2_000})
})

test("tells the user when renaming an election type fails", async ({page, portal}) => {
    // Defect: the edit drawer's onError only refreshes and closes, so the failure is silent.
    mockTenant(portal)
    mockElectionTypes(portal, [electionType()])
    portal.graphql.on("update_sequent_backend_election_type", () => ({
        errors: [{message: "Uniqueness violation on election type name"}],
    }))
    await openSettings(page, portal)
    await rowWith(page, "General election").getByRole("button").first().click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("Referendum")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_election_type"))
        .toHaveLength(1)
    test.fail(true, "Election-type rename failures close the drawer without an error notification")
    await expect(page.getByText("Uniqueness violation on election type name")).toBeVisible({
        timeout: 2_000,
    })
})
