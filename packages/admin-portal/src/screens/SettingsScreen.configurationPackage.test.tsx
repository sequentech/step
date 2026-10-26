/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The configuration package tab: shown only with every permission the
// platform's route requires, and last, so no other tab's tabIndex link moves.
import React from "react"
import {render} from "@testing-library/react"

let mockGranted: string[] = []
const mockTabs = jest.fn()

jest.mock("react-admin", () => ({Resource: () => null, useSidebarState: () => [true]}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["t1"]}))
jest.mock("@/providers/AuthContextProvider", () => {
    const React = require("react")
    return {
        AuthContext: React.createContext({
            isAuthorized: (_super: boolean, _tenant: string, permission: string) =>
                mockGranted.includes(permission),
        }),
    }
})
jest.mock("@/components/Tabs", () => ({
    Tabs: ({elements}: {elements: Array<{label: string}>}) => {
        mockTabs(elements.map((element) => element.label))
        return null
    },
}))
jest.mock("@/components/HeaderTitle", () => ({HeaderTitle: () => null}))
for (const module of [
    "@/resources/Settings/SettingsLanguages",
    "@/resources/Settings/SettingsMessaging",
    "@/resources/Settings/SettingsVotingChannel",
    "@/resources/Settings/SettingsElectionsTypes",
    "@/resources/Settings/SettingsElectionsTypesCreate",
    "@/resources/Settings/SettingsTrustees",
    "@/resources/Settings/SettingsLookAndFeel",
    "@/resources/Settings/SettingsIntegrations",
    "@/resources/Settings/SettingsCountries",
    "@/resources/Settings/SettingsBackupRestore",
    "@/resources/Settings/SettingsConfigurationPackage",
    "@/resources/Settings/SettingsPreviews",
    "@/resources/Settings/SettingsLogs",
    "@/plugins/PluginSettings",
    "@/components/styles/ResourceListStyles",
]) {
    jest.doMock(module, () => new Proxy({}, {get: () => () => null}))
}
jest.mock("@/resources/Settings/SettingsLocalization", () => () => null)

import {IPermissions} from "@/types/keycloak"
import {PACKAGE_PERMISSIONS} from "@/resources/Settings/configurationPackage"

// Every permission the settings screen may ask for, as one "everything" grant.
const ALL = Object.values(IPermissions) as string[]

const tabs = () => {
    // Loaded after the mocks above.
    const {SettingsScreen} = require("./SettingsScreen")
    mockTabs.mockClear()
    render(<SettingsScreen />)
    return mockTabs.mock.calls.at(-1)?.[0] as string[] | undefined
}

it("is the last tab, with every permission the route requires", () => {
    mockGranted = ALL
    const labels = tabs()!
    expect(labels.at(-1)).toBe("configurationPackage.tab")
    expect(labels.filter((label) => label === "configurationPackage.tab")).toHaveLength(1)
})

it.each(PACKAGE_PERMISSIONS)("is hidden without %s, leaving the other tabs in place", (missing) => {
    mockGranted = ALL
    const all = tabs()!
    mockGranted = ALL.filter((permission) => permission !== missing)
    const labels = tabs()
    if (labels) {
        expect(labels).not.toContain("configurationPackage.tab")
        // The tabs before it keep their positions.
        expect(labels).toEqual(
            all.filter((label) => label !== "configurationPackage.tab" && labels.includes(label))
        )
    }
})
