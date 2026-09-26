// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_TRUSTEE} from "@/__stories__/storyAuth"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {StartStep} from "./StartStep"

interface Scenario {
    goNext: () => void
    goBack: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Keys ceremony/StartStep",
    component: StartStep,
    args: {goNext: fn(), goBack: fn()},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql} role={EStoryPermissions.TRUSTEE}>
            <StartStep {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Trustee Key Ceremony"})).toBeVisible()
        // The signed-in trustee is named in bold.
        await expect(canvas.getByText(STORY_TRUSTEE, {selector: "strong"})).toBeVisible()
        expect(canvas.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
            "Download your Encrypted Private Key.",
            "Create multiple Backups of the Encrypted Private Key.",
            "Check that the backups works well.",
        ])
        expect(args.goNext).not.toHaveBeenCalled()
        expect(graphql.calls).toEqual([])
    },
}

export const Next: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Next"}))
        expect(args.goNext).toHaveBeenCalledTimes(1)
        expect(args.goBack).not.toHaveBeenCalled()
    },
}

export const Back: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Back"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
        expect(args.goNext).not.toHaveBeenCalled()
    },
}
