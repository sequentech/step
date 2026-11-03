// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, spyOn, within} from "storybook/test"
import {HistoricalImportPlugin} from "./HistoricalImportPlugin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"

type Props = React.ComponentProps<typeof HistoricalImportPlugin> & {outcome: "ready" | "abi-error"}
let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Election event/Import data/HistoricalImportPlugin",
    component: HistoricalImportPlugin,
    args: {
        disabled: false,
        baseUrl: "/synthetic-converter",
        onPrepare: fn(),
        onAccepted: fn(async () => {}),
        outcome: "ready",
    },
    beforeEach: ({args}) => {
        boundary = graphqlBoundary({})
        // A code-only worker boundary: stories never fetch plugin or source files.
        const mock = spyOn(window, "Worker").mockImplementation(() => {
            const worker = {
                onmessage: null as Worker["onmessage"],
                onerror: null as Worker["onerror"],
                terminate: fn(),
                postMessage: (data: {id: number}) => {
                    queueMicrotask(() =>
                        worker.onmessage?.call(
                            worker as unknown as Worker,
                            {
                                data: {
                                    id: data.id,
                                    ...(args.outcome === "abi-error"
                                        ? {
                                              error: "Historical import converter has an incompatible ABI, profile or package version.",
                                          }
                                        : {output: null}),
                                },
                            } as MessageEvent
                        )
                    )
                },
            }
            return worker as unknown as Worker
        })
        return () => {
            try {
                expect(boundary.unexpected).toEqual([])
            } finally {
                mock.mockRestore()
            }
        }
    },
    render: ({outcome: _outcome, ...args}) => (
        <AdminStoryProvider boundary={boundary}>
            <HistoricalImportPlugin {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>
export const Configured: Story = {
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                "Drop the SQL dump into the same zone to complete the source pair."
            )
        ).toBeVisible()
    },
}
export const IncompatiblePlugin: Story = {
    args: {outcome: "abi-error"},
    play: async ({canvasElement}) => {
        const view = within(canvasElement)
        await expect(await view.findByRole("alert")).toHaveTextContent("incompatible ABI")
        await expect(view.queryByLabelText("Historical SQL")).not.toBeInTheDocument()
    },
}
export const Unconfigured: Story = {
    args: {baseUrl: undefined},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).queryByLabelText("Historical SQL")
        ).not.toBeInTheDocument()
    },
}
