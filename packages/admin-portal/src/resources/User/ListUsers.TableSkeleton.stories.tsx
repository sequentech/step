// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect} from "storybook/test"
import {TableSkeleton} from "./ListUsers"

/** The placeholder rows, one flex box of text skeletons each. */
const rows = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll(".MuiBox-root > .MuiBox-root"))

const meta = {
    title: "Admin/User/TableSkeleton",
    component: TableSkeleton,
    args: {rowCount: 3},
} satisfies Meta<typeof TableSkeleton>
export default meta
type Story = StoryObj<typeof meta>

export const LoadingRows: Story = {
    play: async ({canvasElement}) => {
        expect(rows(canvasElement)).toHaveLength(3)
        // Six columns by default, like the voter list's first columns.
        expect(rows(canvasElement)[0].querySelectorAll(".MuiSkeleton-text")).toHaveLength(6)
    },
}

export const CustomColumns: Story = {
    args: {rowCount: 2, columnWidths: ["50%", "50%"]},
    play: async ({canvasElement}) => {
        expect(rows(canvasElement)).toHaveLength(2)
        for (const row of rows(canvasElement)) {
            expect(row.querySelectorAll(".MuiSkeleton-text")).toHaveLength(2)
        }
    },
}

export const DefaultPageOfRows: Story = {
    args: {rowCount: undefined},
    play: async ({canvasElement}) => {
        expect(rows(canvasElement)).toHaveLength(10)
    },
}
