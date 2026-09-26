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
    tenantRow,
} from "./data"

// A 1x1 transparent PNG.
const PNG = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
    "base64"
)
const HELP_LINKS = [{url: "https://help.synthetic.example", title: "Support portal"}]

test.use({roles: SETTINGS_ROLES})
test.beforeEach(({portal}) => {
    mockElectionTypes(portal)
})

test("saves the Google Calendar integration key and email", async ({page, portal}) => {
    const tenant = mockTenant(portal)
    await openSettings(page, portal, "Integrations")
    const save = page.getByRole("button", {name: "Save", exact: true})
    await expect(save).toBeDisabled()
    await page
        .getByRole("textbox", {name: "Google Calendar Service Account Key"})
        .fill('{"type": "service_account", "client_email": "calendar@synthetic.example"}')
    await page
        .getByRole("textbox", {name: "Google Calendar Authentication Email"})
        .fill("admin@synthetic.example")
    await expect(save).toBeEnabled()
    await save.click()
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    await expect(save).toBeDisabled()
    expect(tenant.updates()).toEqual([
        {
            _set: {
                settings: {
                    language_conf: LANGUAGE_CONF,
                    gapi_key: {type: "service_account", client_email: "calendar@synthetic.example"},
                    gapi_email: "admin@synthetic.example",
                },
            },
            where: TENANT_WHERE,
        },
    ])
})

test("clears a stored integration key and email", async ({page, portal}) => {
    const tenant = mockTenant(
        portal,
        tenantRow({
            settings: {
                language_conf: LANGUAGE_CONF,
                gapi_key: {type: "service_account"},
                gapi_email: "old@synthetic.example",
            },
        })
    )
    await openSettings(page, portal, "Integrations")
    const key = page.getByRole("textbox", {name: "Google Calendar Service Account Key"})
    await key.fill("{}")
    await key.fill("")
    const email = page.getByRole("textbox", {name: "Google Calendar Authentication Email"})
    await email.fill("x")
    await email.fill("")
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    expect(tenant.updates()).toEqual([
        {_set: {settings: {language_conf: LANGUAGE_CONF, gapi_key: null}}, where: TENANT_WHERE},
    ])
})

test("rejects an integration key that is not a JSON object", async ({page, portal}) => {
    const tenant = mockTenant(portal)
    await openSettings(page, portal, "Integrations")
    await page.getByRole("textbox", {name: "Google Calendar Service Account Key"}).fill("42")
    await expect(
        page.getByText("Invalid Google Calendar Service Account Key format", {exact: true})
    ).toBeVisible()
    await expect(page.getByRole("button", {name: "Save", exact: true})).toBeDisabled()
    await page.getByRole("textbox", {name: "Google Calendar Service Account Key"}).fill("{")
    await expect(
        page.getByText("Invalid Google Calendar Service Account Key format", {exact: true})
    ).toBeVisible()
    expect(tenant.updates()).toEqual([])
})

test("saves the logo, custom CSS and help links and applies them to the portal", async ({
    page,
    portal,
}) => {
    const tenant = mockTenant(portal)
    const logoUrl = portal.s3.putBytes("public", "branding/logo.png", PNG, "image/png")
    await openSettings(page, portal, "Look & Feel")
    await expect(page.getByRole("button", {name: "Help", exact: true})).toHaveCount(0)
    const logo = page.getByRole("textbox", {name: "Logo URL"})
    await logo.fill(logoUrl)
    await logo.blur()
    const css = page.getByRole("textbox", {name: "Custom CSS"})
    await css.fill(".settings-box { outline: 1px solid; }")
    await css.blur()
    const helpLinks = page.getByRole("textbox", {name: "Help Links"})
    await expect(helpLinks).toHaveValue("[]")
    await helpLinks.fill(JSON.stringify(HELP_LINKS))
    await helpLinks.blur()
    const save = page.getByRole("button", {name: "Save", exact: true})
    await save.click()
    await expect(save).toBeDisabled()
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    expect(tenant.updates()).toEqual([
        {
            _set: {
                annotations: {logo_url: logoUrl, css: ".settings-box { outline: 1px solid; }"},
                settings: {language_conf: LANGUAGE_CONF, help_links: HELP_LINKS},
            },
            where: TENANT_WHERE,
        },
    ])
    await expect(page.getByRole("img", {name: "Logo Image"})).toHaveAttribute("src", logoUrl)
    await page.getByRole("button", {name: "Help", exact: true}).click()
    await expect(page.getByRole("menuitem", {name: "Support portal", exact: true})).toBeVisible()
})

