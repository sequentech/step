/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../services/theme"
import ReviewChangesTable from "./ReviewChangesTable"

test("a review identifies each previous value and its replacement under named columns", () => {
    render(
        <ThemeProvider theme={theme}>
            <ReviewChangesTable
                title="Review voter changes"
                subtitle="Check the new contact details"
                fieldLabel="Field"
                currentValueLabel="Previous value"
                newValueLabel="New value"
                rows={[
                    {
                        field: "email",
                        label: "Email",
                        currentValue: "old@example.test",
                        newValue: "new@example.test",
                    },
                ]}
            />
        </ThemeProvider>
    )
    const table = within(screen.getByRole("table", {name: "Review voter changes"}))
    expect(table.getAllByRole("columnheader").map((cell) => cell.textContent)).toEqual([
        "Field",
        "Previous value",
        "New value",
    ])
    const row = within(table.getByRole("row", {name: "Email old@example.test new@example.test"}))
    expect(row.getByText("old@example.test").closest("del")).not.toBeNull()
    expect(row.getByText("new@example.test").closest("del")).toBeNull()
    expect(screen.getByText("Check the new contact details")).toBeVisible()
})
