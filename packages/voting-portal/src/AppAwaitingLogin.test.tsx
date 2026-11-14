// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, render, waitFor} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import theme from "../../ui-essentials/src/services/theme"
import {store} from "./store/store"
import {IElectionEventPresentation} from "@sequentech/ui-core"
import {seedElectionEvent} from "./store/electionEvents/electionEventsSlice"
import App from "./App"

const mockLoadConfig = jest.fn()
const mockDispatch = jest.fn()

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
        Footer: () => <footer>Default footer</footer>,
        Header: () => <header>Default header</header>,
        PageBanner: ({children}: React.PropsWithChildren) => <main>{children}</main>,
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-essentials/public/Sequent_logo.svg", () => "logo", {virtual: true})
jest.mock("@sequentech/ui-essentials/public/blank_logo.svg", () => "logo", {virtual: true})
jest.mock("./providers/AuthContextProvider", () => {
    const auth = {isAuthenticated: false, getExpiry: () => undefined, setTenantEvent: jest.fn()}
    return {AuthContext: require("react").createContext(auth), mockAuth: auth}
})
jest.mock("./providers/SettingsContextProvider", () => {
    const settings = {globalSettings: {DISABLE_AUTH: false, PUBLIC_BUCKET_URL: "http://bucket/"}}
    return {SettingsContext: require("react").createContext(settings), mockSettings: settings}
})
jest.mock("./providers/ApolloContextProvider", () => ({
    ApolloWrapper: ({children}: React.PropsWithChildren) => <>{children}</>,
}))
jest.mock("./components/WaterMark/Watermark", () => ({__esModule: true, default: () => null}))
jest.mock("./hooks/useElectionClassName", () => ({useElectionClassName: () => []}))
jest.mock("./services/ElectionEventConfig", () => ({
    createElectionEventConfigLoader: () => mockLoadConfig,
}))
jest.mock("./store/hooks", () => ({
    useAppSelector: (selector: (state: unknown) => unknown) => selector(mockState),
    useAppDispatch: () => mockDispatch,
}))
const {mockAuth} = jest.requireMock("./providers/AuthContextProvider")
const {mockSettings} = jest.requireMock("./providers/SettingsContextProvider")

let mockState: unknown
const eventCss = "color: rgb(1, 2, 3);"
const eventConfig = {
    id: "config",
    tenant_id: "tenant",
    election_event_id: "event",
    election_event_presentation: {css: eventCss} as IElectionEventPresentation,
}

const renderApp = () => {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId",
                element: <App />,
                children: [{path: "login", element: <div>Route content</div>}],
            },
        ],
        {initialEntries: ["/tenant/tenant/event/event/login"]}
    )
    const view = render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
    return {
        view,
        dispose: () => {
            view.unmount()
            router.dispose()
        },
    }
}

beforeEach(() => {
    jest.spyOn(window, "scrollTo").mockImplementation(() => {})
    jest.spyOn(console, "error").mockImplementation(() => {})
    mockAuth.isAuthenticated = false
    mockSettings.globalSettings.DISABLE_AUTH = false
    mockAuth.setTenantEvent.mockReset()
    mockLoadConfig.mockReset()
    mockDispatch.mockReset()
    mockLoadConfig.mockReturnValue(new Promise(() => {}))
    mockState = {
        ...store.getState(),
        electionEvent: {event: {id: "event", presentation: {css: eventCss}}},
    }
})

afterEach(() => {
    jest.restoreAllMocks()
})

