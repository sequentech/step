// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import {Provider} from "react-redux"
import {MemoryRouter} from "react-router-dom"
import {
    AuthContext,
    type AuthContextValues,
    CONTACT_CHANGE_ACTION,
    ContactChangeOutcome,
    contactChangeOutcomeOf,
    fetchContactChangeAvailable,
    messagingChannelsUrl,
} from "../providers/AuthContextProvider"
import {store} from "../store/store"
import {PortalChrome} from "./PortalChrome"

// keycloak-js is an ES module jest can't load; the provider's own Keycloak is never created here.
jest.mock("keycloak-js", () => ({__esModule: true, default: jest.fn()}))
// Only the header's actions slot matters here: the shared header has its own tests.
jest.mock("@sequentech/ui-essentials", () => ({
    ...jest.requireActual("../../../ui-essentials/src/ballot/presentation"),
    ...jest.requireActual("../../../ui-essentials/src/ballot/engine"),
    ...jest.requireActual("../../../ui-essentials/src/ballot/selection"),
    Header: ({actions}: {actions?: React.ReactNode}) => <header>{actions}</header>,
    Footer: () => null,
    PageBanner: ({children}: React.PropsWithChildren) => <main>{children}</main>,
}))
jest.mock("./WaterMark/Watermark", () => ({__esModule: true, default: () => null}))
jest.mock("./ScreenAudioInstructions/ScreenAudioInstructions", () => ({
    ScreenAudioInstructions: () => null,
}))
jest.mock("./BallotSelectionAdapter", () => ({
    BallotSelectionAdapter: ({children}: React.PropsWithChildren) => <>{children}</>,
}))
jest.mock("../hooks/useElectionClassName", () => ({useElectionClassName: () => undefined}))
// The portal's English texts, so the assertions read as the voter does.
jest.mock("react-i18next", () => {
    const english = jest.requireActual("../translations/en").default.translations
    const t = (key: string) =>
        key.split(".").reduce<any>((node, part) => node?.[part], english) ?? key
    return {
        ...jest.requireActual("react-i18next"),
        useTranslation: () => ({t, i18n: {language: "en", changeLanguage: jest.fn()}}),
    }
})

const CHANGE = "Change how we contact you"

const auth = (values: Partial<AuthContextValues> = {}): AuthContextValues => ({
    isAuthContextInitialized: true,
    isAuthenticated: true,
    userId: "voter",
    username: "voter",
    email: "",
    firstName: "Voter",
    keycloakAccessToken: "token",
    logout: jest.fn(),
    getExpiry: () => undefined,
    setTenantEvent: jest.fn(),
    hasRole: () => false,
    isKiosk: () => false,
    openProfileLink: async () => undefined,
    isGoldUser: () => false,
    reauthWithGold: async () => undefined,
    changeContactChannel: jest.fn(async () => undefined),
    ...values,
})

const renderChrome = (values: AuthContextValues) =>
    render(
        <Provider store={store}>
            <MemoryRouter>
                <AuthContext.Provider value={values}>
                    <PortalChrome>
                        <p>screen</p>
                    </PortalChrome>
                </AuthContext.Provider>
            </MemoryRouter>
        </Provider>
    )

describe("the contact change entry", () => {
    test("is hidden while the event offers no channel for codes", () => {
        renderChrome(auth({contactChangeAvailable: false}))
        expect(screen.getByText("screen")).toBeInTheDocument()
        expect(screen.queryByRole("button", {name: CHANGE})).toBeNull()
    })

    test("is hidden when the context can't tell (an older or offline context)", () => {
        renderChrome(auth({contactChangeAvailable: undefined}))
        expect(screen.queryByRole("button", {name: CHANGE})).toBeNull()
    })

    test("is hidden for a voter who isn't signed in", () => {
        renderChrome(auth({isAuthenticated: false, contactChangeAvailable: true}))
        expect(screen.queryByRole("button", {name: CHANGE})).toBeNull()
    })

    test("is hidden without a way to start the change", () => {
        renderChrome(auth({contactChangeAvailable: true, changeContactChannel: undefined}))
        expect(screen.queryByRole("button", {name: CHANGE})).toBeNull()
    })

    test("is shown when the event offers a channel, and starts the change", () => {
        const changeContactChannel = jest.fn(async () => undefined)
        renderChrome(auth({contactChangeAvailable: true, changeContactChannel}))
        fireEvent.click(screen.getByRole("button", {name: CHANGE}))
        expect(changeContactChannel).toHaveBeenCalledTimes(1)
    })
})

