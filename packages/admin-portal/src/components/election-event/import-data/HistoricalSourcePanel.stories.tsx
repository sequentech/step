// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, within} from "storybook/test"
import {HistoricalSourcePanel} from "./HistoricalSourcePanel"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"

let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Election event/Import data/HistoricalSourcePanel",
    component: HistoricalSourcePanel,
    beforeEach: () => {
        boundary = graphqlBoundary({})
        return () => {
            expect(boundary.unexpected).toEqual([])
        }
    },
    args: {
        disabled: false,
        api: {buildFromHistorical: fn()},
        onPrepare: fn(),
        onAccepted: fn(async () => {}),
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <HistoricalSourcePanel {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof HistoricalSourcePanel>
export default meta
type Story = StoryObj<typeof meta>
export const SelectBothLocalSources: Story = {
    play: async ({canvasElement, args}) => {
        const view = within(canvasElement)
        await expect(view.getByText(/Drop the SQL dump/)).toHaveTextContent("Drop the SQL dump")
        await expect(view.queryByRole("combobox")).not.toBeInTheDocument()
        await expect(args.api.buildFromHistorical).not.toHaveBeenCalled()
        await expect(args.onAccepted).not.toHaveBeenCalled()
    },
}
