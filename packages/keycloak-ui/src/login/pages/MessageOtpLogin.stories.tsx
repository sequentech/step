// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, spyOn, userEvent, waitFor, within} from "storybook/test"
import {createKcPageStory, getKcContextMock} from "../KcPageStory"
import {
    CodeDisclosure,
    DeliveryState,
    MessageChannel,
    MessageCourier,
    MessengerLinkState,
} from "../KcContext"
import KcPage from "../KcPage"
import {expectStickyActions} from "../scanovate/stickyActions"

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
        await expectStickyActions(canvas.getByRole("button", {name: "Submit"}))
        const form = canvas.getByLabelText("Digit 1 of 6").closest("form")!
        await expect(new FormData(form).get("code")).toBe("123456")
    },
}

/** The code page shares the login header: unresolved build details are left out. */
export const BuildInfoUnavailable: Story = {
    args: {
        kcContext: {
            properties: {systemVersion: "${env.APP_VERSION}", systemHash: "${env.APP_HASH}"},
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const header = within(within(canvasElement).getByRole("banner"))
        await expect(canvasElement.ownerDocument.body.textContent).not.toContain("${")
        await expect(header.queryByRole("term")).toBeNull()
    },
}

// A flow that asks for two codes in a row says which one the page is for, so that the second
// doesn't look like the first one failed.
export const FirstOfTwoCodes: Story = {
    args: {kcContext: {codeRequest: 1, codeRequests: 2}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText("Code 1 of 2")).toBeVisible()
        await expect(canvas.getByRole("progressbar", {name: "Code 1 of 2"})).toHaveAttribute(
            "aria-valuenow",
            "1"
        )
        await expect(
            canvas.getByText(
                "You’ll get 2 codes, one after the other. After this one, we’ll send you the next."
            )
        ).toBeVisible()
    },
}

export const LastOfTwoCodes: Story = {
    args: {kcContext: {codeRequest: 2, codeRequests: 2}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText("Code 2 of 2")).toBeVisible()
        await expect(
            canvas.getByText("Your previous code was accepted. This is the last one.")
        ).toBeVisible()
    },
}

/** A single code needs no count. */
export const SingleCode: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByText(/^Code \d of \d$/)).toBeNull()
        await expect(canvas.queryByRole("progressbar")).toBeNull()
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

// Synthetic contacts, masked the way Keycloak shows them: country code and last three digits.
const WHATSAPP_CODE = {
    courier: MessageCourier.Chosen,
    channel: MessageChannel.WhatsApp,
    address: "+155 ••• ••• 999",
    senderLabel: "Synthetic Commission",
    deliveryState: DeliveryState.Accepted,
}

const expectOneChannel = async (canvas: ReturnType<typeof within>) => {
    await expect(canvas.queryByRole("radio")).not.toBeInTheDocument()
    await expect(canvas.queryByText(/another way/i)).not.toBeInTheDocument()
    await expect(canvas.queryByRole("button", {name: "Send code"})).not.toBeInTheDocument()
}

/** The code page names the one channel, the masked number and who the message is from. */
export const WhatsAppCode: Story = {
    args: {kcContext: WHATSAPP_CODE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("We sent a code to your WhatsApp, +155 ••• ••• 999.")
        ).toBeVisible()
        await expect(
            canvas.getByText(
                "Open WhatsApp on your phone. The message is from Synthetic Commission."
            )
        ).toBeVisible()
        await expect(canvasElement.querySelector('[data-channel="WHATSAPP"]')).not.toBeNull()
        await expectOneChannel(canvas)
        const resend = canvas.getByRole("button", {name: "Resend code"})
        await expect(resend).toHaveAttribute("name", "resend")
        const codeForm = canvas.getByLabelText("Digit 1 of 6").closest("form")!
        await expect(resend.closest("form")).toBe(codeForm)
        await expect(new FormData(codeForm).get("channel")).toBeNull()
    },
}

export const ViberCode: Story = {
    args: {kcContext: {...WHATSAPP_CODE, channel: MessageChannel.Viber}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("We sent a code to your Viber, +155 ••• ••• 999.")
        ).toBeVisible()
        await expect(
            canvas.getByText("Open Viber on your phone. The message is from Synthetic Commission.")
        ).toBeVisible()
        await expectOneChannel(canvas)
    },
}

