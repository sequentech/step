// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {act, render} from "@testing-library/react"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import useUpdateTranslation from "../hooks/useUpdateTranslation"
import TenantEvent from "./TenantEvent"

jest.mock("../store/hooks", () => ({
    useAppSelector: jest.fn(() => ({id: "event-a", presentation: {}})),
}))
jest.mock("../providers/SettingsContextProvider", () => {
    const ReactActual = jest.requireActual<typeof React>("react")
    return {
        SettingsContext: ReactActual.createContext({
            defaultLanguageTouched: false,
            setDefaultLanguageTouched: jest.fn(),
        }),
    }
})
jest.mock("../hooks/useUpdateTranslation", () => ({
    __esModule: true,
    default: jest.fn(),
}))

describe("TenantEvent translation ownership", () => {
    it("keeps the event translation owner mounted across child-route navigation", async () => {
        let mounts = 0
        let cleanups = 0
        const updateTranslationMock = useUpdateTranslation as jest.MockedFunction<
            typeof useUpdateTranslation
        >
        updateTranslationMock.mockImplementation(() => {
            useEffect(() => {
                mounts += 1
                return () => {
                    cleanups += 1
                }
            }, [])
            return {}
        })

        const router = createMemoryRouter(
            [
                {
                    path: "/tenant/:tenantId/event/:eventId",
                    element: React.createElement(TenantEvent),
                    children: [
                        {
                            path: "election-chooser",
                            element: React.createElement("div", null, "Chooser"),
                        },
                        {
                            path: "election/election-a/start",
                            element: React.createElement("div", null, "Start"),
                        },
                    ],
                },
            ],
            {initialEntries: ["/tenant/tenant-a/event/event-a/election-chooser"]}
        )
        const view = render(React.createElement(RouterProvider, {router}))

        expect(mounts).toBe(1)
        expect(cleanups).toBe(0)

        await act(async () => {
            await router.navigate("/tenant/tenant-a/event/event-a/election/election-a/start")
        })

        expect(mounts).toBe(1)
        expect(cleanups).toBe(0)

        view.unmount()
        expect(cleanups).toBe(1)
    })
})
