/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {MonitoringSwitcher} from "./MonitoringSwitcher"
import type {MonitoringDashboardSummary} from "./types"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))

const dashboard = (id: string, title: string, section?: string): MonitoringDashboardSummary => ({
    id,
    title,
    section,
    requirements: [],
    widget_count: 1,
})

describe("MonitoringSwitcher", () => {
    it("lists each section's dashboards under its heading, in order", () => {
        const onChange = jest.fn()
        render(
            <MonitoringSwitcher
                dashboards={[
                    dashboard("overview", "Overview"),
                    dashboard("req-0249", "Enrollment decisions", "Enrollment"),
                    dashboard("req-0250", "Disapproval reasons", "Enrollment"),
                    dashboard("req-0259", "Voted vs registered", "Voter turnout"),
                ]}
                dashboardId="overview"
                onChange={onChange}
            />
        )
        fireEvent.mouseDown(screen.getByRole("combobox", {name: "monitoring.header.dashboard"}))
        const list = within(screen.getByRole("listbox"))
        const entries = list.getAllByRole("option").map((option) => option.textContent)
        expect(entries).toEqual([
            "Overview",
            "Enrollment decisions",
            "Disapproval reasons",
            "Voted vs registered",
        ])
        // Headings are not choices, and each is shown once.
        expect(list.getAllByText("Enrollment")).toHaveLength(1)
        expect(list.getByText("Voter turnout")).toBeInTheDocument()
        fireEvent.click(list.getByRole("option", {name: "Disapproval reasons"}))
        expect(onChange).toHaveBeenCalledWith("req-0250")
    })
})
