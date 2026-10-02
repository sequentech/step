/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {
    EMessageAttemptState,
    EMessageChannel,
    EMessagingProvider,
    IMessagingAccount,
} from "@/types/messaging"
import {MessagingTestDialog, testStateSeverity} from "./MessagingTestDialog"

const mockTest = jest.fn()
jest.mock("@apollo/client", () => ({gql: jest.fn(() => ({})), useMutation: () => [mockTest]}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))

const account: IMessagingAccount = {
    id: "viber-1",
    tenant_id: "tenant-1",
    channel: EMessageChannel.VIBER,
    provider: EMessagingProvider.VIBER_INFOBIP,
    name: "COMELEC Viber",
    sender: {provider: EMessagingProvider.VIBER_INFOBIP, base_url: "https://x", sender: "COMELEC"},
    is_default: true,
}

describe("testStateSeverity", () => {
    it("only treats a delivered message as a success", () => {
        expect(testStateSeverity(EMessageAttemptState.DELIVERED)).toBe("success")
        expect(testStateSeverity(EMessageAttemptState.ACCEPTED)).toBe("info")
        expect(testStateSeverity(EMessageAttemptState.UNKNOWN)).toBe("warning")
        expect(testStateSeverity(EMessageAttemptState.FAILED)).toBe("error")
    })
})

describe("MessagingTestDialog", () => {
    beforeEach(() => mockTest.mockReset())

    it("sends the chosen purpose and destination and reports an unknown outcome as unconfirmed", async () => {
        mockTest.mockResolvedValue({
            data: {
                test_messaging_account: {
                    message_id: "m-1",
                    state: EMessageAttemptState.UNKNOWN,
                    reason: "timeout",
                },
            },
        })
        render(
            <MessagingTestDialog account={account} languages={["en", "tl"]} onClose={jest.fn()} />
        )
        const send = screen.getByRole("button", {name: "messagingAccounts.test.send"})
        expect(send).toBeDisabled()
        fireEvent.change(screen.getByLabelText("messagingAccounts.test.destination.PHONE_NUMBER"), {
            target: {value: " +639170000000 "},
        })
        fireEvent.click(send)
        expect(await screen.findByText("messaging.state.UNKNOWN")).toBeInTheDocument()
        expect(screen.getByText(/messaging.stateHelp.UNKNOWN/)).toBeInTheDocument()
        expect(mockTest).toHaveBeenCalledWith({
            variables: {
                id: "viber-1",
                purpose: "OTP",
                destination: "+639170000000",
                language: "en",
            },
        })
    })

    it("requires and sends the approved template for a WhatsApp test", async () => {
        mockTest.mockResolvedValue({
            data: {
                test_messaging_account: {
                    message_id: "m-2",
                    state: EMessageAttemptState.ACCEPTED,
                    reason: null,
                },
            },
        })
        const whatsapp: IMessagingAccount = {
            ...account,
            id: "wa-1",
            channel: EMessageChannel.WHATSAPP,
            provider: EMessagingProvider.WHATSAPP_CLOUD_API,
            sender: {
                provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                business_account_id: "1",
                phone_number_id: "2",
                display_phone_number: "+63 917 000 0000",
                api_version: "v23.0",
            },
        }
        render(<MessagingTestDialog account={whatsapp} languages={["en"]} onClose={jest.fn()} />)
        fireEvent.change(screen.getByLabelText("messagingAccounts.test.destination.PHONE_NUMBER"), {
            target: {value: "+639170000000"},
        })
        const send = screen.getByRole("button", {name: "messagingAccounts.test.send"})
        expect(send).toBeDisabled()
        fireEvent.change(screen.getByLabelText("messagingAccounts.test.template"), {
            target: {value: " otp_en "},
        })
        fireEvent.click(send)
        expect(await screen.findByText("messaging.state.ACCEPTED")).toBeInTheDocument()
        expect(mockTest).toHaveBeenCalledWith({
            variables: {
                id: "wa-1",
                purpose: "OTP",
                destination: "+639170000000",
                language: "en",
                template: "otp_en",
            },
        })
    })

    it("explains that a Viber test uses the approved template of the purpose and language", () => {
        render(<MessagingTestDialog account={account} languages={["en"]} onClose={jest.fn()} />)
        expect(screen.queryByLabelText("messagingAccounts.test.template")).toBeNull()
        expect(screen.getByText("messagingAccounts.test.viberTemplate")).toBeInTheDocument()
    })

    it("reports a failed request", async () => {
        mockTest.mockRejectedValue(new Error("down"))
        render(<MessagingTestDialog account={account} languages={[]} onClose={jest.fn()} />)
        fireEvent.change(screen.getByLabelText("messagingAccounts.test.destination.PHONE_NUMBER"), {
            target: {value: "+639170000000"},
        })
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.test.send"}))
        expect(await screen.findByText("messagingAccounts.test.error")).toBeInTheDocument()
    })
})
