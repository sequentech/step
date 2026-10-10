// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, spyOn, userEvent, waitFor, within} from "storybook/test"
import {createKcPageStory, getKcContextMock} from "../KcPageStory"
import {
    DeliveryState,
    MessageChannel,
    MessageCourier,
    MessengerLinkState,
    OtpView,
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

const WHATSAPP_CODE = {
    courier: MessageCourier.Chosen,
    otpView: OtpView.Code,
    channel: MessageChannel.WhatsApp,
    address: "+155*****999",
    senderLabel: "Synthetic Commission",
    deliveryState: DeliveryState.Accepted,
    otherWayChannels: [MessageChannel.Email],
    channelAddresses: {
        [MessageChannel.WhatsApp]: "+155*****999",
        [MessageChannel.Email]: "sy***@*****le.test",
    },
}

/** The code page names the channel, the masked number and who the message is from. */
export const WhatsAppCode: Story = {
    args: {kcContext: WHATSAPP_CODE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("We sent a code to your WhatsApp, +155*****999.")
        ).toBeVisible()
        await expect(
            canvas.getByText(
                "Open WhatsApp on your phone. The message is from Synthetic Commission."
            )
        ).toBeVisible()
        await expect(canvas.queryByRole("radio")).not.toBeInTheDocument()
        const otherWay = canvas.getByRole("button", {name: "Get the code another way"})
        await expect(otherWay).toHaveAttribute("aria-expanded", "false")
        await userEvent.click(otherWay)
        await expect(
            canvas.getByText(
                "Choose an available method. Requesting a new code replaces the previous code."
            )
        ).toBeVisible()
        const option = canvas.getByRole("radio", {
            name: "We send the code to your Email, sy***@*****le.test.",
        })
        await expect(option).toBeChecked()
        const send = canvas.getByRole("button", {name: "Send code"})
        await expect(send).toBeEnabled()
        const form = send.closest("form")!
        await expect(new FormData(form).get("channel")).toBe("EMAIL")
        await expect(new FormData(form).get("code")).toBeNull()
        const codeForm = canvas.getByLabelText("Digit 1 of 6").closest("form")!
        await expect(new FormData(codeForm).get("channel")).toBeNull()
    },
}

/** A send whose outcome is unknown is neither "sent" nor "failed"; the other way is open. */
export const UnconfirmedDelivery: Story = {
    args: {kcContext: {...WHATSAPP_CODE, deliveryState: DeliveryState.Unknown}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("status")).toHaveTextContent("Delivery is not confirmed yet.")
        await expect(canvas.queryByText(/We sent a code/)).not.toBeInTheDocument()
        await expect(canvas.queryByText(/could not send/)).not.toBeInTheDocument()
        await expect(canvas.queryByText(/Open WhatsApp/)).not.toBeInTheDocument()
        await expect(
            canvas.getByRole("button", {name: "Get the code another way"})
        ).toHaveAttribute("aria-expanded", "true")
        await expect(canvas.getByRole("button", {name: "Send code"})).toBeVisible()
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
        await expect(canvas.getByRole("button", {name: "Send code"})).toBeVisible()
    },
}

/** A replacement code waits for the same resend timer, so switching channel is no shortcut. */
export const OtherWayWaitsForTheResendTimer: Story = {
    args: {kcContext: {...WHATSAPP_CODE, codeJustSent: true}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await userEvent.click(canvas.getByRole("button", {name: "Get the code another way"}))
        await expect(canvas.getByRole("button", {name: "Send code"})).toBeDisabled()
        await expect(
            canvas.getByRole("button", {name: /Resend code in \d+ seconds/})
        ).toBeDisabled()
    },
}

