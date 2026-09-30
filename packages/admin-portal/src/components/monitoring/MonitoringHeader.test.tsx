/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {MonitoringHeader, type MonitoringHeaderProps} from "./MonitoringHeader"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

function header(refreshMs: number, snapshot: MonitoringHeaderProps["snapshot"] = null) {
    render(
        <MonitoringHeader
            presetTitle="COMELEC overseas voting"
            dashboards={[{id: "overview", title: "Overview", requirements: [], widget_count: 1}]}
            dashboardId="overview"
            onSelectDashboard={jest.fn()}
            snapshot={snapshot}
            timeZone="Asia/Manila"
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

    it("names the preset the dashboards came from", () => {
        header(30_000)
        expect(screen.getByText("COMELEC overseas voting")).toBeInTheDocument()
        expect(screen.getByText("monitoring.header.preset")).toBeInTheDocument()
    })

    it("names the time zone the update time is shown in", () => {
        header(30_000, {revision: 4, as_of: "2026-09-30T02:00:00Z"})
        expect(
            screen.getByText(
                /monitoring\.header\.updated \{"time":"[^"]*10:00[^"]*","timeZone":"Asia\/Manila"\}/
            )
        ).toBeInTheDocument()
    })
})
