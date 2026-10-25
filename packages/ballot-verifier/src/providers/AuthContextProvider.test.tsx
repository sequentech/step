// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {sessionActivityStorageKey} from "../../../ui-core/src/services/sessionActivity"
import {act, render, screen} from "@testing-library/react"
import AuthContextProvider, {AuthContext, AuthContextValues} from "./AuthContextProvider"
import {GlobalSettings, SettingsContext} from "./SettingsContextProvider"
import FakeKeycloak, {keycloakSession, resetKeycloak} from "../__mocks__/keycloak"
import {resetSequentCore, sequentCore} from "../__mocks__/sequentCore"
import {IDS} from "../__mocks__/auditableBallots"

jest.mock("keycloak-js", () => jest.requireActual("../__mocks__/keycloak"))

// The deployment settings the Playwright journeys serve to the verifier.
const settings: GlobalSettings = {
    DISABLE_AUTH: false,
    QUERY_POLL_INTERVAL_MS: 60000,
    DEFAULT_TENANT_ID: IDS.tenant,
    DEFAULT_EVENT_ID: IDS.event,
    ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
    KEYCLOAK_URL: "https://keycloak.test/",
    HASURA_URL: "https://hasura.test/v1/graphql",
    APP_VERSION: "test",
    APP_HASH: "test",
    PUBLIC_BUCKET_URL: "https://s3.test/public/",
}
const realm = `tenant-${IDS.tenant}-event-${IDS.event}`

let observedTokens: Array<string | undefined>
let input: (event: Event) => void
let auth: AuthContextValues
const Voter = () => {
    auth = useContext(AuthContext)
    if (auth.isAuthenticated) observedTokens.push(auth.getAccessToken())
    return (
        <p>
            {auth.isAuthenticated
                ? `Signed in as ${auth.username} (${auth.firstName}, ${auth.email})`
                : "Signed out"}
        </p>
    )
}

function renderProvider(loaded = true) {
    const tree = (isLoaded: boolean) => (
        <SettingsContext.Provider
            value={
                isLoaded
                    ? {loaded: true, globalSettings: settings}
                    : {
                          loaded: false,
                          globalSettings: {...settings, KEYCLOAK_URL: "http://localhost:8090/"},
                      }
            }
        >
            <AuthContextProvider>
                <Voter />
            </AuthContextProvider>
        </SettingsContext.Provider>
    )
    const view = render(tree(loaded))
    return {
        loadSettings: () => view.rerender(tree(true)),
        /** What ApolloContextProvider does once it knows the route's election event. */
        openEvent: (defaultLocale?: string) =>
            act(async () => auth.login(IDS.tenant, IDS.event, defaultLocale)),
    }
}

const client = () => {
    expect(FakeKeycloak.instances).toHaveLength(1)
    return FakeKeycloak.instances[0]
}
const signedIn = "Signed in as synthetic-voter (Sam, voter@example.org)"

const withoutLanguageParameter = () => window.history.replaceState(null, "", "/")
const setLanguageCookie = (language: string) => {
    document.cookie = `USER_LANGUAGE=${language}; path=/`
}

beforeEach(() => {
    jest.useFakeTimers()
    const original = window.addEventListener.bind(window)
    jest.spyOn(window, "addEventListener").mockImplementation((type, listener, options) => {
        if (type === "pointerdown") input = listener as (event: Event) => void
        original(type, listener, options)
    })
    observedTokens = []
    resetKeycloak()
    resetSequentCore()
    localStorage.clear()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
})
afterEach(() => {
    // Pending refresh timers are dropped with the fake clock.
    jest.useRealTimers()
    jest.restoreAllMocks()
    window.history.replaceState(null, "", "/?lang=en")
    document.cookie = "USER_LANGUAGE=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/"
})