export const SmsCode: Story = {
    args: {kcContext: {...WHATSAPP_CODE, channel: MessageChannel.Sms}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("We sent a code to your SMS, +155 ••• ••• 999.")
        ).toBeVisible()
        await expectOneChannel(canvas)
    },
}

/** A send whose outcome is unknown is neither "sent" nor "failed"; the voter resends on it. */
export const UnconfirmedDelivery: Story = {
    args: {kcContext: {...WHATSAPP_CODE, deliveryState: DeliveryState.Unknown}},
    beforeEach: () => localStorage.removeItem("resendOtpEndTime"),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("status")).toHaveTextContent("Delivery is not confirmed yet.")
        await expect(canvas.queryByText(/We sent a code/)).not.toBeInTheDocument()
        await expect(canvas.queryByText(/could not send/)).not.toBeInTheDocument()
        await expect(canvas.queryByText(/Open WhatsApp/)).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Resend code"})).toBeEnabled()
        await expectOneChannel(canvas)
    },
}

export const FailedDelivery: Story = {
    args: {kcContext: {...WHATSAPP_CODE, deliveryState: DeliveryState.Failed}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "We could not send the code to your WhatsApp."
        )
        await expect(canvas.getByRole("button", {name: /Resend code/})).toBeVisible()
        await expectOneChannel(canvas)
    },
}

/** A failed send starts the 60 s timer too: the voter waits, as after a delivered one. */
export const FailedDeliveryStartsTheResendTimer: Story = {
    args: {
        kcContext: {
            ...WHATSAPP_CODE,
            deliveryState: DeliveryState.Failed,
            codeJustSent: false,
            resendTimerStarted: true,
        },
    },
    beforeEach: () => {
        localStorage.removeItem("resendOtpEndTime")
        return () => localStorage.removeItem("resendOtpEndTime")
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByRole("button", {name: /Resend code in \d+ seconds/})
        ).toBeDisabled()
        await expect(Number(localStorage.getItem("resendOtpEndTime"))).toBeGreaterThan(Date.now())
    },
}

/** Showing a failed send again never lifts the countdown that is running. */
export const FailedDeliveryKeepsTheResendTimer: Story = {
    args: {kcContext: {...WHATSAPP_CODE, deliveryState: DeliveryState.Failed}},
    beforeEach: () => {
        localStorage.setItem("resendOtpEndTime", String(Date.now() + 60_000))
        return () => localStorage.removeItem("resendOtpEndTime")
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByRole("button", {name: /Resend code in \d+ seconds/})
        ).toBeDisabled()
        await expect(localStorage.getItem("resendOtpEndTime")).not.toBeNull()
    },
}

/** After too many wrong codes the session is over: no code entry, no resend, only a restart. */
export const TooManyAttempts: Story = {
    args: {
        kcContext: {
            ...WHATSAPP_CODE,
            attemptsExhausted: true,
            message: {type: "error", summary: "Too many attempts. Start again."},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getAllByRole("alert")).toHaveLength(1)
        await expect(canvas.getByRole("alert")).toHaveTextContent("Too many attempts. Start again.")
        const restart = canvas.getByRole("link", {name: "Start again"})
        await expect(restart.getAttribute("href")).toBeTruthy()
        await expect(canvas.queryByLabelText("Digit 1 of 6")).not.toBeInTheDocument()
        await expect(canvas.queryByRole("button", {name: /Resend/})).not.toBeInTheDocument()
        await expectOneChannel(canvas)
    },
}

export const TooManyAttemptsSpanish: Story = {
    args: {locale: "es", kcContext: {...WHATSAPP_CODE, attemptsExhausted: true}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "Demasiados intentos. Empiece de nuevo."
        )
        await expect(canvas.getByRole("link", {name: "Empezar de nuevo"})).toBeVisible()
    },
}

// A tiny SVG standing in for the QR image Keycloak draws (an SVG data URI).
const SYNTHETIC_QR =
    "data:image/svg+xml;base64," +
    btoa(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 2"><rect width="1" height="1"/></svg>'
    )

