// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"

const {KcPageStory} = createKcPageStory({pageId: "scanovate-error.ftl"})

const meta = {
    title: "Keycloak/Scanovate error",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

export const Retryable: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "We couldn’t verify your ID"})
        ).toBeVisible()
        await expect(canvas.getByText("Step 3 of 4 · Verify identity")).toBeVisible()
        await expect(canvas.getByText("Keep all four corners inside the frame.")).toBeVisible()
        await expect(canvas.getByText("You can try 2 more times.")).toBeVisible()
        const retry = canvas.getByRole("button", {name: "Try again"})
        const form = retry.closest("form")!
        await expect(form).toHaveAttribute("method", "post")
        await expect(new FormData(form).get("action")).toBe("retry")
        await expect(canvas.getByText("Q5KWeXSFuWKwRTRuA3R1FkF7")).toBeVisible()
    },
}

export const LastAttempt: Story = {
    args: {
        kcContext: {error: "scanovateVerificationFailedError", attemptsLeft: 1},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText("You can try 1 more time.")).toBeVisible()
        await expect(
            canvas.getByText("Use your own ID, and check that it hasn’t expired.")
        ).toBeVisible()
    },
}

export const NotRetryable: Story = {
    args: {
        kcContext: {error: "scanovateMaxRetriesError", canRetry: false, attemptsLeft: 0},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {
                level: 1,
                name: "We couldn’t verify your identity",
            })
        ).toBeVisible()
        await expect(canvas.queryByRole("button", {name: "Try again"})).toBeNull()
        await expect(canvas.queryByText("Before you try again")).toBeNull()
        await expect(canvas.getByText("Q5KWeXSFuWKwRTRuA3R1FkF7")).toBeVisible()
    },
}

export const CopyReference: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await userEvent.click(canvas.getByRole("button", {name: "Copy support reference"}))
        await waitFor(() =>
            expect(canvas.getByRole("status")).toHaveTextContent(/Reference copied\.|Couldn’t copy/)
        )
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {
                level: 1,
                name: "No pudimos verificar su documento",
            })
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Volver a intentarlo"})).toBeVisible()
    },
}
