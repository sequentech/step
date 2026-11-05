/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {act, renderHook, waitFor} from "@testing-library/react"
import {afterEach, beforeEach, expect, it, jest} from "@jest/globals"
import Keycloak, {type KeycloakInitOptions} from "keycloak-js"
import {useAuthenticatedResults} from "./useAuthenticatedResults"
import type {GlobalSettings} from "../providers/SettingsContextProvider"

jest.mock("keycloak-js", () => ({__esModule: true, default: jest.fn()}))

const settings: GlobalSettings = {
    RESULTS_PORTAL_CLIENT_ID: "results-portal",
    KEYCLOAK_URL: "https://identity.invalid",
    HASURA_URL: "https://graphql.invalid",
    PUBLIC_BUCKET_URL: "https://files.invalid",
    APP_VERSION: "test",
    APP_HASH: "test",
}

function session(token: string) {
    let finish!: (value: boolean) => void
    const ready = new Promise<boolean>((resolve) => {
        finish = resolve
    })
    const adapter = {
        token: undefined as string | undefined,
        tokenParsed: {sub: "synthetic-voter"},
        init: jest.fn(async (_options?: KeycloakInitOptions) => {
            await ready
            adapter.token = token
            return true
        }),
        updateToken: jest.fn(async () => false),
        loadUserProfile: jest.fn(async () => ({username: "synthetic-voter"})),
        logout: jest.fn(async () => {}),
        accountManagement: jest.fn(async () => {}),
        onTokenExpired: undefined as (() => void) | undefined,
    }
    return {adapter, finish: () => finish(true)}
}

beforeEach(() => {
    jest.mocked(Keycloak).mockReset()
})
afterEach(() => {
    jest.restoreAllMocks()
})

for (const change of ["event", "server", "client"] as const) {
    it(`withholds the previous token in every render when the ${change} changes`, async () => {
        const first = session(`token-first-${change}`)
        const second = session(`token-second-${change}`)
        jest.mocked(Keycloak)
            .mockImplementationOnce(() => first.adapter as unknown as jest.Mocked<Keycloak>)
            .mockImplementationOnce(() => second.adapter as unknown as jest.Mocked<Keycloak>)
        const observations: {scope: string; token?: string}[] = []
        const initial = {eventId: `event-${change}`, configuration: settings, scope: "first"}
        const hook = renderHook(
            ({eventId, configuration, scope}) => {
                const auth = useAuthenticatedResults(
                    configuration,
                    "tenant-synthetic",
                    eventId,
                    true
                )
                observations.push({scope, token: auth.token})
                return auth
            },
            {initialProps: initial}
        )
        await act(async () => first.finish())
        await waitFor(() => expect(hook.result.current.token).toBe(`token-first-${change}`))
        expect(first.adapter.init).toHaveBeenCalledWith(
            expect.objectContaining({onLoad: "login-required", checkLoginIframe: false})
        )
        const next = {
            eventId: change === "event" ? "different-event" : initial.eventId,
            configuration: {
                ...settings,
                ...(change === "server" ? {KEYCLOAK_URL: "https://other-identity.invalid"} : {}),
                ...(change === "client" ? {RESULTS_PORTAL_CLIENT_ID: "other-results-client"} : {}),
            },
            scope: "second",
        }
        hook.rerender(next)
        expect(
            observations.filter(({scope}) => scope === "second").map(({token}) => token)
        ).not.toContain(`token-first-${change}`)
        await act(async () => second.finish())
        await waitFor(() => expect(hook.result.current.token).toBe(`token-second-${change}`))
        hook.unmount()
    })
}
