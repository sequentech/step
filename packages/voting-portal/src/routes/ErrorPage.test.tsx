// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render} from "@testing-library/react"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {store} from "../store/store"
import {ErrorPage} from "./ErrorPage"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Header: ({logoUrl}: {logoUrl?: string}) => <header data-logo={logoUrl} />,
        HeaderErrorVariant: {HIDE_PROFILE: "hide-profile"},
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-essentials/public/Sequent_logo.svg", () => "platform-logo", {
    virtual: true,
})
jest.mock("@sequentech/ui-essentials/public/blank_logo.svg", () => "blank-logo", {virtual: true})
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({isAuthenticated: false}),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {APP_VERSION: "1.0.0", PUBLIC_BUCKET_URL: "http://bucket/"},
    }),
}))
jest.mock("../hooks/root-back-link", () => ({useRootBackLink: () => "/"}))
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: unknown) => unknown) => selector(mockState),
}))

let mockState: unknown

const Failing = (): React.ReactElement => {
    throw new Error("broken")
}

const renderErrorPage = () => {
    const router = createMemoryRouter(
        [{path: "/", element: <Failing />, errorElement: <ErrorPage />}],
        {initialEntries: ["/"]}
    )
    const view = render(<RouterProvider router={router} />)
    return {
        view,
        dispose: () => {
            view.unmount()
            router.dispose()
        },
    }
}

beforeEach(() => {
    jest.spyOn(console, "error").mockImplementation(() => {})
})

afterEach(() => {
    jest.restoreAllMocks()
})

test("carries the known election event's styling and logo", () => {
    mockState = {
        ...store.getState(),
        electionEvent: {
            event: {
                id: "event",
                presentation: {css: "color: rgb(1, 2, 3);", logo_url: "https://event/logo.svg"},
            },
        },
    }
    const {view, dispose} = renderErrorPage()
    try {
        const wrapper = view.container.querySelector(".voting-portal-wrapper")!
        expect(getComputedStyle(wrapper).color).toBe("rgb(1, 2, 3)")
        expect(view.container.querySelector("header")?.getAttribute("data-logo")).toBe(
            "https://event/logo.svg"
        )
        expect(view.getByText("errors.page.oopsWithoutStatus")).toBeTruthy()
    } finally {
        dispose()
    }
})

test("prefers the presentation frozen in a loaded ballot", () => {
    mockState = {
        ...store.getState(),
        electionEvent: {event: {id: "event", presentation: {css: "color: rgb(1, 2, 3);"}}},
        ballotStyles: {
            first: {
                id: "first",
                election_id: "first",
                ballot_eml: {election_event_presentation: {css: "color: rgb(4, 5, 6);"}},
            },
        },
    }
    const {view, dispose} = renderErrorPage()
    try {
        const wrapper = view.container.querySelector(".voting-portal-wrapper")!
        expect(getComputedStyle(wrapper).color).toBe("rgb(4, 5, 6)")
    } finally {
        dispose()
    }
})

test("shows no logo and no event styling while no event is known", () => {
    mockState = store.getState()
    const {view, dispose} = renderErrorPage()
    try {
        expect(view.container.querySelector("header")?.getAttribute("data-logo")).toBe("blank-logo")
        expect(view.getByText("errors.page.oopsWithoutStatus")).toBeTruthy()
    } finally {
        dispose()
    }
})
