/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {MonitoringHeader, type MonitoringHeaderProps} from "./MonitoringHeader"

// The portal's top action button: a labelled button, as react-admin draws it.
jest.mock("react-admin", () => ({
    Button: ({
        label,
        onClick,
        disabled,
    }: {
        label: string
        onClick?: () => void
        disabled?: boolean
    }) => (
        <button onClick={onClick} disabled={disabled}>
            {label}
        </button>
    ),
}))

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))

function header(
    refreshMs: number,
    snapshot: MonitoringHeaderProps["snapshot"] = null,
    actions: Pick<MonitoringHeaderProps, "onExport" | "onEditDashboard"> = {}
) {
    render(
        <MonitoringHeader
            presetTitle="COMELEC overseas voting"
            dashboards={[{id: "overview", title: "Overview", requirements: [], widget_count: 1}]}
            dashboardId="overview"
            onSelectDashboard={jest.fn()}
            snapshot={snapshot}
            timeZone="Asia/Manila"
            refreshMs={refreshMs}
            {...actions}
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

    it("offers Export and Edit dashboard as the portal's top actions, in one group", () => {
        const onExport = jest.fn()
        const onEditDashboard = jest.fn()
        header(30_000, null, {onExport, onEditDashboard})
        const exportButton = screen.getByRole("button", {name: "monitoring.header.export"})
        const edit = screen.getByRole("button", {name: "monitoring.header.editDashboard"})
        expect(exportButton.parentElement).toHaveClass("list-actions")
        expect(edit.parentElement).toBe(exportButton.parentElement)
        fireEvent.click(exportButton)
        fireEvent.click(edit)
        expect(onExport).toHaveBeenCalled()
        expect(onEditDashboard).toHaveBeenCalled()
    })

    it("disables Export while there is nothing to export, and hides Edit from viewers", () => {
        header(30_000)
        expect(screen.getByRole("button", {name: "monitoring.header.export"})).toBeDisabled()
        expect(screen.queryByRole("button", {name: "monitoring.header.editDashboard"})).toBeNull()
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