/** Sign-in with several verified contacts: nothing is sent until the voter picks one. */
export const ChooseChannel: Story = {
    args: {
        kcContext: {
            courier: MessageCourier.Chosen,
            otpView: OtpView.Choose,
            otherWayChannels: [MessageChannel.WhatsApp, MessageChannel.Email],
            channelAddresses: {
                [MessageChannel.WhatsApp]: "+155*****999",
                [MessageChannel.Email]: "sy***@*****le.test",
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {level: 1})).toHaveTextContent(
            "Get a code by"
        )
        await expect(canvas.getByText("Use one of the ways you set up.")).toBeVisible()
        await expect(canvas.queryByLabelText("Digit 1 of 6")).not.toBeInTheDocument()
        await userEvent.click(
            canvas.getByRole("radio", {name: "We send the code to your Email, sy***@*****le.test."})
        )
        const send = canvas.getByRole("button", {name: "Send code"})
        await expect(new FormData(send.closest("form")!).get("channel")).toBe("EMAIL")
    },
}

const MESSENGER = {
    courier: MessageCourier.Chosen,
    otpView: OtpView.Code,
    channel: MessageChannel.Messenger,
    address: "",
    messengerPage: "Synthetic Page",
    messengerLink: "https://m.me/synthetic.page?ref=synthetic-reference",
    messengerWord: "MAPLE",
    messengerState: MessengerLinkState.Pending,
    otherWayChannels: [MessageChannel.Email],
    channelAddresses: {[MessageChannel.Email]: "sy***@*****le.test"},
}

/** Messenger: the voter opens the Page's chat; the code arrives there. */
export const MessengerConnect: Story = {
    args: {kcContext: MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByRole("heading", {level: 2, name: "Get your code in Messenger"})
        ).toBeVisible()
        await expect(
            canvas.getByText("The Facebook Page Synthetic Page sends your code in a chat.")
        ).toBeVisible()
        const connect = canvas.getByRole("link", {name: "Connect Messenger"})
        await expect(connect).toHaveAttribute(
            "href",
            "https://m.me/synthetic.page?ref=synthetic-reference"
        )
        await expect(connect).toHaveAttribute("rel", "noopener noreferrer")
        await expect(
            canvas.getByText(
                "No code after tapping Get Started? Send MAPLE to Synthetic Page in the chat."
            )
        ).toBeVisible()
        await expect(canvas.getByRole("status")).toHaveTextContent(
            "Waiting for you to open the chat."
        )
        const check = canvas.getByRole("button", {name: "Check again"})
        await expect(check).toHaveAttribute("name", "messengerStatus")
        await expect(check).toHaveAttribute("value", "true")
        await expect(canvas.queryByText(/We sent a code/)).not.toBeInTheDocument()
        await expect(canvas.getByLabelText("Digit 1 of 6")).toBeVisible()
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

/** A replaced or expired reference cannot be used: the link is gone and the other way offered. */
export const MessengerExpired: Story = {
    args: {kcContext: {...MESSENGER, messengerState: MessengerLinkState.Replaced}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "This request has expired. Get the code another way."
        )
        await expect(
            canvas.queryByRole("link", {name: "Connect Messenger"})
        ).not.toBeInTheDocument()
        await expect(canvas.queryByText(/MAPLE/)).not.toBeInTheDocument()
    },
}

/** A code that was not sent starts no countdown: the voter can ask again at once. */
export const FailedDeliveryCanRetryAtOnce: Story = {
    args: {kcContext: {...WHATSAPP_CODE, deliveryState: DeliveryState.Failed}},
    beforeEach: () => {
        localStorage.setItem("resendOtpEndTime", String(Date.now() + 60_000))
        return () => localStorage.removeItem("resendOtpEndTime")
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("button", {name: "Resend code"})).toBeEnabled()
        await expect(canvas.getByRole("button", {name: "Send code"})).toBeEnabled()
        await expect(localStorage.getItem("resendOtpEndTime")).toBeNull()
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
            canvas.getByText("Enviamos un código a su WhatsApp, +155*****999.")
        ).toBeVisible()
        await expect(
            canvas.getByRole("button", {name: "Recibir el código de otra forma"})
        ).toBeVisible()
    },
}
