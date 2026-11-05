// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {DiffView} from "./DiffView"

let boundary: ReturnType<typeof graphqlBoundary>

const current = {name: "Council event", status: "draft"}
const modified = {name: "Council event", status: "published"}

/**
 * A publication longer than the story build's MAX_DIFF_LINES (500): its only
 * change is on line 593, so the truncated view shows none.
 */
const longPublication = (value: string) =>
    Object.fromEntries(
        Array.from({length: 600}, (_, index) => [
            `area_${String(index).padStart(3, "0")}`,
            index === 590 ? value : "unchanged",
        ])
    )

/** PublishGenerate passes null for both versions while the publication loads. */
const NOT_LOADED = null as unknown as Record<string, string>

const meta = {
    title: "Admin/Components/DiffView",
    component: DiffView,
    args: {
        currentTitle: "Current",
        diffTitle: "Changes",
        current,
        modify: modified,
        fetchAllPublishChanges: fn(async () => {}),
    },
    argTypes: {type: {control: "inline-radio", options: ["modify", "simple"]}},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <DiffView {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof DiffView>
export default meta
type Story = StoryObj<typeof meta>

const region = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("region", {name})

export const Changes: Story = {
    play: async ({canvasElement}) => {
        const before = await region(canvasElement, "Current")
        const removed = within(before).getByText(/"status": "draft"/)
        expect(getComputedStyle(removed).textDecorationLine).toBe("line-through")
        expect(within(before).queryByText(/"status": "published"/)).not.toBeInTheDocument()
        const after = await region(canvasElement, "Changes")
        const added = within(after).getByText(/"status": "published"/)
        expect(getComputedStyle(added).backgroundColor).toBe("rgb(67, 227, 161)")
        expect(within(after).queryByText(/"status": "draft"/)).not.toBeInTheDocument()
        await expect(within(after).getByText(/"name": "Council event"/)).toBeVisible()
    },
}

export const SimpleView: Story = {
    args: {type: "simple", current: modified, modify: modified},
    play: async ({canvasElement}) => {
        const view = await region(canvasElement, "Current")
        await expect(within(view).getByText(/"status": "published"/)).toBeVisible()
        expect(within(canvasElement).queryByRole("region", {name: "Changes"})).toBeNull()
        expect(within(canvasElement).queryByText("Changes")).toBeNull()
    },
}

export const Loading: Story = {
    args: {current: NOT_LOADED, modify: NOT_LOADED},
    parameters: {
        expectedFailure: {
            reason: "The loading spinner has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("progressbar")).toBeVisible()
        expect(canvas.queryByRole("region")).toBeNull()
    },
}

export const ShowAllChangesAfterConfirmation: Story = {
    args: {current: longPublication("before"), modify: longPublication("after")},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const after = await region(canvasElement, "Changes")
        expect(within(after).queryByText(/"area_590"/)).toBeNull()
        const [showMore] = canvas.getAllByRole("button", {name: "Show More"})
        await expect(showMore).toHaveAttribute("aria-expanded", "false")
        await userEvent.click(showMore)
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(
            dialog.getByText(
                "Rendering all changes might make the page unresponsive. Are you sure you want to continue?"
            )
        ).toBeVisible()
        expect(args.fetchAllPublishChanges).not.toHaveBeenCalled()
        await userEvent.click(dialog.getByRole("button", {name: "Confirm"}))
        await waitFor(() => expect(args.fetchAllPublishChanges).toHaveBeenCalledTimes(1))
        const added = await within(after).findByText(/"area_590": "after"/)
        expect(getComputedStyle(added).backgroundColor).toBe("rgb(67, 227, 161)")
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        const [showLess] = canvas.getAllByRole("button", {name: "Show Less"})
        await userEvent.click(showLess)
        await waitFor(() => expect(within(after).queryByText(/"area_590"/)).toBeNull())
        expect(canvas.getAllByRole("button", {name: "Show More"})).toHaveLength(2)
        expect(args.fetchAllPublishChanges).toHaveBeenCalledTimes(1)
    },
}

export const KeepTruncatedWhenCancelled: Story = {
    args: {current: longPublication("before"), modify: longPublication("after")},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const after = await region(canvasElement, "Changes")
        await userEvent.click(canvas.getAllByRole("button", {name: "Show More"})[0])
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        expect(args.fetchAllPublishChanges).not.toHaveBeenCalled()
        expect(within(after).queryByText(/"area_590"/)).toBeNull()
        expect(canvas.getAllByRole("button", {name: "Show More"})).toHaveLength(2)
    },
}
