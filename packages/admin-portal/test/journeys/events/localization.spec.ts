// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {electionEvent, EVENT_ID, EVENT_ROLES, mockEvent, openEvent, type Row} from "./data"

const LOCALIZATION_ROLES = [
    ...EVENT_ROLES,
    "election-event-data-tab",
    "election-event-localization-selector",
    "localization-read",
    "localization-create",
    "localization-write",
    "localization-delete",
]
test.use({roles: LOCALIZATION_ROLES})

const LANGUAGES = {enabled_language_codes: ["en", "es"], default_language_code: "en"}
const ENGLISH = {
    "name": "Council election",
    "votingPortal:common.welcome": "Welcome, council voters",
    "legacy.footer": "Council footer",
}
const SPANISH = {"name": "Elección del consejo", "global:common.welcome": "Bienvenida"}

/** An event with scoped overrides that absorbs updates, answering `failure` when given. */
function localizedEvent(portal: AdminPortal, failure?: string) {
    let event = electionEvent({}, {language_conf: LANGUAGES, i18n: {en: ENGLISH, es: SPANISH}})
    mockEvent(portal, () => event)
    portal.graphql.on("update_sequent_backend_election_event", ({variables}) => {
        if (failure) return {errors: [{message: failure}]}
        event = {...event, ...(variables._set as Row)}
        return {
            data: {update_sequent_backend_election_event: {affected_rows: 1, returning: [event]}},
        }
    })
}

function row(page: Page, key: string) {
    return page.getByRole("row").filter({has: page.getByRole("cell", {name: key, exact: true})})
}

// The row actions are unnamed icon buttons, rendered edit first and delete second.
function rowAction(page: Page, key: string, action: "edit" | "delete") {
    return row(page, key)
        .getByRole("button")
        .nth(action === "edit" ? 0 : 1)
}

async function lastUpdate(portal: AdminPortal, count: number) {
    await expect
        .poll(() => portal.graphql.callsTo("update_sequent_backend_election_event").length)
        .toBe(count)
    return portal.graphql.callsTo("update_sequent_backend_election_event")[count - 1].variables
}

function i18nSet(i18n: Row) {
    return {
        _set: {
            presentation: {
                i18n,
                language_conf: LANGUAGES,
            },
        },
        where: {id: {_eq: EVENT_ID}},
    }
}

test("adds, edits and deletes scoped overrides for the selected language", async ({
    page,
    portal,
}) => {
    localizedEvent(portal)
    await openEvent(page, portal, "Localization")
    await expect(row(page, "common.welcome")).toContainText("Voting portal")
    await expect(row(page, "common.welcome")).toContainText("Welcome, council voters")
    await expect(row(page, "legacy.footer")).toContainText("Legacy (Voting portal)")

    await page.getByRole("button", {name: "Add", exact: true}).click()
    const create = page
        .getByRole("presentation")
        .filter({has: page.getByRole("button", {name: "Save"})})
    await create.getByRole("combobox").click()
    await page.getByRole("option", {name: "Global", exact: true}).click()
    await create.getByRole("textbox", {name: "Key"}).fill("common.goodbye")
    await create.getByRole("textbox", {name: "Value"}).fill("See you at the polls")
    await create.getByRole("button", {name: "Save", exact: true}).click()
    expect(await lastUpdate(portal, 1)).toEqual(
        i18nSet({
            en: {...ENGLISH, "global:common.goodbye": "See you at the polls"},
            es: SPANISH,
        })
    )
    await expect(page.getByText("Localization updated Successfully", {exact: true})).toBeVisible()
    await expect(row(page, "common.goodbye")).toContainText("Global")

    await rowAction(page, "common.welcome", "edit").click()
    const edit = page
        .getByRole("presentation")
        .filter({has: page.getByRole("button", {name: "Save"})})
    await expect(edit.getByRole("textbox", {name: "Value"})).toHaveValue("Welcome, council voters")
    await edit.getByRole("textbox", {name: "Value"}).fill("Welcome to the council vote")
    await edit.getByRole("button", {name: "Save", exact: true}).click()
    expect(await lastUpdate(portal, 2)).toEqual(
        i18nSet({
            en: {
                ...ENGLISH,
                "votingPortal:common.welcome": "Welcome to the council vote",
                "global:common.goodbye": "See you at the polls",
            },
            es: SPANISH,
        })
    )

    await rowAction(page, "legacy.footer", "delete").click()
    const confirm = page.getByRole("dialog", {name: "Warning"})
    await confirm.getByRole("button", {name: "Delete", exact: true}).click()
    expect(await lastUpdate(portal, 3)).toEqual(
        i18nSet({
            en: {
                "name": "Council election",
                "votingPortal:common.welcome": "Welcome to the council vote",
                "global:common.goodbye": "See you at the polls",
            },
            es: SPANISH,
        })
    )
    await expect(row(page, "legacy.footer")).toHaveCount(0)

    await page.getByRole("combobox", {name: "Select Language"}).click()
    await page.getByRole("option", {name: "Spanish", exact: true}).click()
    await expect(row(page, "common.welcome")).toContainText("Bienvenida")
    await expect(row(page, "common.welcome")).toContainText("Global")
})

test("refuses duplicate keys and invalid date formats without updating", async ({page, portal}) => {
    localizedEvent(portal)
    await openEvent(page, portal, "Localization")
    const add = async (scope: string, key: string, value: string) => {
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page
            .getByRole("presentation")
            .filter({has: page.getByRole("button", {name: "Save"})})
        await drawer.getByRole("combobox").click()
        await page.getByRole("option", {name: scope, exact: true}).click()
        await drawer.getByRole("textbox", {name: "Key"}).fill(key)
        await drawer.getByRole("textbox", {name: "Value"}).fill(value)
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
    }
    await add("Voting portal", "common.welcome", "Another welcome")
    await expect(
        page.getByText("An override with this key and portal scope already exists.", {
            exact: true,
        })
    ).toBeVisible()
    await page.keyboard.press("Escape")
    await add("Voting portal", "votingPortalDateTimeFormat", "every other tuesday")
    await expect(
        page.getByText(
            "Invalid date/time format. Use tokens yyyy, MM, dd, HH, mm, ss (e.g. dd/MM/yyyy HH:mm).",
            {exact: true}
        )
    ).toBeVisible()
    expect(portal.graphql.callsTo("update_sequent_backend_election_event")).toHaveLength(0)
})

test("reports a failed override update", async ({page, portal}) => {
    localizedEvent(portal, "permission denied")
    await openEvent(page, portal, "Localization")
    await rowAction(page, "legacy.footer", "delete").click()
    await page
        .getByRole("dialog", {name: "Warning"})
        .getByRole("button", {name: "Delete", exact: true})
        .click()
    await expect(page.getByText("Localization update failed", {exact: true})).toBeVisible()
    await expect(row(page, "legacy.footer")).toBeVisible()
})

test.describe("read-only localization", () => {
    test.use({roles: [...EVENT_ROLES, "election-event-data-tab", "localization-read"]})

    test("lists overrides without add, edit, delete or language controls", async ({
        page,
        portal,
    }) => {
        localizedEvent(portal)
        await openEvent(page, portal, "Localization")
        await expect(row(page, "common.welcome")).toBeVisible()
        await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
        await expect(row(page, "common.welcome").getByRole("button")).toHaveCount(0)
        await expect(page.getByRole("combobox", {name: "Select Language"})).toHaveCount(0)
    })
})
