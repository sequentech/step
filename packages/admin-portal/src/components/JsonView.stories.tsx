// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {JsonView} from "./JsonView"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/JsonView",
    component: JsonView,
    args: {origin: {name: "Council event", elections: ["Council election"]}},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <JsonView {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof JsonView>
export default meta
type Story = StoryObj<typeof meta>

const preformatted = (canvasElement: HTMLElement) => {
    const pre = canvasElement.querySelector("pre")
    if (!pre) throw new Error("JsonView renders no preformatted text")
    return pre
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const pre = preformatted(canvasElement)
        await expect(pre).toBeVisible()
        // Eight spaces per level, as the component formats the value.
        expect(pre.textContent).toBe(
            [
                "{",
                '        "name": "Council event",',
                '        "elections": [',
                '                "Council election"',
                "        ]",
                "}",
            ].join("\n")
        )
    },
}

export const EmptyObject: Story = {
    args: {origin: {}},
    play: async ({canvasElement}) => {
        expect(preformatted(canvasElement).textContent).toBe("{}")
    },
}