const MESSENGER = {
    courier: MessageCourier.Chosen,
    channel: MessageChannel.Messenger,
    address: "",
    messagingOrganization: "Synthetic Commission",
    messengerPage: "Synthetic Page",
    messengerLink: "https://m.me/synthetic.page?ref=synthetic-reference",
    messengerQr: SYNTHETIC_QR,
    messengerState: MessengerLinkState.Pending,
}

/** Messenger: the voter opens the Page's chat and taps Get Started; the code arrives there. */
export const MessengerConnect: Story = {
    args: {kcContext: MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {level: 1})).toHaveTextContent(
            "Get your code in Messenger"
        )
        await expect(
            canvas.getByText(
                "Synthetic Commission's Facebook Page, Synthetic Page, sends your code in a chat."
            )
        ).toBeVisible()
        const open = canvas.getByRole("link", {name: "Open Messenger"})
        await expect(open).toHaveAttribute(
            "href",
            "https://m.me/synthetic.page?ref=synthetic-reference"
        )
        await expect(open).toHaveAttribute("rel", "noopener noreferrer")
        const qr = canvas.getByRole("img", {
            name: "Scan with your phone to open the chat.",
            hidden: true,
        })
        await expect(qr).toHaveAttribute("src", SYNTHETIC_QR)
        await expect(canvas.getByText("Scan the code with your phone")).toBeInTheDocument()
        await expect(canvas.getByText("Open Messenger or Facebook")).toBeInTheDocument()
        await expect(
            canvas.getByText(
                "Messenger lets the Page send you the code only after you tap Get Started."
            )
        ).toBeVisible()
        await expect(canvas.getByRole("status")).toHaveTextContent(
            "Waiting for you to open the chat."
        )
        const check = canvas.getByRole("button", {name: "Check again"})
        await expect(check).toHaveAttribute("name", "messengerStatus")
        await expect(check).toHaveAttribute("value", "true")
        await expect(canvas.getByRole("button", {name: "Resend code"})).toBeVisible()
        await expect(canvas.queryByText(/We sent a code/)).not.toBeInTheDocument()
        await expect(canvasElement.querySelector('[data-channel="MESSENGER"]')).not.toBeNull()
        await expect(canvas.getByLabelText("Digit 1 of 6")).toBeVisible()
        await expectOneChannel(canvas)
    },
}

/** An older Keycloak doesn't send the organization: the Page alone is named. */
export const MessengerIntroWithoutOrganization: Story = {
    args: {kcContext: {...MESSENGER, messagingOrganization: undefined}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("The Facebook Page Synthetic Page sends your code in a chat.")
        ).toBeVisible()
    },
}

/** Only an absolute https link becomes a link: anything else gives no link and no QR code. */
export const MessengerLinkMustBeHttps: Story = {
    args: {
        kcContext: {
            ...MESSENGER,
            messengerLink: "javascript:alert(document.domain)",
            messengerQr: undefined,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("a[href^='javascript']")).toBeNull()
        await expect(canvas.queryByRole("link", {name: "Open Messenger"})).not.toBeInTheDocument()
        await expect(
            canvas.queryByRole("img", {name: /Scan with your phone/, hidden: true})
        ).not.toBeInTheDocument()
        // The steps still say how to reach the chat.
        await expect(canvas.getByText("Open Messenger or Facebook")).toBeVisible()
    },
}

export const MessengerLinkMustNotBePlainHttp: Story = {
    args: {
        kcContext: {...MESSENGER, messengerLink: "http://m.me/synthetic.page?ref=synthetic"},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByRole("link", {name: "Open Messenger"})).not.toBeInTheDocument()
        await expect(
            canvas.queryByRole("img", {name: /Scan with your phone/, hidden: true})
        ).not.toBeInTheDocument()
    },
}

/** Without Keycloak's image, the browser draws the QR code from the same link. */
export const MessengerQrDrawnInTheBrowser: Story = {
    args: {kcContext: {...MESSENGER, messengerQr: undefined}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const qr = canvas.getByRole("img", {
            name: "Scan with your phone to open the chat.",
            hidden: true,
        })
        await expect(qr.tagName.toLowerCase()).toBe("svg")
    },
}

