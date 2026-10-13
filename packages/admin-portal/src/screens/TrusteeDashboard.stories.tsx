// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within, userEvent} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TrusteeDashboard} from "./TrusteeDashboard"

type Scenario = Record<string, never>
const meta = {
    title: "Admin/Screens/TrusteeDashboard",
    component: TrusteeDashboard,
    render: () => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <TrusteeDashboard />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
export const NativeTrustee: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Trustee dashboard"})).toBeVisible()
        await expect(canvas.getByText(/trustee generate --trustee-config/)).toBeVisible()
        await expect(canvas.getByText(/trustee attest --request/)).toBeVisible()
    },
}

export const BrowserIdentity: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Generate browser keys"}))
        const signing = await canvas.findByRole("textbox", {name: "Signing public key"})
        await expect(signing).not.toHaveValue("")
        await expect(
            canvas.getByRole("textbox", {name: "Share encryption public key"})
        ).not.toHaveValue("")
        await expect(canvas.getByRole("button", {name: "Download private keys"})).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Lock keys"}))
        await expect(
            canvas.queryByRole("textbox", {name: "Signing public key"})
        ).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Download private keys"})).toBeDisabled()
    },
}
