// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {SyncDiffTable} from "./SyncDiffTable"
import {ESyncChangeCategory, type SyncDiffRow} from "./types"

const row = (voterId: string, extra: Partial<SyncDiffRow> = {}): SyncDiffRow => ({
    id: `${voterId}:Ward:0:0`,
    voterId,
    field: "Ward",
    label: "Ward",
    oldValue: "North",
    newValue: "South",
    category: ESyncChangeCategory.PROFILE_UPDATE,
    target: "sequent",
    ...extra,
})

const rows: SyncDiffRow[] = [
    row("V-1001"),
    row("V-1002", {
        id: "V-1002:Channel:1:0",
        field: "Channel",
        label: "Channel",
        oldValue: "NONE",
        newValue: "PAPER",
        category: ESyncChangeCategory.VOTED_OTHER_CHANNEL,
    }),
]

const meta = {
    title: "Admin/Voter list sync/SyncDiffTable",
    component: SyncDiffTable,
    args: {rows},
    parameters: {
        expectedFailure: {
            reason: "Struck-through current values are dimmed below the contrast minimum, and white chip labels lack contrast on warning colours.",
            a11y: ["color-contrast"],
        },
    },
} satisfies Meta<typeof SyncDiffTable>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const grid = within(await within(canvasElement).findByRole("grid"))
        const channel = within(grid.getByRole("row", {name: /V-1002/}))
        await expect(
            channel.getByText(i18n.t("reconciliation.categories.VOTED_OTHER_CHANNEL"))
        ).toBeVisible()
        // The current value is struck through, the new one is plain.
        expect(channel.getByText("NONE").tagName).toBe("DEL")
        expect(channel.getByText("PAPER").tagName).not.toBe("DEL")
        expect(grid.getAllByRole("row")).toHaveLength(rows.length + 1)
    },
}

export const RowFailureReason: Story = {
    args: {
        rows: [
            row("V-2001", {
                id: "failure:V-2001:0:0",
                label: i18n.t("reconciliation.table.rowLabel"),
                oldValue: "NONE",
                newValue: "NONE",
                category: ESyncChangeCategory.ROW_FAILURE,
                failureReason: "CountyMun mismatch",
            }),
        ],
    },
    play: async ({canvasElement}) => {
        const grid = within(await within(canvasElement).findByRole("grid"))
        await expect(grid.getByText("CountyMun mismatch")).toBeVisible()
    },
}

export const PagesLongDiffs: Story = {
    args: {
        rows: Array.from({length: 12}, (_, index) =>
            row(`V-3${String(index).padStart(3, "0")}`, {id: `V-3${index}:Ward:${index}:0`})
        ),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const grid = within(await canvas.findByRole("grid"))
        expect(grid.queryByText("V-3011")).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Go to next page"}))
        await expect(await grid.findByText("V-3011")).toBeVisible()
    },
}

export const NoDifferences: Story = {
    args: {rows: []},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("reconciliation.table.noDifferences"))).toBeVisible()
        expect(canvas.queryByRole("grid")).toBeNull()
    },
}

export const CustomEmptyMessage: Story = {
    args: {rows: [], emptyMessage: "No external-side differences."},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No external-side differences.")).toBeVisible()
    },
}
