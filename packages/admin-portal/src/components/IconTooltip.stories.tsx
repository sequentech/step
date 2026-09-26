// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {faCircleInfo} from "@fortawesome/free-solid-svg-icons"
import IconTooltip from "./IconTooltip"

const INFO = "Use at least 12 characters, with a number and a symbol."

const meta = {
    title: "Admin/Components/IconTooltip",
    component: IconTooltip,
    args: {icon: faCircleInfo, info: INFO},
    argTypes: {icon: {table: {disable: true}}},
    parameters: {
        expectedFailure: {
            reason: "The tooltip labels its icon box, a div without a role.",
            a11y: ["aria-prohibited-attr"],
        },
    },
} satisfies Meta<typeof IconTooltip>
export default meta
type Story = StoryObj<typeof meta>

const trigger = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText(INFO, {selector: "div"})

export const Closed: Story = {
    play: async ({canvasElement}) => {
        await expect(trigger(canvasElement).querySelector("svg")).toBeVisible()
        expect(within(document.body).queryByRole("tooltip")).not.toBeInTheDocument()
    },
}

export const HoverShowsTheInformation: Story = {
    play: async ({canvasElement}) => {
        await userEvent.hover(trigger(canvasElement))
        const tooltip = await within(document.body).findByRole("tooltip")
        await waitFor(() => expect(tooltip).toBeVisible())
        expect(tooltip).toHaveTextContent(INFO)
        await userEvent.unhover(trigger(canvasElement))
        await waitFor(() =>
            expect(within(document.body).queryByRole("tooltip")).not.toBeInTheDocument()
        )
    },
}
