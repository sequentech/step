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
    EMessagePurpose,
    EMessagingProvider,
    IMessagingAccount,
} from "@/types/messaging"
import {MessagingTestDialog, testStateSeverity, testTemplateField} from "./MessagingTestDialog"

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
        fireEvent.change(screen.getByLabelText(/messagingAccounts.test.template/), {
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

    it("takes the provider's language code, which need not be a language of the tenant", async () => {
        mockTest.mockResolvedValue({data: {test_messaging_account: null}})
        render(<MessagingTestDialog account={account} languages={["en"]} onClose={jest.fn()} />)
        expect(screen.getByText("messagingAccounts.test.languageHelp")).toBeInTheDocument()
        fireEvent.change(screen.getByLabelText("messagingAccounts.test.language"), {
            target: {value: " en_US "},
        })
        fireEvent.change(screen.getByLabelText("messagingAccounts.test.destination.PHONE_NUMBER"), {
            target: {value: "+639170000000"},
        })
        fireEvent.click(screen.getByRole("button", {name: "messagingAccounts.test.send"}))
        await screen.findByText("messagingAccounts.test.error")
        expect(mockTest.mock.calls[0][0].variables.language).toBe("en_US")
    })

    it("offers a custom HTTP API the template, required for its template-only purposes", () => {
        const custom = (templateRequiredFor: EMessagePurpose[]): IMessagingAccount => ({
            ...account,
            provider: EMessagingProvider.HTTP_API,
            sender: {
                provider: EMessagingProvider.HTTP_API,
                send: {url: "https://partner.example"},
                template_required_for: templateRequiredFor,
            },
        })
        expect(testTemplateField(custom([EMessagePurpose.OTP]), EMessagePurpose.OTP)).toEqual({
            offered: true,
            required: true,
        })
        expect(testTemplateField(custom([EMessagePurpose.OTP]), EMessagePurpose.NOTICE)).toEqual({
            offered: true,
            required: false,
        })
        expect(testTemplateField(account, EMessagePurpose.OTP)).toEqual({
            offered: false,
            required: false,
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
