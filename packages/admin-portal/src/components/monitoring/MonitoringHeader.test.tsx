/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {MonitoringHeader} from "./MonitoringHeader"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

function header(refreshMs: number) {
    render(
        <MonitoringHeader
            title="Overview"
            dashboards={[{id: "overview", title: "Overview", requirements: [], widget_count: 1}]}
            dashboardId="overview"
            onSelectDashboard={jest.fn()}
            widgetCount={1}
            requirements={[]}
            snapshot={null}
            timeZone="UTC"
            refreshMs={refreshMs}
        />
    )
}

describe("MonitoringHeader", () => {
    it("says how often the dashboard asks for new figures", () => {
        header(90_000)
        expect(screen.getByText(/monitoring\.header\.refresh \{"seconds":90\}/)).toBeInTheDocument()
    })

    it("shows the default interval when given it", () => {
        header(30_000)
        expect(screen.getByText(/monitoring\.header\.refresh \{"seconds":30\}/)).toBeInTheDocument()
    })
})