/** Only an image Keycloak drew is shown: any other source is ignored. */
export const MessengerQrIgnoresOtherSources: Story = {
    args: {kcContext: {...MESSENGER, messengerQr: "https://qr.example.test/code.png"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector('img[src^="https:"]')).toBeNull()
        const qr = canvas.getByRole("img", {
            name: "Scan with your phone to open the chat.",
            hidden: true,
        })
        await expect(qr.tagName.toLowerCase()).toBe("svg")
    },
}

/** R7-10: after a Page change, the signed-in voter is told why they connect Messenger again. */
export const MessengerReconnect: Story = {
    args: {kcContext: {...MESSENGER, messengerReconnect: true}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText(
                "Codes now come from a different Facebook Page. Tap Get Started in its chat to connect Messenger again. The new connection is saved when you enter the code."
            )
        ).toBeVisible()
        await expect(canvas.getByRole("link", {name: "Open Messenger"})).toBeVisible()
    },
}

/** Without a Page change the reconnect explanation isn't shown. */
export const MessengerWithoutReconnect: Story = {
    args: {kcContext: MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByText(/different Facebook Page/)).not.toBeInTheDocument()
    },
}

export const MessengerCodeSent: Story = {
    args: {kcContext: {...MESSENGER, messengerState: MessengerLinkState.CodeSent}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("status")).toHaveTextContent("We sent your code in the chat.")
    },
}

/** A replaced or expired reference cannot be used: the link and QR are gone, resend is the way. */
export const MessengerExpired: Story = {
    args: {kcContext: {...MESSENGER, messengerState: MessengerLinkState.Replaced}},
    beforeEach: () => localStorage.removeItem("resendOtpEndTime"),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "This request has expired. Resend the code, then tap Get Started again."
        )
        await expect(canvas.queryByRole("link", {name: "Open Messenger"})).not.toBeInTheDocument()
        await expect(
            canvas.queryByRole("img", {name: /Scan with your phone/, hidden: true})
        ).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Resend code"})).toBeEnabled()
        await expectOneChannel(canvas)
    },
}

// Submitting a form would leave the story: the forms submitted without a click are recorded.
const submittedForms: HTMLFormElement[] = []
const recordFormSubmits = () => {
    submittedForms.length = 0
    sessionStorage.clear()
    const submit = spyOn(HTMLFormElement.prototype, "submit").mockImplementation(function (
        this: HTMLFormElement
    ) {
        submittedForms.push(this)
    })
    return () => submit.mockRestore()
}

/** While the chat is not open yet, the page asks again by itself every few seconds. */
export const MessengerChecksAgainByItself: Story = {
    args: {kcContext: {...MESSENGER, ttl: "300"}},
    beforeEach: recordFormSubmits,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const poll = canvasElement.querySelector<HTMLFormElement>("form#messenger-poll")!
        await expect(poll).toHaveAttribute("method", "post")
        await expect(new FormData(poll).get("messengerStatus")).toBe("true")
        await expect(new FormData(poll).get("code")).toBeNull()
        await expect(submittedForms).toHaveLength(0)
        await waitFor(() => expect(submittedForms).toEqual([poll]), {timeout: 8000})
        await expect(canvas.getByRole("button", {name: "Check again"})).toBeVisible()
    },
}

/** A voter who is typing the code is never interrupted. */
export const MessengerDoesNotCheckWhileTyping: Story = {
    args: {kcContext: {...MESSENGER, ttl: "300"}},
    beforeEach: recordFormSubmits,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await userEvent.type(canvas.getByLabelText("Digit 1 of 6"), "1")
        await new Promise((resolve) => setTimeout(resolve, 6000))
        await expect(submittedForms).toHaveLength(0)
    },
}

/** Once the code was sent there is nothing left to check. */
export const MessengerStopsCheckingWhenTheCodeWasSent: Story = {
    args: {kcContext: {...MESSENGER, ttl: "300", messengerState: MessengerLinkState.CodeSent}},
    beforeEach: () => sessionStorage.clear(),
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("form#messenger-poll")).toBeNull()
    },
}

/** Checking is bounded by the code's lifetime, counted from when the link was first shown. */
export const MessengerStopsCheckingAfterTheCodeLifetime: Story = {
    args: {kcContext: {...MESSENGER, ttl: "300"}},
    beforeEach: () => {
        sessionStorage.setItem(
            `messengerPollStart:${MESSENGER.messengerLink}`,
            String(Date.now() - 301_000)
        )
        return () => sessionStorage.clear()
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("form#messenger-poll")).toBeNull()
        await expect(canvas.getByRole("button", {name: "Check again"})).toBeVisible()
    },
}

