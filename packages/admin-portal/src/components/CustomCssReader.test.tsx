/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, waitFor} from "@testing-library/react"
import {CoreAdminContext, testDataProvider} from "react-admin"
import {Provider, createStore} from "jotai"
import cssInputLookAndFeel from "@/atoms/css-input-look-and-feel"
import {CustomCssReader} from "./CustomLayout"

jest.mock("react-admin", () => {
    const {TextEncoder, TextDecoder} = jest.requireActual("node:util")
    Object.assign(globalThis, {TextEncoder, TextDecoder})
    return jest.requireActual("ra-core")
})
jest.mock("./CustomAppBar", () => ({CustomAppBar: () => null}))
jest.mock("./CustomMenu", () => ({CustomMenu: () => null}))
jest.mock("./menu/CustomSidebar", () => ({CustomSidebar: () => null}))
jest.mock("./election-event/import-data/ImportDataDrawer", () => ({ImportDataDrawer: () => null}))
jest.mock("./election-event/create/CreateElectionEventDrawer", () => ({
    CreateDataDrawer: () => null,
}))
jest.mock("@/providers/CreateElectionEventContextProvider", () => ({
    CreateElectionEventProvider: () => null,
}))
jest.mock("./signing/SigningProvider", () => ({SigningProvider: () => null}))
jest.mock("@/resources/Publish/ClosedVotingCard", () => ({ClosedVotingCard: () => null}))
jest.mock("@/resources/Reports/ReportSigning", () => ({
    ReportCompletionActions: () => null,
    TransmissionCompletionActions: () => null,
}))
jest.mock("@/providers/TenantContextProvider", () => ({
    TenantContext: require("react").createContext({tenantId: "tenant"}),
}))
const mockLanguagePolicy = jest.fn()
jest.mock("@sequentech/ui-core", () => ({
    applyConfigurationLanguagePolicy: (settings: unknown) => mockLanguagePolicy(settings),
}))

const CSS = ".RaLayout-content {display: none}"
const languages = {language_conf: {enabled_language_codes: ["en"]}}
/** The CSS that the reader applies once the tenant has loaded (its languages apply then too). */
const readerCss = async (signing: Record<string, unknown>) => {
    const settings = {...languages, ...signing}
    const store = createStore()
    const getOne = jest.fn().mockResolvedValue({
        data: {id: "tenant", annotations: {css: CSS}, settings},
    })
    render(
        <CoreAdminContext dataProvider={testDataProvider({getOne})}>
            <Provider store={store}>
                <CustomCssReader />
            </Provider>
        </CoreAdminContext>
    )
    await waitFor(() => expect(mockLanguagePolicy).toHaveBeenCalledWith(settings))
    expect(getOne).toHaveBeenCalledWith("sequent_backend_tenant", {id: "tenant"})
    return store.get(cssInputLookAndFeel)
}

beforeEach(() => mockLanguagePolicy.mockReset())

it("applies the tenant's custom CSS where signed configuration packages are optional", async () => {
    expect(await readerCss({configuration_signing: {policy: "optional"}})).toBe(CSS)
})

it.each([
    ["requires signed packages", {configuration_signing: {policy: "required"}}],
    ["has no signing setting", {}],
])("keeps the staff screens as built for a tenant that %s", async (_, signing) => {
    expect(await readerCss(signing)).toBe("")
})
