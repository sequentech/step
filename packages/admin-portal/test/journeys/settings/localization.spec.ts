// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect, type AdminPortal} from "../fixtures"
import {
    SETTINGS_ROLES,
    TENANT_WHERE,
    mockElectionTypes,
    mockTenant,
    openSettings,
    rowWith,
    tenantRow,
} from "./data"

const LANGUAGE_CONF = {enabled_language_codes: ["en", "es"], default_language_code: "en"}
const ENGLISH = {"adminPortal:sideMenu.help": "Support", "legacy.key": "Legacy value"}
const SPANISH = {"global:hello": "Hola"}

test.use({roles: SETTINGS_ROLES})
test.beforeEach(({portal}) => {
    mockElectionTypes(portal)
})

function mockOverrides(portal: AdminPortal) {
    return mockTenant(
        portal,
        tenantRow({settings: {language_conf: LANGUAGE_CONF, i18n: {en: ENGLISH, es: SPANISH}}})
    )
}

function savedOverrides(english: Record<string, string>) {
    return {
        _set: {settings: {language_conf: LANGUAGE_CONF, i18n: {en: english, es: SPANISH}}},
        where: TENANT_WHERE,
    }
}

test("lists the tenant's overrides per language with their portal scope", async ({
    page,
    portal,
}) => {
    mockOverrides(portal)
    await openSettings(page, portal, "LOCALIZATION")
    await expect(rowWith(page, "sideMenu.help")).toContainText("Admin portal")
    await expect(rowWith(page, "sideMenu.help")).toContainText("Support")
    await expect(rowWith(page, "legacy.key")).toContainText("Legacy (Admin portal)")
    await expect(rowWith(page, "legacy.key")).toContainText("Legacy value")

    await page.getByRole("combobox", {name: "Select Language"}).click()
    await expect(page.getByRole("option")).toHaveText(["English", "Spanish"])
    await page.getByRole("option", {name: "Spanish", exact: true}).click()
    await expect(rowWith(page, "hello")).toContainText("Global")
    await expect(rowWith(page, "hello")).toContainText("Hola")
    await expect(rowWith(page, "sideMenu.help")).toHaveCount(0)
})

