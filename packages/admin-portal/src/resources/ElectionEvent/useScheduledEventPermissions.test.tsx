/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {renderHook} from "@testing-library/react"
import {useScheduledEventPermissions} from "./useScheduledEventPermissions"

const mockGetOne = jest.fn()

jest.mock("react-admin", () => ({
    useGetOne: (...args: unknown[]) => mockGetOne(...args),
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthorized: () => true}),
}))

/** Whether only lockdown periods can change, with `tenant` as the tenant read. */
const onlyLockdownPeriodFor = (tenant: unknown) => {
    mockGetOne.mockReturnValue({data: tenant})
    return renderHook(() => useScheduledEventPermissions()).result.current.onlyLockdownPeriod
}

beforeEach(() => mockGetOne.mockReset())

it("reads the selected tenant", () => {
    onlyLockdownPeriodFor({id: "tenant", settings: null})
    expect(mockGetOne).toHaveBeenCalledWith(
        "sequent_backend_tenant",
        {id: "tenant"},
        {enabled: true}
    )
})

it("keeps to lockdown periods where the tenant requires signed packages", () => {
    expect(
        onlyLockdownPeriodFor({
            id: "tenant",
            settings: {configuration_signing: {policy: "required"}},
        })
    ).toBe(true)
})

it("keeps to lockdown periods for a tenant without the setting", () => {
    expect(onlyLockdownPeriodFor({id: "tenant", settings: {languages: ["en"]}})).toBe(true)
    expect(onlyLockdownPeriodFor({id: "tenant", settings: null})).toBe(true)
})

it("keeps to lockdown periods until the tenant has loaded", () => {
    expect(onlyLockdownPeriodFor(undefined)).toBe(true)
})

it("changes every date where the setting makes signed packages optional", () => {
    expect(
        onlyLockdownPeriodFor({
            id: "tenant",
            settings: {configuration_signing: {policy: "optional"}},
        })
    ).toBe(false)
})