test("removes the logo and CSS when their fields are emptied", async ({page, portal}) => {
    const tenant = mockTenant(
        portal,
        tenantRow({annotations: {logo_url: null, css: "body { margin: 0; }"}})
    )
    await openSettings(page, portal, "Look & Feel")
    const css = page.getByRole("textbox", {name: "Custom CSS"})
    await expect(css).toHaveValue("body { margin: 0; }")
    await css.fill("")
    await css.blur()
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    expect(tenant.updates()).toEqual([
        {
            _set: {
                annotations: {logo_url: null, css: null},
                settings: {language_conf: LANGUAGE_CONF, help_links: []},
            },
            where: TENANT_WHERE,
        },
    ])
})

test("rejects help links that are not a JSON list", async ({page, portal}) => {
    const tenant = mockTenant(portal)
    await openSettings(page, portal, "Look & Feel")
    const helpLinks = page.getByRole("textbox", {name: "Help Links"})
    for (const invalid of ['{"url": "https://help.synthetic.example"}', "[{"]) {
        await helpLinks.fill(invalid)
        await helpLinks.blur()
        await expect(page.getByText("Invalid Help Links format", {exact: true})).toBeVisible()
        await page.clock.runFor(5_000)
        await expect(page.getByText("Invalid Help Links format", {exact: true})).toHaveCount(0)
    }
    expect(tenant.updates()).toEqual([])
})

test("blocks voting and enrollment from the chosen countries", async ({page, portal}) => {
    const tenant = mockTenant(
        portal,
        tenantRow({settings: {language_conf: LANGUAGE_CONF, voting_countries: ["FR"]}})
    )
    portal.graphql.on("limitAccessByCountries", () => ({
        data: {limit_access_by_countries: {success: true}},
    }))
    await openSettings(page, portal, "Countries")
    await expect(page.getByText("Country Blocking", {exact: true})).toBeVisible()
    await expect(page.getByRole("button", {name: "France", exact: true})).toBeVisible()
    const [voting, enrollment] = [0, 1].map((index) =>
        page.getByRole("combobox", {name: "Countries"}).nth(index)
    )
    await voting.fill("Spai")
    await page.getByRole("option", {name: "Spain", exact: true}).click()
    await enrollment.fill("Portug")
    await page.getByRole("option", {name: "Portugal", exact: true}).click()
    await page.getByRole("button", {name: "Save", exact: true}).click()

    await expect.poll(() => portal.graphql.callsTo("limitAccessByCountries").length).toBe(1)
    expect(portal.graphql.callsTo("limitAccessByCountries")[0].variables).toEqual({
        votingCountries: ["FR", "ES"],
        enrollCountries: ["PT"],
    })
    await commitUndoable(page, portal, "update_sequent_backend_tenant")
    expect(tenant.updates()).toEqual([
        {
            _set: {
                settings: {
                    language_conf: LANGUAGE_CONF,
                    voting_countries: ["FR", "ES"],
                    enroll_countries: ["PT"],
                },
            },
            where: TENANT_WHERE,
        },
    ])
})

test("tells the user when the country list cannot be saved", async ({page, portal}) => {
    // Defect: Apollo rejects on GraphQL errors, so SettingsCountries never reaches its error notification.
    // Keeps the pinned rejection from also failing the fixture's page error check.
    await page.addInitScript(() =>
        window.addEventListener("unhandledrejection", (event) => {
            if (String(event.reason?.message) === "Keycloak unavailable") event.preventDefault()
        })
    )
    const tenant = mockTenant(portal)
    portal.graphql.on("limitAccessByCountries", () => ({
        errors: [{message: "Keycloak unavailable"}],
    }))
    await openSettings(page, portal, "Countries")
    await page.getByRole("combobox", {name: "Countries"}).first().fill("Spai")
    await page.getByRole("option", {name: "Spain", exact: true}).click()
    await page.getByRole("button", {name: "Save", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("limitAccessByCountries")).toHaveLength(1)
    await page.clock.runFor(10_000)
    expect(tenant.updates()).toEqual([])
    test.fail(true, "A failed country restriction mutation never reaches the error notification")
    await expect(page.getByText("Error saving the country list", {exact: true})).toBeVisible({
        timeout: 2_000,
    })
})