describe("signing in", () => {
    it("signs the voter in to the event's realm with the verifier client", async () => {
        const {openEvent} = renderProvider()
        expect(screen.getByText("Signed out")).toBeVisible()

        await openEvent()

        expect(await screen.findByText(signedIn)).toBeVisible()
        expect(client().config).toEqual({
            realm,
            url: settings.KEYCLOAK_URL,
            clientId: "ballot-verifier",
        })
        expect(client().init).toHaveBeenCalledTimes(1)
        expect(client().init).toHaveBeenCalledWith({
            onLoad: "login-required",
            checkLoginIframe: false,
            locale: "en",
        })
        expect(client().login).not.toHaveBeenCalled()
        expect(observedTokens.length).toBeGreaterThan(0)
        expect(observedTokens).not.toContain(undefined)
        expect(auth.getAccessToken()).toBe("voter-access-token")
        expect(localStorage.getItem("token")).toBe("voter-access-token")
        expect(auth.hasRole("voter")).toBe(true)
        expect(auth.hasRole("admin")).toBe(false)
    })

    it("waits for the global settings and an election event before creating a client", async () => {
        const {loadSettings, openEvent} = renderProvider(false)
        await openEvent()
        expect(FakeKeycloak.instances).toHaveLength(0)

        await act(async () => loadSettings())

        expect(await screen.findByText(signedIn)).toBeVisible()
        expect(client().config).toMatchObject({url: settings.KEYCLOAK_URL})
    })

    it("does not contact Keycloak before an election event is known", async () => {
        renderProvider()
        await act(async () => jest.advanceTimersByTimeAsync(60_000))

        expect(FakeKeycloak.instances).toHaveLength(0)
        expect(screen.getByText("Signed out")).toBeVisible()
    })

    it("sends a signed-out voter to the Keycloak login form in their language", async () => {
        keycloakSession.authenticated = false
        const {openEvent} = renderProvider()

        await openEvent()

        expect(client().login).toHaveBeenCalledWith(expect.objectContaining({locale: "en"}))
        expect(screen.getByText("Signed out")).toBeVisible()
        expect(localStorage.getItem("token")).toBeNull()
        expect(client().loadUserProfile).not.toHaveBeenCalled()
    })

    it("stays signed out when Keycloak cannot be initialised", async () => {
        keycloakSession.authenticated = new Error("Keycloak is unreachable")
        const {openEvent} = renderProvider()

        await openEvent()

        expect(client().init).toHaveBeenCalledTimes(1)
        expect(screen.getByText("Signed out")).toBeVisible()
        expect(auth.getAccessToken()).toBeUndefined()
        expect(localStorage.getItem("token")).toBeNull()
        expect(client().loadUserProfile).not.toHaveBeenCalled()
    })

    it("keeps the voter signed in when their profile cannot be loaded", async () => {
        keycloakSession.profile = new Error("Account API unavailable")
        const {openEvent} = renderProvider()

        await openEvent()

        expect(client().loadUserProfile).toHaveBeenCalledTimes(1)
        expect(screen.getByText(/^Signed in as/)).toBeVisible()
        expect(auth.getAccessToken()).toBe("voter-access-token")
    })
})

// Language priority: the lang URL parameter, then the voter's saved choice,
// then a forced election default (docs: election managers' Languages reference).
describe("the Keycloak login language", () => {
    it("prefers the lang URL parameter over the saved language", async () => {
        setLanguageCookie("es")
        const {openEvent} = renderProvider()

        await openEvent("fr")

        expect(client().init).toHaveBeenCalledWith(expect.objectContaining({locale: "en"}))
    })

    it("uses the saved language as a BCP 47 tag", async () => {
        withoutLanguageParameter()
        setLanguageCookie("cat")
        // ISO 639-2/T "cat" is BCP 47 "ca", as sequent-core converts it.
        sequentCore.iso_639_2t_to_bcp47_js.mockImplementation((code: string) =>
            code === "cat" ? "ca" : code
        )
        const {openEvent} = renderProvider()

        await openEvent("fr")

        expect(client().init).toHaveBeenCalledWith(expect.objectContaining({locale: "ca"}))
    })

    it("leaves the language to Keycloak when nothing selects one", async () => {
        withoutLanguageParameter()
        const {openEvent} = renderProvider()

        await openEvent()

        expect(client().init).toHaveBeenCalledWith(expect.objectContaining({locale: undefined}))
    })

    it("uses the election's forced default language otherwise", async () => {
        withoutLanguageParameter()
        const {openEvent} = renderProvider()

        await openEvent("fr")

        expect(client().init).toHaveBeenCalledWith(expect.objectContaining({locale: "fr"}))
    })
})

