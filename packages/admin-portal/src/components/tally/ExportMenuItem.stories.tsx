// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MenuList} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {EExportFormat} from "@/types/results"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {RESULT_DOCUMENTS} from "@/resources/Tally/__stories__/TallyFixture"
import {ExportMenuItem} from "./ExportMenuItem"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Tally/ExportMenuItem",
    component: ExportMenuItem,
    args: {
        documents: {documents: RESULT_DOCUMENTS.election, name: "Council", class_type: "election"},
        className: "tally-document-item pdf election",
        formatValue: EExportFormat.PDF,
        formatLabel: "PDF",
        handleExport: fn(),
        handleClose: fn(),
    },
    argTypes: {
        formatValue: {control: "select", options: Object.values(EExportFormat)},
        documents: {table: {disable: true}},
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    // The item is an entry of the results' export menu.
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MenuList aria-label="Export results">
                <ExportMenuItem {...args} />
            </MenuList>
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof ExportMenuItem>
export default meta
type Story = StoryObj<typeof meta>

const exportLabel = (format: string) =>
    i18n.t("common.label.exportFormat", {item: "Council", format})

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const item = within(canvasElement).getByRole("menuitem", {name: exportLabel("PDF")})
        await expect(item).toBeVisible()
        expect(item).toHaveClass("tally-document-item", "pdf", "election")
        expect(args.handleExport).not.toHaveBeenCalled()
    },
}

export const ExportClosesTheMenu: Story = {
    args: {formatValue: EExportFormat.JSON, formatLabel: "JSON"},
    play: async ({canvasElement, args}) => {
        await userEvent.click(
            within(canvasElement).getByRole("menuitem", {name: exportLabel("JSON")})
        )
        expect(args.handleExport).toHaveBeenCalledTimes(1)
        expect(args.handleExport).toHaveBeenCalledWith(
            RESULT_DOCUMENTS.election,
            EExportFormat.JSON
        )
        // The menu closes on the next tick, after the export has started.
        await waitFor(() => expect(args.handleClose).toHaveBeenCalledTimes(1))
    },
}

export const CustomLabel: Story = {
    args: {
        formatValue: EExportFormat.ALL_AREAS_JSON,
        formatLabel: "JSON",
        label: "Export All Areas Results in JSON format for 'Council'",
    },
    play: async ({canvasElement, args}) => {
        const item = within(canvasElement).getByRole("menuitem", {
            name: "Export All Areas Results in JSON format for 'Council'",
        })
        await userEvent.click(item)
        expect(args.handleExport).toHaveBeenCalledWith(
            RESULT_DOCUMENTS.election,
            EExportFormat.ALL_AREAS_JSON
        )
    },
}
