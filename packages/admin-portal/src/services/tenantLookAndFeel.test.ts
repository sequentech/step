// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    adminPortalCustomCss,
    adminPortalHelpLinks,
    adminPortalLogoUrl,
    staffScreensCustomizable,
} from "./tenantLookAndFeel"

const LOGO = "https://logo.example.test/logo.svg"
const links = [{url: "https://help.example.test/start", title: "Getting started"}]
const tenant = (configuration_signing?: unknown) => ({
    annotations: {logo_url: LOGO, css: "body {color: red}"},
    settings: {help_links: links, ...(configuration_signing ? {configuration_signing} : {})},
})

it("applies the logo, custom CSS and help links of a tenant whose signed packages are optional", () => {
    const optional = tenant({policy: "optional"})
    expect(staffScreensCustomizable(optional)).toBe(true)
    expect(adminPortalLogoUrl(optional)).toBe(LOGO)
    expect(adminPortalCustomCss(optional)).toBe("body {color: red}")
    expect(adminPortalHelpLinks(optional)).toEqual(links)
})

it.each([
    ["requires signed packages", tenant({policy: "required"})],
    ["has no signing setting", tenant()],
    ["has an unknown policy", tenant({policy: "sometimes"})],
    ["has not loaded", undefined],
])("keeps the staff screens as built for a tenant that %s", (_, value) => {
    expect(staffScreensCustomizable(value)).toBe(false)
    expect(adminPortalLogoUrl(value)).toBeUndefined()
    expect(adminPortalCustomCss(value)).toBe("")
    expect(adminPortalHelpLinks(value)).toEqual([])
})

it("reads missing or malformed values of an optional tenant as none", () => {
    const optional = {settings: {configuration_signing: {policy: "optional"}}}
    expect(adminPortalLogoUrl(optional)).toBeUndefined()
    expect(adminPortalCustomCss(optional)).toBe("")
    expect(adminPortalHelpLinks(optional)).toEqual([])
    const malformed = {
        annotations: {logo_url: 7, css: 7},
        settings: {...optional.settings, help_links: {}},
    }
    expect(adminPortalLogoUrl(malformed)).toBeUndefined()
    expect(adminPortalCustomCss(malformed)).toBe("")
    expect(adminPortalHelpLinks(malformed)).toEqual([])
    expect(adminPortalLogoUrl({...optional, annotations: {logo_url: ""}})).toBeUndefined()
    expect(adminPortalLogoUrl({...optional, annotations: {logo_url: null}})).toBeUndefined()
})
