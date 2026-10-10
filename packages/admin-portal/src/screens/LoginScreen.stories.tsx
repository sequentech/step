// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {LoginScreen} from "./LoginScreen"

const meta = {
    title: "Admin/Screens/LoginScreen",
    component: LoginScreen,
} satisfies Meta<typeof LoginScreen>
export default meta
type Story = StoryObj<typeof meta>

export const WaitingForSignIn: Story = {
    parameters: {
        expectedFailure: {
            reason: "The sign-in progress indicator has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
    },
}
