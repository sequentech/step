// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render} from "@testing-library/react"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {PortalChrome} from "./PortalChrome"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Footer: () => null,
        Header: () => null,
        PageBanner: ({children}: {children: React.ReactNode}) => <main>{children}</main>,
    }),
    {virtual: true}
)
jest.mock("@sequentech/ui-essentials/public/Sequent_logo.svg", () => "logo", {virtual: true})
jest.mock("@sequentech/ui-essentials/public/blank_logo.svg", () => "logo", {virtual: true})
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthenticated: true,
        getExpiry: () => undefined,
    }),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("./WaterMark/Watermark", () => ({__esModule: true, default: () => null}))
jest.mock("./ScreenAudioInstructions/ScreenAudioInstructions", () => ({
    ScreenAudioInstructions: () => null,
}))
jest.mock("./BallotSelectionAdapter", () => ({
    BallotSelectionAdapter: ({children}: {children: React.ReactNode}) => <>{children}</>,
}))
jest.mock("../hooks/useElectionClassName", () => ({useElectionClassName: () => []}))
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: unknown) => unknown) => selector(mockState),
    useAppDispatch: () => jest.fn(),
}))

let mockState: unknown

const renderChrome = () =>
    render(
        <MemoryRouter initialEntries={["/tenant/tenant/event/event"]}>
            <Routes>
                <Route
                    path="/tenant/:tenantId/event/:eventId"
                    element={
                        <PortalChrome>
                            <div>Route content</div>
                        </PortalChrome>
                    }
                />
            </Routes>
        </MemoryRouter>
    )

const stateWithCss = (css: string) => ({
    elections: {first: {id: "first", presentation: {}}},
    electionEvent: {event: {id: "event", presentation: {css: "color: rgb(1, 2, 3);"}}},
    ballotStyles: {
        first: {
            id: "first",
            election_id: "first",
            ballot_eml: {election_event_presentation: {css}},
        },
    },
})

test("leaves images from other origins out of the event styling", () => {
    mockState = stateWithCss(
        "color: rgb(4, 5, 6); background-image: url(https://elsewhere.example/image.png);"
    )
    const view = renderChrome()
    try {
        const style = getComputedStyle(view.container.querySelector(".voting-portal-wrapper")!)
        expect(style.color).toBe("rgb(4, 5, 6)")
        expect(style.backgroundImage).not.toContain("elsewhere.example")
    } finally {
        view.unmount()
    }
})