describe("event routes before the voter is logged in", () => {
    test("show only a neutral loader, styled by the event when it is known", () => {
        const {view, dispose} = renderApp()
        try {
            expect(view.container.querySelector(".neutral-loader")).not.toBeNull()
            expect(view.queryByText("Default header")).toBeNull()
            expect(view.queryByText("Default footer")).toBeNull()
            expect(view.queryByText("Route content")).toBeNull()
            const wrapper = view.container.querySelector(".voting-portal-wrapper")!
            expect(getComputedStyle(wrapper).color).toBe("rgb(1, 2, 3)")
        } finally {
            dispose()
        }
    })

    test("stay neutral when the event configuration cannot be loaded", async () => {
        mockLoadConfig.mockRejectedValue(new Error("HTTP 500"))
        mockState = {...store.getState(), electionEvent: {}}
        const {view, dispose} = renderApp()
        try {
            await waitFor(() => expect(mockAuth.setTenantEvent).toHaveBeenCalled())
            expect(view.container.querySelector(".neutral-loader")).not.toBeNull()
            expect(view.queryByText("Default header")).toBeNull()
            expect(view.queryByText("Default footer")).toBeNull()
        } finally {
            dispose()
        }
    })

    test("the neutral loader does not use the default theme colour", () => {
        const {view, dispose} = renderApp()
        try {
            const loader = view.container.querySelector(".neutral-loader")!
            expect(getComputedStyle(loader).color).not.toBe(theme.palette.primary.main)
            expect(view.container.querySelector(".MuiCircularProgress-colorPrimary")).toBeNull()
        } finally {
            dispose()
        }
    })
})

describe("event routes for a voter who comes back logged in", () => {
    test("stay neutral until the event configuration has been looked up", async () => {
        mockAuth.isAuthenticated = true
        let resolveConfig: (config: unknown) => void = () => {}
        mockLoadConfig.mockReturnValue(new Promise((resolve) => (resolveConfig = resolve)))
        const {view, dispose} = renderApp()
        try {
            expect(view.container.querySelector(".neutral-loader")).not.toBeNull()
            expect(view.queryByText("Default header")).toBeNull()
            expect(view.queryByText("Default footer")).toBeNull()
            await act(async () => resolveConfig(eventConfig))
            await view.findByText("Route content")
            expect(mockDispatch).toHaveBeenCalledWith(
                seedElectionEvent({
                    id: "event",
                    tenant_id: "tenant",
                    presentation: eventConfig.election_event_presentation,
                })
            )
        } finally {
            dispose()
        }
    })

    test("show the page without the platform footer when the event configuration cannot be loaded", async () => {
        mockAuth.isAuthenticated = true
        mockState = store.getState()
        mockLoadConfig.mockRejectedValue(new Error("HTTP 500"))
        const {view, dispose} = renderApp()
        try {
            await view.findByText("Route content")
            expect(view.container.querySelector(".neutral-loader")).toBeNull()
            expect(view.queryByText("Default footer")).toBeNull()
        } finally {
            dispose()
        }
    })
})

describe("event routes once the page can be shown", () => {
    test("render the full page for a logged-in voter", async () => {
        mockAuth.isAuthenticated = true
        mockLoadConfig.mockResolvedValue(eventConfig)
        const {view, dispose} = renderApp()
        try {
            await view.findByText("Route content")
            expect(view.queryByText("Default header")).not.toBeNull()
            expect(view.queryByText("Default footer")).not.toBeNull()
            expect(view.container.querySelector(".neutral-loader")).toBeNull()
        } finally {
            dispose()
        }
    })

    test("leave out the platform footer when the event hides the platform branding", async () => {
        mockAuth.isAuthenticated = true
        mockLoadConfig.mockResolvedValue(eventConfig)
        mockState = {
            ...store.getState(),
            electionEvent: {
                event: {
                    id: "event",
                    presentation: {css: eventCss, platform_branding_policy: "hide"},
                },
            },
        }
        const {view, dispose} = renderApp()
        try {
            await view.findByText("Route content")
            expect(view.queryByText("Default header")).not.toBeNull()
            expect(view.queryByText("Default footer")).toBeNull()
            expect(
                view.container.querySelector(".voting-portal-wrapper.platform-branding-hidden")
            ).not.toBeNull()
        } finally {
            dispose()
        }
    })

    test("render the full page when authentication is disabled", async () => {
        mockSettings.globalSettings.DISABLE_AUTH = true
        mockLoadConfig.mockResolvedValue(eventConfig)
        const {view, dispose} = renderApp()
        try {
            await view.findByText("Route content")
            expect(view.container.querySelector(".neutral-loader")).toBeNull()
        } finally {
            dispose()
        }
    })
})