describe("keeping the session", () => {
    it("refreshes after actual input and exposes the new token", async () => {
        const {openEvent} = renderProvider()
        await openEvent()
        await screen.findByText(signedIn)
        client().updateToken.mockImplementation(async () => {
            client().token = "refreshed-token"
            return true
        })
        await act(async () => {
            jest.advanceTimersByTime(1000)
            input({isTrusted: true} as Event)
        })
        expect(client().updateToken).toHaveBeenCalledWith(60)
        expect(auth.getAccessToken()).toBe("refreshed-token")
        expect(localStorage.getItem("token")).toBe("refreshed-token")
    })

    it("ends an idle session at fifteen minutes without background refresh", async () => {
        const {openEvent} = renderProvider()
        await openEvent()
        await screen.findByText(signedIn)
        await act(async () => jest.advanceTimersByTimeAsync(15 * 60_000))
        expect(client().updateToken).not.toHaveBeenCalled()
        expect(client().logout).toHaveBeenCalledTimes(1)
        expect(auth.getAccessToken()).toBeUndefined()
        expect(localStorage.getItem("token")).toBeNull()
        expect(screen.getByText("Signed out")).toBeVisible()
    })

    it("clears authentication when Keycloak no longer has a token", async () => {
        const {openEvent} = renderProvider()
        await openEvent()
        await screen.findByText(signedIn)
        client().updateToken.mockImplementation(async () => {
            client().token = undefined
            return false
        })
        await act(async () => {
            jest.advanceTimersByTime(1000)
            input({isTrusted: true} as Event)
        })
        await act(async () => jest.advanceTimersByTimeAsync(5 * 60_000))
        expect(client().updateToken).toHaveBeenCalledTimes(1)
        expect(localStorage.getItem("token")).toBeNull()
        expect(screen.getByText("Signed out")).toBeVisible()
    })
})

describe("account actions", () => {
    it("logs the voter out of Keycloak and forgets the stored token", async () => {
        const {openEvent} = renderProvider()
        await openEvent()
        await screen.findByText(signedIn)

        await act(async () => {
            await auth.logout()
        })

        expect(client().logout).toHaveBeenCalledTimes(1)
        expect(localStorage.getItem("token")).toBeNull()
    })

    it("opens the voter's Keycloak account page", async () => {
        const {openEvent} = renderProvider()
        await openEvent()
        await screen.findByText(signedIn)

        await act(() => auth.openProfileLink())

        expect(client().accountManagement).toHaveBeenCalledTimes(1)
    })

    it("does nothing before a Keycloak client exists", async () => {
        renderProvider()

        await act(async () => {
            await auth.logout()
        })
        await act(() => auth.openProfileLink())

        expect(auth.hasRole("voter")).toBe(false)
        expect(auth.getAccessToken()).toBeUndefined()
        expect(FakeKeycloak.instances).toHaveLength(0)
    })
})

it("checks suspended-tab request expiry before attaching its access token", async () => {
    const {openEvent} = renderProvider()
    await openEvent()
    await act(async () => {
        jest.setSystemTime(Date.now() + 15 * 60_000)
        expect(await auth.getAccessTokenForRequest()).toBeUndefined()
    })
    expect(client().updateToken).not.toHaveBeenCalled()
    expect(client().logout).toHaveBeenCalledTimes(1)
})

it("does not load a profile from a persisted session that already expired", async () => {
    localStorage.setItem(
        sessionActivityStorageKey(realm, "synthetic-voter", "synthetic-session")!,
        String(Date.now() - 15 * 60_000)
    )
    const {openEvent} = renderProvider()
    await openEvent()
    expect(auth.isAuthenticated).toBe(false)
    expect(client().loadUserProfile).not.toHaveBeenCalled()
    expect(client().logout).toHaveBeenCalledTimes(1)
})
