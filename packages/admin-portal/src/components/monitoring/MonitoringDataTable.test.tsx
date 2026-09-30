/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import englishTranslation from "@/translations/en"
import {MonitoringDataTable, DATA_TABLE_PAGE_SIZE} from "./MonitoringDataTable"
import {MonitoringDataTableDialog} from "./MonitoringDataTableDialog"
import {EColumnKind, type MonitoringTable} from "./types"

/** The English words, with `{{name}}` filled in, as the portal shows them. */
function mockTranslate(key: string, options?: Record<string, unknown>): string {
    const found = key
        .split(".")
        .reduce<unknown>(
            (node, part) =>
                typeof node === "object" && node !== null
                    ? (node as Record<string, unknown>)[part]
                    : undefined,
            englishTranslation.translations
        )
    if (typeof found !== "string") return key
    return found.replace(/{{\s*(\w+)\s*}}/g, (_match, name: string) =>
        String(options?.[name] ?? "")
    )
}

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: mockTranslate,
        i18n: {language: "en", exists: () => false},
    }),
}))

/** `count` rows of a bucketed series: a time label, two counts and a ratio. */
function bigTable(count: number): MonitoringTable {
    return {
        columns: [
            {name: "bucket", kind: EColumnKind.TEXT},
            {name: "voted", kind: EColumnKind.INTEGER},
            {name: "registered", kind: EColumnKind.INTEGER},
            {name: "pct", kind: EColumnKind.NUMBER},
            {name: "position", kind: EColumnKind.INTEGER},
        ],
        rows: Array.from({length: count}, (_, index) => [
            `row-${index + 1}`,
            index * 3,
            index * 5 + 1,
            (index * 3) / (index * 5 + 1),
            index + 1,
        ]),
    }
}

const bodyRows = () => within(screen.getByRole("table")).getAllByRole("row").slice(1)
const firstCell = (row: HTMLElement) => within(row).getAllByRole("cell")[0].textContent

describe("MonitoringDataTable at scale", () => {
    it.each([
        [1_000, "1,000"],
        [10_000, "10,000"],
    ])("renders %i rows a page at a time", (count, total) => {
        const table = bigTable(count)
        const started = performance.now()
        const {unmount} = render(<MonitoringDataTable table={table} caption="Big · data" />)
        const renderMs = performance.now() - started
        process.stdout.write(`[scale] rows=${count} render_ms=${renderMs.toFixed(1)}\n`)
        // Only the first page is in the document, not every row.
        expect(DATA_TABLE_PAGE_SIZE).toBe(100)
        expect(bodyRows()).toHaveLength(100)
        expect(firstCell(bodyRows()[0])).toBe("row-1")
        expect(screen.getByText(`1–100 of ${total}`)).toBeInTheDocument()
        unmount()
    })

    it("pages forward, back, to the end, and changes the page size", () => {
        render(<MonitoringDataTable table={bigTable(1_050)} caption="Big · data" />)
        fireEvent.click(screen.getByRole("button", {name: "Next page"}))
        expect(firstCell(bodyRows()[0])).toBe("row-101")
        expect(screen.getByText("101–200 of 1,050")).toBeInTheDocument()
        fireEvent.click(screen.getByRole("button", {name: "Last page"}))
        // The last page holds what is left.
        expect(bodyRows()).toHaveLength(50)
        expect(firstCell(bodyRows()[49])).toBe("row-1050")
        fireEvent.click(screen.getByRole("button", {name: "Previous page"}))
        expect(firstCell(bodyRows()[0])).toBe("row-901")
        fireEvent.click(screen.getByRole("button", {name: "First page"}))
        expect(firstCell(bodyRows()[0])).toBe("row-1")

        fireEvent.mouseDown(screen.getByRole("combobox"))
        fireEvent.click(screen.getByRole("option", {name: "250"}))
        expect(bodyRows()).toHaveLength(250)
        expect(screen.getByText("1–250 of 1,050")).toBeInTheDocument()
        expect(screen.getByText("Rows per page:")).toBeInTheDocument()
    })

    it("stays on a page that exists when a refresh brings fewer rows", () => {
        const {rerender} = render(
            <MonitoringDataTable table={bigTable(1_000)} caption="Big · data" />
        )
        fireEvent.click(screen.getByRole("button", {name: "Last page"}))
        expect(firstCell(bodyRows()[0])).toBe("row-901")
        rerender(<MonitoringDataTable table={bigTable(150)} caption="Big · data" />)
        // Page 10 is gone: the last page there is, not an empty table.
        expect(firstCell(bodyRows()[0])).toBe("row-101")
        expect(bodyRows()).toHaveLength(50)
    })

    it("shows a table of a page or less whole, with no pager", () => {
        render(
            <MonitoringDataTable table={bigTable(DATA_TABLE_PAGE_SIZE)} caption="Small · data" />
        )
        expect(bodyRows()).toHaveLength(DATA_TABLE_PAGE_SIZE)
        expect(screen.queryByRole("button", {name: "Next page"})).toBeNull()
        expect(screen.queryByText("Rows per page:")).toBeNull()
        // The renderer's ordering column is still left out.
        expect(screen.queryByRole("columnheader", {name: "position"})).toBeNull()
    })

    it("still says there are no rows", () => {
        render(<MonitoringDataTable table={bigTable(0)} caption="Empty · data" />)
        expect(screen.getByText("No rows")).toBeInTheDocument()
    })
})

describe("MonitoringDataTableDialog at scale", () => {
    it.each([
        [1_000, "1,000"],
        [10_000, "10,000"],
    ])("opens View data on %i rows", (count, total) => {
        const started = performance.now()
        const {unmount} = render(
            <MonitoringDataTableDialog
                open
                onClose={() => undefined}
                title="Turnout over time"
                scope="All Posts"
                table={bigTable(count)}
            />
        )
        const renderMs = performance.now() - started
        process.stdout.write(`[scale] dialog rows=${count} render_ms=${renderMs.toFixed(1)}\n`)
        const dialog = within(screen.getByRole("dialog"))
        expect(dialog.getAllByRole("row")).toHaveLength(DATA_TABLE_PAGE_SIZE + 1)
        expect(dialog.getByText(`1–100 of ${total}`)).toBeInTheDocument()
        unmount()
    })
})
