// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {MemoryRouter} from "react-router-dom"
import App from "./App"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"

jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual("../../ui-core/src/services/pageBranding"),
        applyConfigurationLanguagePolicy: jest.fn(),
        ETranslationScope: {BALLOT_VERIFIER: "ballot-verifier"},
        getValueFromCookie: jest.fn(),
        overwriteTranslations: jest.fn(),
        setCookie: jest.fn(),
        USER_LANGUAGE_COOKIE_NAME: "language",
    }),
    {virtual: true}
)
jest.mock(
    "@sequentech/ui-essentials",
    () => {
        const {createElement} = require("react")
        return {
            Footer: () => createElement("footer"),
            Header: ({logoUrl}: {logoUrl?: string}) =>
                createElement("header", {"data-logo": logoUrl}),
            NotFoundScreen: () => null,
            PageBanner: ({children}: React.PropsWithChildren) =>
                createElement("main", null, children),
        }
    },
    {virtual: true}
)
jest.mock("./index", () => ({
    RouteParameterProvider: ({children}: React.PropsWithChildren) => children,
}))
jest.mock("./providers/ApolloContextProvider", () => ({
    ApolloContextProvider: ({children}: React.PropsWithChildren) => children,
    ApolloWrapper: ({children}: React.PropsWithChildren) => children,
}))
jest.mock("./providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({}),
}))
jest.mock("./providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {
            DISABLE_AUTH: false,
            PUBLIC_BUCKET_URL: "https://files.example.org/public/",
        },
    }),
}))
jest.mock("./screens/ConfirmationScreen", () => ({ConfirmationScreen: () => null}))
jest.mock("./screens/LoginScreen", () => ({LoginScreen: () => null}))
jest.mock("./screens/HomeScreen", () => {
    const {createElement} = require("react")
    return {
        HomeScreen: ({setConfirmationBallot}: {setConfirmationBallot: (ballot: unknown) => void}) =>
            createElement(
                "button",
                {onClick: () => setConfirmationBallot(mockImportedBallot)},
                "Import ballot"
            ),
    }
})
jest.mock("./services/BallotService", () => ({provideBallotService: () => ({})}))
jest.mock("./store/ballotStyles/ballotStylesSlice", () => ({
    selectBallotStyleByElectionEventId: () => () => mockBallotStyle,
}))
jest.mock("./store/hooks", () => ({
    useAppSelector: (selector: (state: unknown) => unknown) => selector(undefined),
}))

let mockBallotStyle: unknown
let mockImportedBallot: unknown

const renderApp = () =>
    render(
        <MemoryRouter initialEntries={["/tenant/tenant/event/event/start"]}>
            <App />
        </MemoryRouter>
    ).container.firstElementChild as Element

beforeEach(() => {
    mockBallotStyle = undefined
    mockImportedBallot = undefined
})

describe("the event's branding", () => {
    const eventWith = (fields: Record<string, unknown>) => ({
        ballot_eml: {election_event_presentation: {css: "", ...fields}},
    })
    const logo = () => document.querySelector("header")?.getAttribute("data-logo")
    const tabIcons = () =>
        Array.from(document.head.querySelectorAll("link[data-page-branding]")).map((link) =>
            link.getAttribute("href")
        )

    test("shows nothing of the platform while the event is unknown", () => {
        renderApp()

        expect(document.querySelector("footer")).toBeNull()
        expect(logo()).toBe(BlankLogoImg)
        expect(tabIcons()).toEqual(["data:,"])
    })

    test("shows the platform's logo, footer and icons for an event that sets none", () => {
        mockBallotStyle = eventWith({logo_url: null})

        const wrapper = renderApp()

        expect(document.querySelector("footer")).not.toBeNull()
        expect(logo()).toBe(SequentLogo)
        expect(tabIcons()).toContain("/favicon.ico")
        expect(wrapper.classList.contains("platform-branding-hidden")).toBe(false)
    })

    test("shows only the event's own branding when the platform is hidden", () => {
        mockBallotStyle = eventWith({
            logo_url: null,
            favicon_url: "https://event.example/icon.png",
            page_title: "Check your ballot",
            platform_branding_policy: "hide",
        })

        const wrapper = renderApp()

        expect(document.querySelector("footer")).toBeNull()
        expect(logo()).toBe(BlankLogoImg)
        expect(tabIcons()).toEqual(["https://event.example/icon.png"])
        expect(document.title).toBe("Check your ballot")
        expect(wrapper.classList.contains("platform-branding-hidden")).toBe(true)
    })

    test("follows the event inside an imported auditable ballot", () => {
        mockBallotStyle = eventWith({logo_url: "https://event.example/published.svg"})
        mockImportedBallot = {
            election_config: {
                election_event_presentation: {
                    css: "",
                    logo_url: "https://event.example/imported.svg",
                    platform_branding_policy: "hide",
                },
            },
        }

        renderApp()
        fireEvent.click(screen.getByRole("button", {name: "Import ballot"}))

        expect(document.querySelector("footer")).toBeNull()
    })
})
