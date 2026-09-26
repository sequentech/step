// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../fixtures"
import {
    LANGUAGE_CONF,
    SETTINGS_ROLES,
    TENANT_WHERE,
    commitUndoable,
    mockElectionTypes,
    mockTenant,
    openSettings,
    switchFor,
    tenantRow,
} from "./data"

test.use({roles: SETTINGS_ROLES})
test.beforeEach(({portal}) => {
    mockElectionTypes(portal)
})

test("saves a toggled voting channel once the undo window closes", async ({page, portal}) => {
    const tenant = mockTenant(
        portal,
        tenantRow({voting_channels: {online: true, kiosk: false, telephone: true}})
    )
    await openSettings(page, portal, "VOTING CHANELS")
    await expect(switchFor(page, "Online Voting")).toBeChecked()
    await expect(switchFor(page, "Telephone Voting")).toBeChecked()
    const kiosk = switchFor(page, "Kiosk Voting")
    await expect(kiosk).not.toBeChecked()

    await kiosk.click()
    await expect(page.getByText("Element updated", {exact: true})).toBeVisible()
    expect(tenant.updates()).toEqual([])
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    expect(tenant.updates()).toEqual([
        {
            _set: {voting_channels: {online: true, kiosk: true, telephone: true}},
            where: TENANT_WHERE,
        },
    ])
    await expect(kiosk).toBeChecked()
})

test("undoing a voting channel change restores the switch and sends nothing", async ({
    page,
    portal,
}) => {
    const tenant = mockTenant(portal)
    await openSettings(page, portal, "VOTING CHANELS")
    const telephone = switchFor(page, "Telephone Voting")
    await telephone.click()
    await expect(telephone).toBeChecked()
    await page.getByRole("button", {name: "Undo", exact: true}).click()
    await expect(telephone).not.toBeChecked()
    await page.clock.runFor(10_000)
    expect(tenant.updates()).toEqual([])
})

test("shows online voting as off when the tenant disabled it", async ({page, portal}) => {
    // Defect: SettingsVotingChannel reads `voting_channels.online || true`, so a stored false renders as on.
    mockTenant(portal, tenantRow({voting_channels: {online: false, kiosk: true, telephone: false}}))
    await openSettings(page, portal, "VOTING CHANELS")
    await expect(switchFor(page, "Kiosk Voting")).toBeChecked()
    test.fail(true, "The online voting switch coerces the stored false value to true")
    await expect(switchFor(page, "Online Voting")).not.toBeChecked({timeout: 2_000})
})

test("shows the mail and SMS template channels as read-only switches", async ({page, portal}) => {
    mockTenant(portal, tenantRow({settings: {language_conf: LANGUAGE_CONF, mail: true, sms: true}}))
    await openSettings(page, portal, "TEMPLATES")
    for (const label of ["Mails", "SMS"]) {
        await expect(switchFor(page, label)).toBeChecked()
        await expect(switchFor(page, label)).toBeDisabled()
    }
})

test("enables a language, makes it the default and forces it on voters", async ({page, portal}) => {
    const tenant = mockTenant(portal)
    await openSettings(page, portal, "LANGUAGES")
    await expect(
        page.getByText(
            "Enable languages in the system. Only languages enabled here will be available for election events.",
            {exact: true}
        )
    ).toBeVisible()
    await expect(switchFor(page, "English")).toBeChecked()
    const spanish = switchFor(page, "Español")
    await expect(spanish).not.toBeChecked()
    await spanish.click()
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    await expect(spanish).toBeChecked()

    const defaultLanguage = page.getByRole("combobox", {name: "Default Language"})
    await expect(defaultLanguage).toHaveText("English")
    await defaultLanguage.click()
    await expect(page.getByRole("option")).toHaveText(["English", "Español"])
    await page.getByRole("option", {name: "Español", exact: true}).click()
    await commitUndoable(page, portal, "update_sequent_backend_tenant", 2)

    const policy = page.getByRole("combobox", {name: "Language Detection Policy"})
    await expect(policy).toHaveText("Browser Detect")
    await policy.click()
    await page.getByRole("option", {name: "Force Default", exact: true}).click()
    await commitUndoable(page, portal, "update_sequent_backend_tenant", 3)
    await expect(policy).toHaveText("Force Default")

    const languageConf = {enabled_language_codes: ["en", "es"], default_language_code: "en"}
    expect(tenant.updates()).toEqual([
        {_set: {settings: {language_conf: languageConf}}, where: TENANT_WHERE},
        {
            _set: {settings: {language_conf: {...languageConf, default_language_code: "es"}}},
            where: TENANT_WHERE,
        },
        {
            _set: {
                settings: {
                    language_conf: {
                        ...languageConf,
                        default_language_code: "es",
                        language_detection_policy: "force-default",
                    },
                },
            },
            where: TENANT_WHERE,
        },
    ])
})

test("disabling a language removes it from the enabled codes", async ({page, portal}) => {
    const tenant = mockTenant(
        portal,
        tenantRow({
            settings: {
                language_conf: {enabled_language_codes: ["en", "fr"], default_language_code: "en"},
            },
        })
    )
    await openSettings(page, portal, "LANGUAGES")
    const french = switchFor(page, "Français")
    await expect(french).toBeChecked()
    await french.click()
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    await expect(french).not.toBeChecked()
    expect(tenant.updates()).toEqual([
        {
            _set: {
                settings: {
                    language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
                },
            },
            where: TENANT_WHERE,
        },
    ])
})
