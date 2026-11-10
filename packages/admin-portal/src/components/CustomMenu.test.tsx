/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {TenantContext} from "@/providers/TenantContextProvider"
import {CustomMenu} from "./CustomMenu"

jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        IconButton: () => null,
        adminTheme: {
            palette: {
                white: "#fff",
                brandColor: "#0b6",
                green: {light: "#efe"},
                customGrey: {light: "#eee"},
            },
        },
    }),
    {virtual: true}
)
jest.mock("react-admin", () => {
    const {createElement} = require("react")
    const Menu = ({children, className}: {children: React.ReactNode; className?: string}) =>
        createElement("div", {className}, children)
    Menu.Item = ({primaryText, to}: {primaryText: React.ReactNode; to: string}) =>
        createElement("a", {href: to}, primaryText)
    return {Menu, useSidebarState: () => [true, jest.fn()]}
})
jest.mock("./menu/items/SelectTenants", () => ({__esModule: true, default: () => null}))
jest.mock("./menu/items/ElectionEvents", () => ({
    __esModule: true,
    default: () => null,
    TREE_RESOURCE_NAMES: [],
}))
jest.mock("@/providers/TenantContextProvider", () => ({
    TenantContext: require("react").createContext({}),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({tenantId: "tenant", isAuthorized: () => false}),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {PUBLIC_BUCKET_URL: "https://bucket.example.test/"},
    }),
}))
jest.mock("react-router-dom", () => ({
    useNavigate: () => jest.fn(),
    useLocation: () => ({pathname: "/"}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => (key === "sideMenu.help" ? "Help" : key),
        i18n: {language: "en"},
    }),
}))

const tenant = (configuration_signing?: unknown) => ({
    id: "tenant",
    settings: {
        help_links: [{url: "${PUBLIC_BUCKET_URL}guides/admin.pdf", title: "Administrator guide"}],
        ...(configuration_signing ? {configuration_signing} : {}),
    },
})
const renderFor = (value: unknown) =>
    render(
        <TenantContext.Provider value={{tenant: value} as never}>
            <CustomMenu />
        </TenantContext.Provider>
    )

it("offers the tenant's help links where signed configuration packages are optional", () => {
    const open = jest.spyOn(window, "open").mockReturnValue(null)
    renderFor(tenant({policy: "optional"}))
    fireEvent.click(screen.getByRole("button", {name: "Help"}))
    fireEvent.click(screen.getByRole("menuitem", {name: "Administrator guide"}))
    expect(open).toHaveBeenCalledWith(
        "https://bucket.example.test/guides/admin.pdf",
        "_blank",
        "noopener,noreferrer"
    )
    open.mockRestore()
})

it.each([
    ["requires signed packages", tenant({policy: "required"})],
    ["has no signing setting", tenant()],
    ["has not loaded", undefined],
])("offers no help links for a tenant that %s", (_, value) => {
    renderFor(value)
    expect(screen.queryByRole("button", {name: "Help"})).not.toBeInTheDocument()
    expect(screen.queryByText("Administrator guide")).not.toBeInTheDocument()
})
