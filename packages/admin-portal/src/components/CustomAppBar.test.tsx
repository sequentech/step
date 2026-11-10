/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {TenantContext} from "@/providers/TenantContextProvider"
import {CustomAppBar} from "./CustomAppBar"

let mockTenant: Record<string, unknown> | undefined
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        adminTheme: {palette: {lightBackground: "#fff"}},
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
jest.mock("@sequentech/ui-core", () => ({
    getValueFromCookie: () => undefined,
    setCookie: jest.fn(),
    shortAppHash: () => undefined,
    USER_LANGUAGE_COOKIE_NAME: "language",
}))
jest.mock("react-admin", () => ({
    AppBar: ({children}: {children: React.ReactNode}) =>
        require("react").createElement("header", null, children),
    useGetOne: () => ({data: mockTenant}),
}))
jest.mock("react-router-dom", () => ({useMatch: () => null}))
jest.mock("@/resources/ElectionEvent/Signatures/WaitingForMySignature", () => ({
    WaitingForMySignature: () => null,
}))
jest.mock("@/providers/TenantContextProvider", () => ({
    TenantContext: require("react").createContext({}),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthenticated: false}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {APP_VERSION: "test", APP_HASH: "-"},
    }),
}))

const LOGO = "https://logo.example.test/council.svg"
const tenant = (configuration_signing?: unknown, logo_url: string | null = LOGO) => ({
    id: "tenant",
    annotations: {logo_url},
    settings: configuration_signing ? {configuration_signing} : {},
})
/** The app bar under a tenant context that keeps the tenant it is given, as the portal's does. */
const Bar = () => {
    const [current, setTenant] = useState<unknown>(undefined)
    return (
        <TenantContext.Provider value={{tenantId: "tenant", tenant: current, setTenant} as never}>
            <CustomAppBar />
        </TenantContext.Provider>
    )
}
const logo = () => screen.getByRole("img", {name: "Logo Image"})

it("shows the logo of a tenant whose signed configuration packages are optional", async () => {
    mockTenant = tenant({policy: "optional"})
    render(<Bar />)
    await waitFor(() => expect(logo()).toHaveAttribute("src", LOGO))
})

it.each([
    ["requires signed packages", tenant({policy: "required"})],
    ["has no signing setting", tenant()],
    ["has optional packages and no logo", tenant({policy: "optional"}, null)],
])("shows the built-in logo for a tenant that %s", async (_, value) => {
    mockTenant = value
    render(<Bar />)
    await waitFor(() => expect(logo()).toHaveAttribute("src", "sequent-logo.svg"))
})

it("shows a blank logo until the tenant has loaded", () => {
    mockTenant = undefined
    render(<Bar />)
    expect(logo()).toHaveAttribute("src", "blank-logo.svg")
})