export const WhatsAppCodeSpanish: Story = {
    args: {locale: "es", kcContext: WHATSAPP_CODE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("Enviamos un código a su WhatsApp, +155 ••• ••• 999.")
        ).toBeVisible()
        await expect(canvas.queryByText(/otra forma/)).not.toBeInTheDocument()
    },
}

export const MessengerConnectSpanish: Story = {
    args: {locale: "es", kcContext: MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {level: 1})).toHaveTextContent(
            "Reciba su código en Messenger"
        )
        await expect(canvas.getByRole("link", {name: "Abrir Messenger"})).toBeVisible()
        await expect(
            canvas.getByText(
                "La página de Facebook de Synthetic Commission, Synthetic Page, le envía el código en un chat."
            )
        ).toBeVisible()
    },
}

// A password reset doesn't say whether the account exists: the page names no channel or contact.
const NEUTRAL_CODE = {
    courier: MessageCourier.Chosen,
    codeDisclosure: CodeDisclosure.Neutral,
    channel: undefined,
    address: undefined,
    senderLabel: undefined,
    deliveryState: undefined,
}

const expectNoContact = async (canvasElement: HTMLElement) => {
    const text = canvasElement.textContent ?? ""
    await expect(text).not.toMatch(/WhatsApp|Viber|Messenger|SMS|e-?mail|\+155|•/i)
    await expect(canvasElement.querySelector("[data-channel]")).toBeNull()
    await expect(canvasElement.querySelector("#messenger-intro")).toBeNull()
}

export const NeutralResetCode: Story = {
    args: {kcContext: {...NEUTRAL_CODE, resendTimerStarted: true}},
    beforeEach: () => {
        localStorage.removeItem("resendOtpEndTime")
        return () => localStorage.removeItem("resendOtpEndTime")
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("If an account exists, we sent a code to its contact.")
        ).toBeVisible()
        await expect(canvas.queryByText(/We sent a code/)).not.toBeInTheDocument()
        await expect(canvas.queryByText(/not confirmed/)).not.toBeInTheDocument()
        await expect(canvas.getByRole("group", {name: "Verification code"})).toBeVisible()
        // The 60 s timer runs as on any code page.
        await expect(
            canvas.getByRole("button", {name: /Resend code in \d+ seconds/})
        ).toBeDisabled()
        await expectNoContact(canvasElement)
    },
}

/** Contact data sent by mistake is never shown on a neutral page. */
export const NeutralResetCodeIgnoresContactData: Story = {
    args: {
        kcContext: {
            ...WHATSAPP_CODE,
            ...MESSENGER,
            codeDisclosure: CodeDisclosure.Neutral,
            address: "+155 ••• ••• 999",
            senderLabel: "Synthetic Commission",
            deliveryState: DeliveryState.Unknown,
        },
    },
    beforeEach: () => sessionStorage.clear(),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("If an account exists, we sent a code to its contact.")
        ).toBeVisible()
        await expect(canvasElement.querySelector("form#messenger-poll")).toBeNull()
        await expect(canvas.queryByRole("button", {name: "Check again"})).not.toBeInTheDocument()
        await expectNoContact(canvasElement)
    },
}

export const NeutralResetCodeSpanish: Story = {
    args: {locale: "es", kcContext: NEUTRAL_CODE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("Si existe una cuenta, le enviamos un código a su contacto.")
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: /Reenviar código/})).toBeVisible()
        await expectNoContact(canvasElement)
    },
}

export const NeutralResetCodeTooManyAttempts: Story = {
    args: {kcContext: {...NEUTRAL_CODE, attemptsExhausted: true}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent("Too many attempts. Start again.")
        await expect(canvas.getByRole("link", {name: "Start again"})).toBeVisible()
        await expectNoContact(canvasElement)
    },
}

// When the event offers Messenger for codes, every reset page shows how to connect it, for any
// account: whether this one uses Messenger is never told.
const NEUTRAL_MESSENGER = {
    ...NEUTRAL_CODE,
    messengerConnect: true,
    messagingOrganization: "Synthetic Commission",
    messengerPage: "Synthetic Page",
    messengerLink: "https://m.me/synthetic.page?ref=synthetic-reference",
    messengerQr: SYNTHETIC_QR,
    messengerState: undefined,
}

