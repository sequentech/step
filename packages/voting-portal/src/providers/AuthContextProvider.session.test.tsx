/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {sessionActivityStorageKey} from "../../../ui-core/src/services/sessionActivity"
import {act, render} from "@testing-library/react"
import AuthContextProvider, {AuthContext, AuthContextValues} from "./AuthContextProvider"
import {SettingsContext} from "./SettingsContextProvider"
import FakeKeycloak, {resetKeycloak} from "../../../ballot-verifier/src/__mocks__/keycloak"

jest.mock("keycloak-js", () =>
    jest.requireActual("../../../ballot-verifier/src/__mocks__/keycloak")
)
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))

let auth: AuthContextValues
let input: (event: Event) => void
let observedTokens: Array<string | undefined>
const Consumer = () => {
    auth = useContext(AuthContext)
    if (auth.isAuthenticated) observedTokens.push(auth.keycloakAccessToken)
    return null
}
const Settings = ({children}: {children: React.ReactNode}) => {
    const defaults = useContext(SettingsContext)
    return (
        <SettingsContext.Provider value={{...defaults, loaded: true}}>
            {children}
        </SettingsContext.Provider>
    )
}
beforeEach(() => {
    jest.useFakeTimers()
    resetKeycloak()
    localStorage.clear()
    observedTokens = []
    jest.spyOn(console, "log").mockImplementation(() => {})
    const original = window.addEventListener.bind(window)
    jest.spyOn(window, "addEventListener").mockImplementation((type, listener, options) => {
        if (type === "pointerdown") input = listener as (event: Event) => void
        original(type, listener, options)
    })
})
afterEach(() => {
    jest.useRealTimers()
    jest.restoreAllMocks()
})
const mount = async () => {
    const view = render(
        <Settings>
            <AuthContextProvider>
                <Consumer />
            </AuthContextProvider>
        </Settings>
    )
    await act(async () => {
        auth.setTenantEvent("tenant-a", "event-a")
    })
    return view
}
it("has a usable token on every authenticated render and expires idle authentication", async () => {
    await mount()
    expect(auth.isAuthenticated).toBe(true)
    expect(observedTokens.length).toBeGreaterThan(0)
    expect(observedTokens).not.toContain(undefined)
    const client = FakeKeycloak.instances[0]
    await act(async () => jest.advanceTimersByTimeAsync(15 * 60_000))
    expect(client.updateToken).not.toHaveBeenCalled()
    expect(client.logout).toHaveBeenCalledTimes(1)
    expect(auth.isAuthenticated).toBe(false)
    expect(auth.keycloakAccessToken).toBeUndefined()
})
it("refreshes on trusted input and removes refresh listeners on unmount", async () => {
    const remove = jest.spyOn(window, "removeEventListener")
    const view = await mount()
    const client = FakeKeycloak.instances[0]
    client.updateToken.mockImplementation(async () => {
        client.token = "updated-token"
        return true
    })
    await act(async () => {
        jest.advanceTimersByTime(1000)
        input({isTrusted: true} as Event)
    })
    expect(auth.keycloakAccessToken).toBe("updated-token")
    expect(client.updateToken).toHaveBeenCalledWith(60)
    view.unmount()
    expect(remove).toHaveBeenCalledWith("pointerdown", expect.any(Function), true)
    await jest.advanceTimersByTimeAsync(15 * 60_000)
    expect(client.updateToken).toHaveBeenCalledTimes(1)
    expect(client.logout).not.toHaveBeenCalled()
})

it("refuses a suspended-tab request synchronously at the deadline", async () => {
    await mount()
    const beforeSuspension = auth
    let token: string | undefined
    await act(async () => {
        jest.setSystemTime(Date.now() + 15 * 60_000)
        token = await beforeSuspension.getAccessTokenForRequest()
    })
    expect(token).toBeUndefined()
    expect(FakeKeycloak.instances[0].updateToken).not.toHaveBeenCalled()
    expect(FakeKeycloak.instances[0].logout).toHaveBeenCalledTimes(1)
})
it("keeps an idle session recoverable before fifteen minutes without sending an expired bearer", async () => {
    await mount()
    const client = FakeKeycloak.instances[0]
    client.isTokenExpired.mockReturnValue(true)
    await act(async () => {
        jest.setSystemTime(Date.now() + 5 * 60_000)
        client.onTokenExpired?.()
        expect(await auth.getAccessTokenForRequest()).toBeUndefined()
    })
    expect(client.logout).not.toHaveBeenCalled()
    expect(client.updateToken).not.toHaveBeenCalled()
    client.updateToken.mockImplementation(async () => {
        client.token = "recovered-token"
        client.isTokenExpired.mockReturnValue(false)
        return true
    })
    await act(async () => {
        input({isTrusted: true} as Event)
        expect(await auth.getAccessTokenForRequest()).toBe("recovered-token")
    })
    expect(auth.isAuthenticated).toBe(true)
    expect(client.logout).not.toHaveBeenCalled()
})

it("does not send profile reads when a persisted session was already idle", async () => {
    localStorage.setItem(
        sessionActivityStorageKey(
            "tenant-tenant-a-event-event-a",
            "synthetic-voter",
            "synthetic-session"
        )!,
        String(Date.now() - 15 * 60_000)
    )
    await mount()
    expect(auth.isAuthenticated).toBe(false)
    expect(FakeKeycloak.instances[0].loadUserProfile).not.toHaveBeenCalled()
    expect(FakeKeycloak.instances[0].logout).toHaveBeenCalledTimes(1)
})
