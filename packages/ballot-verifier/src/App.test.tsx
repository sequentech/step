// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {MemoryRouter} from "react-router-dom"
import App from "./App"

jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual("../../ui-core/src/services/sanitizePresentationCss"),
    }),
    {virtual: true}
)
jest.mock(
    "@sequentech/ui-essentials",
    () => {
        const {createElement} = require("react")
        return {
            Footer: () => null,
            Header: () => null,
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
    selectBallotStyleByElectionId: () => () => mockBallotStyle,
    selectBallotStyleElectionIds: () => ["election"],
    selectFirstBallotStyle: () => mockBallotStyle,
}))
jest.mock("./store/hooks", () => ({
    useAppSelector: (selector: (state: unknown) => unknown) => selector(undefined),
}))

let mockBallotStyle: unknown
let mockImportedBallot: unknown

const presentation = (css: string) => ({election_event_presentation: {css}})
const unsafeCss = (source: string) => `
    background-image: url(https://${source}.example/image.png);
    .candidate::after { content: "${source}"; }
`

const renderApp = () =>
    render(
        <MemoryRouter initialEntries={["/tenant/tenant/event/event/start"]}>
            <App />
        </MemoryRouter>
    ).container.firstElementChild as Element

// Emotion adds rules through the CSSOM here, so <style> elements have no text to read.
const appliedCss = () =>
    Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules))
        .map((rule) => rule.cssText)
        .join("\n")

const expectOnlySafeCss = (keptColor: string, source: string) => {
    const applied = appliedCss()

    expect(applied).toContain(keptColor)
    expect(applied).not.toContain(source)
}

beforeEach(() => {
    mockBallotStyle = undefined
    mockImportedBallot = undefined
})

test("filters the CSS of the published ballot style", () => {
    mockBallotStyle = {ballot_eml: presentation(`color: rgb(1, 2, 3); ${unsafeCss("published")}`)}

    const wrapper = renderApp()

    expect(getComputedStyle(wrapper).color).toBe("rgb(1, 2, 3)")
    expectOnlySafeCss("rgb(1, 2, 3)", "published")
})

test("filters the CSS inside an imported auditable ballot", () => {
    mockBallotStyle = {ballot_eml: presentation("color: rgb(1, 2, 3);")}
    mockImportedBallot = {
        election_config: presentation(`color: rgb(4, 5, 6); ${unsafeCss("imported")}`),
    }

    const wrapper = renderApp()
    fireEvent.click(screen.getByRole("button", {name: "Import ballot"}))

    expect(getComputedStyle(wrapper).color).toBe("rgb(4, 5, 6)")
    expectOnlySafeCss("rgb(4, 5, 6)", "imported")
})

test("keeps images from the public bucket", () => {
    mockBallotStyle = {
        ballot_eml: presentation(
            "background-image: url(https://files.example.org/public/logo.png);"
        ),
    }

    const wrapper = renderApp()

    expect(getComputedStyle(wrapper).backgroundImage).toContain(
        "https://files.example.org/public/logo.png"
    )
})
