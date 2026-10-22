/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {MonitoringFooter} from "./MonitoringFooter"

// Exercise the real timezone formatter without loading the shared UI barrel.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
}))

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

describe("MonitoringFooter", () => {
    it("says what the figures are of and when they were counted, in the event's zone", () => {
        render(
            <MonitoringFooter
                scopeLabel="All regions · Dubai PCG"
                snapshot={{revision: 4, as_of: "2026-09-30T02:00:00Z"}}
                timeZone="Asia/Manila"
                requirements={["SW-F-0259", "SW-F-0371"]}
            />
        )
        expect(
            screen.getByText(
                /^All regions · Dubai PCG · monitoring\.footer\.dataThrough \{"time":"[^"]*10:00[^"]*","timeZone":"PhST"\}$/
            )
        ).toBeInTheDocument()
        expect(screen.getByText("SW-F-0259, SW-F-0371")).toBeInTheDocument()
    })

    it("says nothing is counted before the first update, and no requirements when none", () => {
        const {container} = render(
            <MonitoringFooter
                scopeLabel="All Posts"
                snapshot={null}
                timeZone="UTC"
                requirements={[]}
            />
        )
        expect(screen.getByText("All Posts · monitoring.header.notUpdated")).toBeInTheDocument()
        expect(container.querySelectorAll("p, span").length).toBe(1)
    })
})
