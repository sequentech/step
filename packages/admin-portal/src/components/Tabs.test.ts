/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {cleanup, fireEvent, render, screen} from "@testing-library/react"
import {Tabs} from "./Tabs"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))

const elements = [
    {label: "Data", component: () => React.createElement("div", null, "Data panel")},
    {
        label: "Tally sheet imports",
        component: () => React.createElement("div", null, "Imports panel"),
    },
    {
        label: "Scheduled events",
        component: () => React.createElement("div", null, "Events panel"),
    },
]

afterEach(cleanup)

describe("Tabs", () => {
    it("constrains the scrollable tab bar to the available container width", () => {
        render(React.createElement(Tabs, {elements}))

        const tabBar = screen.getByRole("tablist").closest(".MuiTabs-root")
        expect(tabBar).not.toBeNull()
        expect(window.getComputedStyle(tabBar!.parentElement!).width).toBe("100%")
        expect(tabBar!.querySelector(".MuiTabs-scrollableX")).not.toBeNull()
    })

    it("keeps tab selection and panel rendering working", () => {
        const onSelectedTabChange = jest.fn()
        render(React.createElement(Tabs, {elements, onSelectedTabChange}))

        expect(screen.getByText("Data panel")).toBeTruthy()
        fireEvent.click(screen.getByRole("tab", {name: "Scheduled events"}))

        expect(onSelectedTabChange).toHaveBeenCalledWith(2)
        expect(
            screen.getByRole("tab", {name: "Scheduled events"}).getAttribute("aria-selected")
        ).toBe("true")
        expect(screen.getByText("Events panel")).toBeTruthy()
        expect(screen.queryByText("Data panel")).toBeNull()
    })
})