const expectNoNamedChannel = async (canvasElement: HTMLElement) => {
    await expect(canvasElement.textContent ?? "").not.toMatch(/WhatsApp|Viber|SMS|e-?mail|\+155|•/i)
    await expect(canvasElement.querySelector("[data-channel]")).toBeNull()
}

export const NeutralResetCodeWithMessenger: Story = {
    args: {kcContext: NEUTRAL_MESSENGER},
    beforeEach: recordFormSubmits,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("If an account exists, we sent a code to its contact.")
        ).toBeVisible()
        const box = within(canvas.getByRole("region", {name: "If you get your codes in Messenger"}))
        await expect(
            box.getByRole("heading", {level: 2, name: "If you get your codes in Messenger"})
        ).toBeVisible()
        await expect(
            box.getByText(
                "Synthetic Commission's Facebook Page, Synthetic Page, sends your code in a chat."
            )
        ).toBeVisible()
        await expect(box.getByRole("link", {name: "Open Messenger"})).toHaveAttribute(
            "href",
            "https://m.me/synthetic.page?ref=synthetic-reference"
        )
        await expect(
            box.getByRole("img", {name: "Scan with your phone to open the chat.", hidden: true})
        ).toHaveAttribute("src", SYNTHETIC_QR)
        await expect(box.getByText("Open Messenger or Facebook")).toBeInTheDocument()
        await expect(box.getByText("Scan the code with your phone")).toBeInTheDocument()
        await expect(
            box.getByText(
                "Messenger lets the Page send you the code only after you tap Get Started."
            )
        ).toBeVisible()
        // No link state, no checking by itself, no "Check again".
        await expect(canvas.queryByRole("status")).not.toBeInTheDocument()
        await expect(canvas.queryByText(/Waiting for you|sent your code|expired/)).toBeNull()
        await expect(canvasElement.querySelector("form#messenger-poll")).toBeNull()
        await expect(canvas.queryByRole("button", {name: "Check again"})).not.toBeInTheDocument()
        // The code entry and resend stay.
        await expect(canvas.getByRole("group", {name: "Verification code"})).toBeVisible()
        await expect(canvas.getByRole("button", {name: /Resend code/})).toBeVisible()
        await new Promise((resolve) => setTimeout(resolve, 6000))
        await expect(submittedForms).toHaveLength(0)
        await expectNoNamedChannel(canvasElement)
    },
}

export const NeutralResetCodeWithMessengerSpanish: Story = {
    args: {locale: "es", kcContext: NEUTRAL_MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByRole("heading", {level: 2, name: "Si recibe sus códigos por Messenger"})
        ).toBeVisible()
        await expect(
            canvas.getByText(
                "La página de Facebook de Synthetic Commission, Synthetic Page, le envía el código en un chat."
            )
        ).toBeVisible()
        await expect(canvas.getByRole("link", {name: "Abrir Messenger"})).toBeVisible()
        await expectNoNamedChannel(canvasElement)
    },
}

/** On a neutral page too, only an https link becomes a link. */
export const NeutralResetCodeWithMessengerLinkMustBeHttps: Story = {
    args: {
        kcContext: {
            ...NEUTRAL_MESSENGER,
            messengerLink: "javascript:alert(document.domain)",
            messengerQr: undefined,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByRole("heading", {level: 2, name: "If you get your codes in Messenger"})
        ).toBeVisible()
        await expect(canvasElement.querySelector("a[href^='javascript']")).toBeNull()
        await expect(canvas.queryByRole("link", {name: "Open Messenger"})).not.toBeInTheDocument()
        await expect(
            canvas.queryByRole("img", {name: /Scan with your phone/, hidden: true})
        ).not.toBeInTheDocument()
    },
}

/** Without messengerConnect the neutral page has no Messenger box. */
export const NeutralResetCodeWithoutMessenger: Story = {
    args: {kcContext: {...NEUTRAL_MESSENGER, messengerConnect: undefined}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByRole("heading", {level: 2})).not.toBeInTheDocument()
        await expectNoContact(canvasElement)
    },
}