test("adds an admin portal override that the portal applies at once", async ({page, portal}) => {
    const tenant = mockOverrides(portal)
    await openSettings(page, portal, "LOCALIZATION")
    await expect(page.getByText("General Configuration", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Add", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await expect(drawer.getByText("Localization configuration", {exact: true})).toBeVisible()
    await expect(drawer.getByRole("combobox", {name: "Portal scope"})).toHaveText("Admin portal")
    await drawer
        .getByRole("textbox", {name: "Key", exact: true})
        .fill("electionTypeScreen.common.settingSubtitle")
    await drawer.getByRole("textbox", {name: "Value", exact: true}).fill("Tenant configuration")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()

    await expect(page.getByText("Localization updated Successfully", {exact: true})).toBeVisible()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(tenant.updates()).toEqual([
        savedOverrides({
            ...ENGLISH,
            "adminPortal:electionTypeScreen.common.settingSubtitle": "Tenant configuration",
        }),
    ])
    await expect(rowWith(page, "electionTypeScreen.common.settingSubtitle")).toContainText(
        "Tenant configuration"
    )
    await expect(page.getByText("Tenant configuration", {exact: true}).first()).toBeVisible()
    await expect(page.getByText("General Configuration", {exact: true})).toHaveCount(0)
})

test("moves a legacy override to the global scope when edited", async ({page, portal}) => {
    const tenant = mockOverrides(portal)
    await openSettings(page, portal, "LOCALIZATION")
    await rowWith(page, "legacy.key").getByRole("button").first().click()
    const drawer = page.getByRole("dialog")
    const key = drawer.getByRole("textbox", {name: "Key", exact: true})
    await expect(key).toHaveValue("legacy.key")
    await expect(key).not.toBeEditable()
    await expect(drawer.getByRole("textbox", {name: "Value", exact: true})).toHaveValue(
        "Legacy value"
    )
    await drawer.getByRole("combobox", {name: "Portal scope"}).click()
    await expect(page.getByRole("option")).toHaveText(["Global", "Admin portal"])
    await page.getByRole("option", {name: "Global", exact: true}).click()
    await drawer.getByRole("textbox", {name: "Value", exact: true}).fill("Global value")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()

    await expect(page.getByText("Localization updated Successfully", {exact: true})).toBeVisible()
    expect(tenant.updates()).toEqual([
        savedOverrides({
            "adminPortal:sideMenu.help": "Support",
            "global:legacy.key": "Global value",
        }),
    ])
    await expect(rowWith(page, "legacy.key")).toContainText("Global")
    await expect(rowWith(page, "legacy.key")).toContainText("Global value")
})

test("deletes an override only after the warning is confirmed", async ({page, portal}) => {
    const tenant = mockOverrides(portal)
    await openSettings(page, portal, "LOCALIZATION")
    const row = rowWith(page, "sideMenu.help")
    await row.getByRole("button").last().click()
    await page.getByRole("dialog").getByRole("button", {name: "Cancel", exact: true}).click()
    expect(tenant.updates()).toEqual([])

    await row.getByRole("button").last().click()
    await expect(page.getByText("Are you sure you want to delete this item?")).toBeVisible()
    await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
    await expect(page.getByText("Localization updated Successfully", {exact: true})).toBeVisible()
    await expect(row).toHaveCount(0)
    expect(tenant.updates()).toEqual([savedOverrides({"legacy.key": "Legacy value"})])
})

test("refuses a second override for the same key and scope", async ({page, portal}) => {
    const tenant = mockOverrides(portal)
    await openSettings(page, portal, "LOCALIZATION")
    await page.getByRole("button", {name: "Add", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Key", exact: true}).fill("sideMenu.help")
    await drawer.getByRole("textbox", {name: "Value", exact: true}).fill("Assistance")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect(
        page.getByText("An override with this key and portal scope already exists.", {exact: true})
    ).toBeVisible()
    await expect(drawer).toBeVisible()
    expect(tenant.updates()).toEqual([])
})

test("reports a failed save and keeps the overrides unchanged", async ({page, portal}) => {
    mockOverrides(portal)
    portal.graphql.on("update_sequent_backend_tenant", () => ({
        errors: [{message: "permission denied"}],
    }))
    await openSettings(page, portal, "LOCALIZATION")
    await page.getByRole("button", {name: "Add", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Key", exact: true}).fill("sideMenu.logout")
    await drawer.getByRole("textbox", {name: "Value", exact: true}).fill("Sign out")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect(page.getByText("Localization update failed", {exact: true})).toBeVisible()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(portal.graphql.callsTo("update_sequent_backend_tenant")).toHaveLength(1)
    await expect(rowWith(page, "sideMenu.logout")).toHaveCount(0)
})

test("ignores an override without a value", async ({page, portal}) => {
    const tenant = mockOverrides(portal)
    await openSettings(page, portal, "LOCALIZATION")
    await page.getByRole("button", {name: "Add", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Key", exact: true}).fill("sideMenu.logout")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect(drawer.getByRole("textbox", {name: "Key", exact: true})).toHaveValue(
        "sideMenu.logout"
    )
    expect(tenant.updates()).toEqual([])
    await expect(rowWith(page, "sideMenu.logout")).toHaveCount(0)
})

test("refuses to overwrite another scope's override", async ({page, portal}) => {
    const tenant = mockTenant(
        portal,
        tenantRow({
            settings: {
                language_conf: LANGUAGE_CONF,
                i18n: {en: {...ENGLISH, "global:sideMenu.help": "Help desk"}, es: SPANISH},
            },
        })
    )
    await openSettings(page, portal, "LOCALIZATION")
    const drawer = page.getByRole("dialog")
    await page.getByRole("row").filter({hasText: "Help desk"}).getByRole("button").first().click()
    await drawer.getByRole("combobox", {name: "Portal scope"}).click()
    await page.getByRole("option", {name: "Admin portal", exact: true}).click()
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect(
        page.getByText("An override with this key and portal scope already exists.", {exact: true})
    ).toBeVisible()
    expect(tenant.updates()).toEqual([])
})
