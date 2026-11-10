/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, cleanup} from "@testing-library/react"
import "@testing-library/jest-dom"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {TenantContext} from "@/providers/TenantContextProvider"
import {TrusteeDashboardRoute} from "./TrusteeDashboardRoute"

jest.mock("@/providers/TenantContextProvider", () => ({
    TenantContext: jest.requireActual("react").createContext({}),
}))
jest.mock("./TrusteeDashboard", () => ({
    TrusteeDashboard: () => <h1>Trustee dashboard</h1>,
}))

const renderRoute = (tenant?: Partial<Sequent_Backend_Tenant>) =>
    render(
        <TenantContext.Provider
            value={{
                tenantId: "tenant",
                setTenantId: () => undefined,
                tenant: tenant as Sequent_Backend_Tenant | undefined,
                setTenant: () => undefined,
            }}
        >
            <TrusteeDashboardRoute />
        </TenantContext.Provider>
    )

afterEach(cleanup)

it("shows the browser trustee page where signed configuration packages are optional", () => {
    renderRoute({
        id: "tenant",
        settings: {languages: ["en"], configuration_signing: {policy: "optional"}},
    })
    expect(screen.getByRole("heading", {name: "Trustee dashboard"})).toBeVisible()
})

it("shows nothing for a tenant without the setting, which requires them", () => {
    const {container} = renderRoute({id: "tenant", settings: {languages: ["en"]}})
    expect(container).toBeEmptyDOMElement()
})

it("shows nothing for a tenant that requires signed configuration packages", () => {
    const {container} = renderRoute({
        id: "tenant",
        settings: {configuration_signing: {policy: "required"}},
    })
    expect(container).toBeEmptyDOMElement()
})

it("shows nothing until the tenant has loaded", () => {
    const {container} = renderRoute()
    expect(container).toBeEmptyDOMElement()
})
