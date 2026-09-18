// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import theme from "../../ui-essentials/src/services/theme"
import {store} from "./store/store"
import App from "./App"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    sortElectionList: (elections: unknown[]) => elections,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Footer: () => null,
        Header: () => null,
        PageBanner: ({children}: any) => <main>{children}</main>,
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-essentials/public/Sequent_logo.svg", () => "logo", {virtual: true})
jest.mock("@sequentech/ui-essentials/public/blank_logo.svg", () => "logo", {virtual: true})
jest.mock("./providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthenticated: true,
        getExpiry: () => undefined,
    }),
}))
jest.mock("./providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("./providers/ApolloContextProvider", () => ({
    ApolloWrapper: ({children}: any) => <>{children}</>,
}))
jest.mock("./components/WaterMark/Watermark", () => ({__esModule: true, default: () => null}))
jest.mock("./hooks/useElectionClassName", () => ({useElectionClassName: () => []}))
jest.mock("./services/ElectionEventConfig", () => ({
    createElectionEventConfigLoader: () => jest.fn(),
}))
jest.mock("./store/hooks", () => ({
    useAppSelector: (selector: any) => selector(mockState),
    useAppDispatch: () => jest.fn(),
}))
let mockState: any
const eventCss = "color: rgb(1, 2, 3);"
const ballotCss = "color: rgb(4, 5, 6);"

beforeEach(() => {
    jest.spyOn(window, "scrollTo").mockImplementation(() => {})
})

afterEach(() => {
    jest.restoreAllMocks()
})

const cases = [
    {
        name: "before any ballot loads",
        loaded: "none",
        presentation: {css: ballotCss},
        expected: "rgb(1, 2, 3)",
    },
    {
        name: "when another election loads first",
        loaded: "other",
        presentation: {css: ballotCss},
        expected: "rgb(1, 2, 3)",
    },
    {
        name: "with the first ballot's frozen presentation",
        loaded: "first",
        presentation: {css: ballotCss},
        expected: "rgb(4, 5, 6)",
    },
    {
        name: "with explicitly empty frozen CSS",
        loaded: "first",
        presentation: {css: ""},
        expected: null,
    },
    {
        name: "with no CSS in the frozen presentation",
        loaded: "first",
        presentation: {},
        expected: null,
    },
]

describe.each(["election-chooser", "support-materials"])("event styling on %s", (route) => {
    test.each(cases)(
        "applies the appropriate presentation $name",
        async ({loaded, presentation, expected}) => {
            const first = {id: "first", presentation: {}}
            const other = {id: "other", presentation: {}}
            const style = (id: string) => ({
                id,
                election_id: id,
                ballot_eml: {election_event_presentation: presentation},
            })
            mockState = {
                ...store.getState(),
                elections: {first, other},
                electionEvent: {event: {id: "event", presentation: {css: eventCss}}},
                ballotStyles: loaded === "none" ? {} : {[loaded]: style(loaded)},
            }
            const router = createMemoryRouter(
                [
                    {
                        path: "/tenant/:tenantId/event/:eventId",
                        element: <App />,
                        children: [{path: route, element: <div>Route content</div>}],
                    },
                ],
                {initialEntries: [`/tenant/tenant/event/event/${route}`]}
            )
            const view = render(
                <ThemeProvider theme={theme}>
                    <RouterProvider router={router} />
                </ThemeProvider>
            )
            try {
                await view.findByText("Route content")
                const color = getComputedStyle(
                    view.container.querySelector(".voting-portal-wrapper")!
                ).color
                if (expected) {
                    expect(color).toBe(expected)
                } else {
                    expect(color).not.toBe("rgb(1, 2, 3)")
                    expect(color).not.toBe("rgb(4, 5, 6)")
                }
            } finally {
                view.unmount()
                router.dispose()
            }
        }
    )
})
