// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {StoryFn, Meta} from "@storybook/react-vite"
import Box from "@mui/material/Box"
import {expect, userEvent, within} from "storybook/test"
import AccessibilityMenu from "../AccessibilityMenu"

export default {
    title: "components/AccessibilityMenu",
    component: AccessibilityMenu,
    parameters: {
        backgrounds: {
            default: "light",
        },
    },
} as Meta<typeof AccessibilityMenu>

const Template: StoryFn<typeof AccessibilityMenu> = () => (
    <Box style={{display: "inline-flex", backgroundColor: "white"}}>
        <AccessibilityMenu />
    </Box>
)

export const Closed = Template.bind({})

export const Open = Template.bind({})
Open.play = async ({canvasElement}) => {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Accessibility"}))
    const dialog = await within(document.body).findByRole("dialog", {
        name: "Accessibility settings",
    })
    await userEvent.click(within(dialog).getByRole("radio", {name: "Large"}))
    await expect(document.documentElement).toHaveAttribute("data-a11y-text-size", "large")
    await userEvent.click(within(dialog).getByRole("button", {name: "Reset settings"}))
    await expect(document.documentElement).not.toHaveAttribute("data-a11y-text-size")
}
