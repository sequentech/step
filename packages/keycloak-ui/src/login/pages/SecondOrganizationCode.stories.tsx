// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The code page of a second organization (ticket: "Configuration 2 · Second
// organization's code page", msg-fixture-verify-mobile.png): the same page,
// with its own name, its voter's channel and a masked Spanish number. Every
// value is invented; the page names no other client.
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"
import {DeliveryState, MessageChannel, MessageCourier, MessengerLinkState} from "../KcContext"

const {KcPageStory} = createKcPageStory({pageId: "message-otp.login.ftl"})

const meta = {
    title: "Keycloak/Second organization code page",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

const ORGANIZATION = "Staff Association"
const PAGE = "Staff Association Elections"
const MASKED_NUMBER = "+34 ••• ••• 214"
const OTHER_CLIENT = /COMELEC/i

// A tiny SVG standing in for the QR image Keycloak draws (an SVG data URI).
const SYNTHETIC_QR =
    "data:image/svg+xml;base64," +
    btoa(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2 2"><rect width="1" height="1"/></svg>'
    )

const WHATSAPP_CODE = {
    courier: MessageCourier.Chosen,
    channel: MessageChannel.WhatsApp,
    address: MASKED_NUMBER,
    senderLabel: ORGANIZATION,
    messagingOrganization: ORGANIZATION,
    deliveryState: DeliveryState.Accepted,
}

const MESSENGER = {
    courier: MessageCourier.Chosen,
    channel: MessageChannel.Messenger,
    address: "",
    messagingOrganization: ORGANIZATION,
    messengerPage: PAGE,
    messengerLink: "https://m.me/staffassocelections?ref=synthetic-reference",
    messengerQr: SYNTHETIC_QR,
    messengerState: MessengerLinkState.Pending,
}

/** Its voter's channel, the masked number and the organization the message is from. */
export const WhatsAppCode: Story = {
    args: {kcContext: WHATSAPP_CODE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText(`We sent a code to your WhatsApp, ${MASKED_NUMBER}.`)
        ).toBeVisible()
        await expect(
            canvas.getByText(`Open WhatsApp on your phone. The message is from ${ORGANIZATION}.`)
        ).toBeVisible()
        await expect(canvasElement.querySelector('[data-channel="WHATSAPP"]')).not.toBeNull()
        await expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}

/** The same page in Spanish. */
export const WhatsAppCodeSpanish: Story = {
    args: {locale: "es", kcContext: WHATSAPP_CODE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText(`Enviamos un código a su WhatsApp, ${MASKED_NUMBER}.`)
        ).toBeVisible()
        await expect(
            canvas.getByText(`Abra WhatsApp en su teléfono. El mensaje es de ${ORGANIZATION}.`)
        ).toBeVisible()
        await expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}

/** Messenger: the organization's own Page sends the code. */
export const MessengerConnect: Story = {
    args: {kcContext: MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {level: 1})).toHaveTextContent(
            "Get your code in Messenger"
        )
        await expect(
            canvas.getByText(`${ORGANIZATION}'s Facebook Page, ${PAGE}, sends your code in a chat.`)
        ).toBeVisible()
        await expect(canvasElement.textContent).not.toMatch(OTHER_CLIENT)
    },
}
