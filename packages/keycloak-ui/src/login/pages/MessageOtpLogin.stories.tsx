// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"
import {MessageCourier} from "../KcContext"

const {KcPageStory} = createKcPageStory({pageId: "message-otp.login.ftl"})

const meta = {
    title: "Keycloak/Message OTP",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

export const Email: Story = {
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Enter the code we sent to your email.")).toBeVisible()
        await userEvent.type(canvas.getByLabelText("Digit 1 of 6"), "123456")
        await expect(canvas.getByLabelText("Digit 6 of 6")).toHaveFocus()
        const form = canvas.getByLabelText("Digit 1 of 6").closest("form")!
        await expect(new FormData(form).get("code")).toBe("123456")
    },
}

export const Sms: Story = {
    args: {kcContext: {courier: MessageCourier.Sms}},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await expect(
            within(canvasElement).getByText("Enter the code we sent to your mobile device via sms.")
        ).toBeVisible()
    },
}

export const InvalidCode: Story = {
    args: {
        kcContext: {
            codeJustSent: false,
            message: {type: "error", summary: "Invalid OTP code."},
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("alert")).toHaveTextContent("Invalid OTP code.")
        await expect(canvas.getByLabelText("Digit 1 of 6")).toBeVisible()
    },
}

export const OneTimeLink: Story = {
    args: {kcContext: {isOtl: true, courier: MessageCourier.Email}},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        await expect(canvas.queryByLabelText("Digit 1 of 6")).not.toBeInTheDocument()
        const resend = canvas.getByRole("button", {
            name: "Didn't receive the link yet? Click here to resend",
        })
        await expect(resend).toBeEnabled()
        await expect(resend).toHaveAttribute("name", "resend")
        await expect(resend).toHaveAttribute("value", "true")
        await expect(resend).toHaveAttribute("formnovalidate")
    },
}

export const ResendCountdown: Story = {
    args: {kcContext: {codeJustSent: true}},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await expect(
            within(canvasElement).getByRole("button", {name: /Wait \d+ seconds to resend/})
        ).toBeDisabled()
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText("Ingrese el código que le enviamos a su email.")
        ).toBeVisible()
        await expect(canvas.getByLabelText("Dígito 1 de 6")).toBeVisible()
    },
}
