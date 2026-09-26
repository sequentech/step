// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, userEvent, waitFor, within} from "storybook/test"
import {createKcPageStory, getKcContextMock} from "../KcPageStory"
import {MessageCourier} from "../KcContext"
import KcPage from "../KcPage"

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
        const header = within(canvas.getByRole("banner"))
        await expect(header.getByText("0.0.0-preview")).toBeVisible()
        await expect(header.getByText("synthetic")).toBeVisible()
        await expect(canvas.getByText("Enter the code we sent to your email.")).toBeVisible()
        await expect(canvas.getByRole("group", {name: "Verification code"})).toBeVisible()
        await userEvent.click(canvas.getByLabelText("Digit 1 of 6"))
        await userEvent.tab({shift: true})
        await expect(header.getByRole("combobox", {name: "Languages"})).toHaveFocus()
        await userEvent.tab()
        await expect(canvas.getByLabelText("Digit 1 of 6")).toHaveFocus()
        await userEvent.type(canvas.getByLabelText("Digit 1 of 6"), "123456")
        await expect(canvas.getByRole("button", {name: "Submit"})).toHaveFocus()
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

export const MissingCourier: Story = {
    render: () => {
        const kcContext = getKcContextMock({pageId: "message-otp.login.ftl"})
        delete kcContext.courier
        return <KcPage kcContext={kcContext} />
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("Enter the code we sent to your mobile device via sms or email.")
        ).toBeVisible()
        await expect(canvas.getByLabelText("Digit 1 of 6")).toBeVisible()
    },
}

export const OneTimeLinkWithoutCourier: Story = {
    render: () => {
        const kcContext = getKcContextMock({
            pageId: "message-otp.login.ftl",
            overrides: {isOtl: true},
        })
        delete kcContext.courier
        return <KcPage kcContext={kcContext} />
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText(
                "We have sent you a verification link to your mobile device via email and/or SMS. Please open this link to continue."
            )
        ).toBeVisible()
        await expect(canvas.queryByLabelText("Digit 1 of 6")).not.toBeInTheDocument()
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
        for (let digit = 1; digit <= 6; digit += 1) {
            const input = canvas.getByLabelText(`Digit ${digit} of 6`)
            await expect(input).toHaveAttribute("aria-invalid", "true")
            await expect(input).toHaveAccessibleDescription(/Invalid OTP code\./)
        }
        await expect(canvas.getByLabelText("Digit 1 of 6")).toHaveFocus()
    },
}

export const OneTimeLink: Story = {
    args: {kcContext: {isOtl: true, courier: MessageCourier.Email}},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        await expect(canvas.queryByLabelText("Digit 1 of 6")).not.toBeInTheDocument()
        const resend = canvas.getByRole("button", {
            name: "Resend link",
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
            within(canvasElement).getByRole("button", {name: /Resend code in \d+ seconds/})
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
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("lang", "es")
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("dir", "ltr")
    },
}

export const PasteWholeCode: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const first = canvas.getByLabelText("Digit 1 of 6")
        await userEvent.click(first)
        await userEvent.paste(" 123456 ")
        for (let digit = 1; digit <= 6; digit += 1) {
            await expect(canvas.getByLabelText(`Digit ${digit} of 6`)).toHaveValue(String(digit))
        }
        await expect(new FormData(first.closest("form")!).get("code")).toBe("123456")
        await expect(canvas.getByRole("button", {name: "Submit"})).toHaveFocus()
    },
}

export const OneTimeCodeAutofill: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const first = canvas.getByLabelText("Digit 1 of 6")
        await expect(first).toHaveAttribute("autocomplete", "one-time-code")
        // A password manager inserts the complete code in a single input event.
        // Pasting alone does not exercise this different browser input path.
        fireEvent.input(first, {target: {value: "654321"}, inputType: "insertReplacementText"})
        await waitFor(() => expect(canvas.getByLabelText("Digit 6 of 6")).toHaveValue("1"))
        for (const [index, digit] of [..."654321"].entries()) {
            await expect(canvas.getByLabelText(`Digit ${index + 1} of 6`)).toHaveValue(digit)
        }
        await expect(new FormData(first.closest("form")!).get("code")).toBe("654321")
        await expect(canvas.getByRole("button", {name: "Submit"})).toHaveFocus()
    },
}

export const KeyboardCorrection: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const first = canvas.getByLabelText("Digit 1 of 6")
        const second = canvas.getByLabelText("Digit 2 of 6")
        const third = canvas.getByLabelText("Digit 3 of 6")
        await userEvent.type(first, "12")
        await expect(third).toHaveFocus()
        await userEvent.keyboard("{ArrowLeft}")
        await expect(second).toHaveFocus()
        await userEvent.keyboard("{ArrowRight}")
        await expect(third).toHaveFocus()
        await userEvent.keyboard("{Backspace}")
        await expect(second).toHaveFocus()
        await expect(second).toHaveValue("2")
        await userEvent.keyboard("{Backspace}")
        await expect(second).toHaveValue("")
        await expect(first).toHaveFocus()
        await expect(new FormData(first.closest("form")!).get("code")).toBe("1")
        await userEvent.keyboard("{ArrowLeft}")
        await expect(first).toHaveFocus()
        await userEvent.tab()
        await expect(second).toHaveFocus()
        await userEvent.click(canvas.getByLabelText("Digit 6 of 6"))
        await userEvent.keyboard("{ArrowRight}")
        await expect(canvas.getByRole("button", {name: "Submit"})).toHaveFocus()
    },
}

export const EightDigitCode: Story = {
    args: {kcContext: {codeLength: "8"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const first = canvas.getByLabelText("Digit 1 of 8")
        await userEvent.click(first)
        await userEvent.paste("12345678")
        await expect(canvas.getByLabelText("Digit 8 of 8")).toHaveValue("8")
        await expect(new FormData(first.closest("form")!).get("code")).toBe("12345678")
        await expect(canvas.getByRole("button", {name: "Submit"})).toHaveFocus()
    },
}

export const StatusMessage: Story = {
    args: {
        kcContext: {
            message: {type: "success", summary: "A new synthetic verification code was sent."},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("status")).toHaveTextContent(
            "A new synthetic verification code was sent."
        )
        await expect(canvas.queryByRole("alert")).not.toBeInTheDocument()
        await expect(canvas.getByLabelText("Digit 1 of 6")).not.toHaveAttribute(
            "aria-invalid",
            "true"
        )
    },
}

export const FrenchPageIdentifiesEnglishOtpFallback: Story = {
    args: {kcContext: {locale: {currentLanguageTag: "fr"}}},
    render: ({kcContext}) => <KcPageStory kcContext={kcContext} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const heading = await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("lang", "fr")
        await expect(canvas.getByText("Votre espace électoral")).toHaveAttribute("lang", "fr")
        await expect(heading).toHaveAttribute("lang", "en")
        await expect(canvas.getByRole("group", {name: "Verification code"})).toHaveAttribute(
            "lang",
            "en"
        )
        await expect(canvas.getByLabelText("Digit 1 of 6")).toHaveAttribute("lang", "en")
        await expect(
            canvas.getByText("Enter the code we sent to your email.").closest("[lang]")
        ).toHaveAttribute("lang", "en")
        await expect(canvas.getByRole("button", {name: "Resend code"})).toHaveAttribute(
            "lang",
            "en"
        )
    },
}
