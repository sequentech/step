/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {SelectTenant} from "./SelectTenant"

jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        adminTheme: {},
        Header: ({logoUrl}: {logoUrl: string}) =>
            require("react").createElement("img", {alt: "Logo Image", src: logoUrl}),
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-essentials/public/Sequent_logo.svg", () => "sequent-logo.svg", {
    virtual: true,
})
jest.mock("@sequentech/ui-essentials/public/blank_logo.svg", () => "blank-logo.svg", {
    virtual: true,
})
jest.mock("@sequentech/ui-core", () => ({shortAppHash: () => undefined}))
jest.mock("react-admin", () => ({AppBar: () => null}))
jest.mock("react-router-dom", () => ({useNavigate: () => jest.fn()}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthenticated: false, initKeycloak: jest.fn()}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {
            HASURA_URL: "https://hasura.example.test/v1/graphql",
            DEFAULT_TENANT_ID: "default-tenant",
            KEYCLOAK_URL: "https://keycloak.example.test/",
            APP_VERSION: "test",
            APP_HASH: "-",
        },
    }),
}))

const LOGO = "https://logo.example.test/council.svg"
/** The style rules on the page, as the browser holds them. */
const styles = () =>
    Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules).map((rule) => rule.cssText))
        .join("\n")
const showWith = async (row: Record<string, unknown>, logo: string) => {
    global.fetch = jest.fn().mockResolvedValue({
        json: async () => ({data: {sequent_backend_tenant_for_sign_in: [row]}}),
    }) as never
    render(<SelectTenant />)
    // The page has read the sign-in row once it shows a logo in place of the blank one.
    await waitFor(() =>
        expect(screen.getByRole("img", {name: "Logo Image"})).toHaveAttribute("src", logo)
    )
    expect(global.fetch).toHaveBeenCalledWith(
        "https://hasura.example.test/v1/graphql",
        expect.objectContaining({method: "POST"})
    )
}
const annotations = {logo_url: LOGO, css: "h1 { letter-spacing: 3px; }"}

it("shows the built-in logo and not the tenant's CSS before sign-in, where its signing policy is unknown", async () => {
    await showWith({id: "default-tenant", annotations}, "sequent-logo.svg")
    expect(styles()).not.toMatch(/letter-spacing:\s*3px/)
})

it("applies the logo and CSS of a sign-in row whose signed configuration packages are optional", async () => {
    await showWith(
        {
            id: "default-tenant",
            annotations,
            settings: {configuration_signing: {policy: "optional"}},
        },
        LOGO
    )
    await waitFor(() => expect(styles()).toMatch(/letter-spacing:\s*3px/))
})