describe("the contact change's outcome", () => {
    test.each([
        [ContactChangeOutcome.Success, "Your contact was changed. Codes and notices now go there."],
        [ContactChangeOutcome.Cancelled, "Your contact was not changed."],
        [
            ContactChangeOutcome.Error,
            "We couldn't change how we contact you. Try again later or contact support.",
        ],
    ])("%s is explained to the voter", (outcome, text) => {
        const clearContactChangeOutcome = jest.fn()
        renderChrome(
            auth({
                contactChangeAvailable: true,
                contactChangeOutcome: outcome,
                clearContactChangeOutcome,
            })
        )
        const alert = screen.getByRole("alert")
        expect(alert).toHaveTextContent(text)
        fireEvent.click(within(alert).getByRole("button", {name: "Close"}))
        expect(clearContactChangeOutcome).toHaveBeenCalledTimes(1)
    })

    test("nothing is shown without an outcome", () => {
        renderChrome(auth({contactChangeAvailable: true}))
        expect(screen.queryByRole("alert")).toBeNull()
    })

    test("only the contact change's action is reported", () => {
        expect(contactChangeOutcomeOf("success", CONTACT_CHANGE_ACTION)).toBe(
            ContactChangeOutcome.Success
        )
        expect(contactChangeOutcomeOf("cancelled", CONTACT_CHANGE_ACTION)).toBe(
            ContactChangeOutcome.Cancelled
        )
        expect(contactChangeOutcomeOf("error", CONTACT_CHANGE_ACTION)).toBe(
            ContactChangeOutcome.Error
        )
        expect(contactChangeOutcomeOf("success", "UPDATE_PASSWORD")).toBeUndefined()
        expect(contactChangeOutcomeOf("success", undefined)).toBeUndefined()
    })
})

describe("whether the event offers a channel for codes", () => {
    const answer = (status: number, body: unknown) =>
        jest.fn(async () => new Response(JSON.stringify(body), {status}))
    let log: jest.SpyInstance

    beforeEach(() => {
        log = jest.spyOn(console, "info").mockImplementation(() => undefined)
    })
    afterEach(() => log.mockRestore())

    test("asks the realm's public messaging channels", () => {
        expect(messagingChannelsUrl("https://auth.example/", "tenant-1-event-2")).toBe(
            "https://auth.example/realms/tenant-1-event-2/messaging-channels"
        )
    })

    test("is yes when the event offers at least one channel", async () => {
        const fetchMock = answer(200, {code_channels: ["WHATSAPP", "SMS"]})
        await expect(
            fetchContactChangeAvailable("https://auth.example", "realm", undefined, fetchMock)
        ).resolves.toBe(true)
        expect(fetchMock).toHaveBeenCalledWith(
            "https://auth.example/realms/realm/messaging-channels",
            expect.anything()
        )
    })

    test.each([
        ["no channel", answer(200, {code_channels: []})],
        ["an older Keycloak without the endpoint", answer(404, {})],
        ["a server error", answer(500, {})],
        ["an unexpected answer", answer(200, {code_channels: "WHATSAPP"})],
        [
            "a network failure",
            jest.fn(async () => {
                throw new TypeError("Failed to fetch")
            }),
        ],
    ])("is no with %s, and says why once", async (_case, fetchMock) => {
        await expect(
            fetchContactChangeAvailable("https://auth.example", "realm", undefined, fetchMock)
        ).resolves.toBe(false)
        expect(log).toHaveBeenCalledTimes(1)
    })

    // R7-14: with the voter's token Keycloak answers for that voter (their Posts and the
    // code-step gate); without one, only for the event.
    test("sends the voter's token, so Keycloak decides for that voter", async () => {
        const fetchMock = answer(200, {code_channels: ["SMS"]})
        await expect(
            fetchContactChangeAvailable("https://auth.example", "realm", "voter-token", fetchMock)
        ).resolves.toBe(true)
        expect(fetchMock).toHaveBeenCalledWith(
            "https://auth.example/realms/realm/messaging-channels",
            {headers: {Accept: "application/json", Authorization: "Bearer voter-token"}}
        )
    })

    test("sends no Authorization header without a token", async () => {
        const fetchMock = answer(200, {code_channels: ["SMS"]})
        await fetchContactChangeAvailable("https://auth.example", "realm", undefined, fetchMock)
        expect(fetchMock).toHaveBeenCalledWith(
            "https://auth.example/realms/realm/messaging-channels",
            {headers: {Accept: "application/json"}}
        )
    })

    test("is no without a Keycloak to ask", async () => {
        const fetchMock = answer(200, {code_channels: ["SMS"]})
        await expect(
            fetchContactChangeAvailable(undefined, "realm", undefined, fetchMock)
        ).resolves.toBe(false)
        expect(fetchMock).not.toHaveBeenCalled()
        expect(log).toHaveBeenCalledTimes(1)
    })
})
