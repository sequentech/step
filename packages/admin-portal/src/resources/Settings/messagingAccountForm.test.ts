// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    ECredentialName,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EProviderApproval,
    EReadinessBlocker,
    IMessagingAccount,
} from "@/types/messaging"
import {
    EAccountFormError,
    IAccountFormValues,
    accountReadiness,
    credentialsPayload,
    emptyAccountForm,
    formFromAccount,
    limitsFromForm,
    parseCallingCodes,
    selectableProviders,
    senderFields,
    senderFromForm,
    validateAccountForm,
} from "./messagingAccountForm"

const form = (
    provider: EMessagingProvider,
    channel: EMessageChannel,
    overrides: Partial<IAccountFormValues> = {}
): IAccountFormValues => ({...emptyAccountForm(channel, provider), name: "Main", ...overrides})

const account = (overrides: Partial<IMessagingAccount> = {}): IMessagingAccount => ({
    id: "account-1",
    tenant_id: "tenant-1",
    channel: EMessageChannel.WHATSAPP,
    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
    name: "COMELEC WhatsApp",
    sender: {
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        business_account_id: "1001",
        phone_number_id: "2002",
        display_phone_number: "+63 917 000 0000",
        display_name: null,
        api_version: "v23.0",
    },
    credentials: {ACCESS_TOKEN: {replaced_at: "2026-09-30T10:00:00Z"}},
    limits: {messages_per_second: 20, otp_reserved_per_second: 5, allowed_calling_codes: ["63"]},
    provider_approval: EProviderApproval.PENDING,
    status: {connected: true, production_access: true, approved_templates: {}},
    webhook_key: "key-1",
    is_default: true,
    ...overrides,
})

describe("parseCallingCodes", () => {
    it("accepts digits with an optional plus, separated by commas or spaces", () => {
        expect(parseCallingCodes("63, +1 44,63")).toEqual({codes: ["63", "1", "44"], invalid: []})
    })

    it("reports entries that are not one to three digits", () => {
        expect(parseCallingCodes("63 PH 12345 +")).toEqual({
            codes: ["63"],
            invalid: ["PH", "12345", "+"],
        })
    })

    it("treats an empty text as any destination", () => {
        expect(parseCallingCodes("  ")).toEqual({codes: [], invalid: []})
    })
})

describe("senderFields", () => {
    it("renders only the selected provider's identifiers", () => {
        expect(senderFields(EMessagingProvider.MESSENGER_SEND_API).map((f) => f.key)).toEqual([
            "page_id",
            "page_name",
            "page_username",
            "api_version",
        ])
        expect(senderFields(EMessagingProvider.AWS_SES).map((f) => f.key)).toEqual([
            "from_address",
            "from_name",
            "region",
            "notification_topic_arn",
        ])
        expect(senderFields(EMessagingProvider.CONSOLE)).toEqual([])
    })
})

describe("selectableProviders", () => {
    it("offers the channel's providers but not the console, whose channel cannot be saved", () => {
        expect(selectableProviders(EMessageChannel.EMAIL)).toEqual([
            EMessagingProvider.AWS_SES,
            EMessagingProvider.SMTP,
        ])
        expect(selectableProviders(EMessageChannel.VIBER)).toEqual([
            EMessagingProvider.VIBER_INFOBIP,
        ])
    })
})

describe("senderFromForm", () => {
    it("builds the tagged sender, turning empty optional fields into null", () => {
        const values = form(EMessagingProvider.AWS_SNS, EMessageChannel.SMS, {
            sender: {sender_id: "COMELEC", origination_number: " ", region: "ap-southeast-1"},
        })
        expect(senderFromForm(values)).toEqual({
            provider: EMessagingProvider.AWS_SNS,
            sender_id: "COMELEC",
            origination_number: null,
            region: "ap-southeast-1",
        })
    })

    it("keeps the Viber approved templates per purpose and language", () => {
        const values = form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
            sender: {base_url: "https://x.api.infobip.com", sender: "COMELEC"},
            viberTemplates: [
                {purpose: EMessagePurpose.OTP, language: "en", templateId: "88213"},
                {purpose: EMessagePurpose.OTP, language: "tl", templateId: " 88214 "},
                {purpose: EMessagePurpose.NOTICE, language: "en", templateId: ""},
            ],
        })
        expect(senderFromForm(values)).toEqual({
            provider: EMessagingProvider.VIBER_INFOBIP,
            base_url: "https://x.api.infobip.com",
            sender: "COMELEC",
            approved_templates: {OTP: {en: "88213", tl: "88214"}},
        })
    })

    it("round-trips an existing account through the form", () => {
        const existing = account()
        const values = formFromAccount(existing)
        expect(values.sender.display_name).toBe("")
        expect(senderFromForm(values)).toEqual(existing.sender)
        expect(limitsFromForm(values)).toEqual(existing.limits)
        expect(values.credentials).toEqual({})
    })
})

describe("validateAccountForm", () => {
    it("requires the name and the provider's required identifiers", () => {
        const values = form(EMessagingProvider.WHATSAPP_CLOUD_API, EMessageChannel.WHATSAPP, {
            name: " ",
        })
        expect(validateAccountForm(values)).toEqual({
            "name": EAccountFormError.REQUIRED,
            "sender.business_account_id": EAccountFormError.REQUIRED,
            "sender.phone_number_id": EAccountFormError.REQUIRED,
            "sender.display_phone_number": EAccountFormError.REQUIRED,
        })
    })

    it("rejects invalid limits and calling codes", () => {
        const values = form(EMessagingProvider.AWS_SNS, EMessageChannel.SMS, {
            limits: {
                messagesPerSecond: "10",
                otpReservedPerSecond: "12",
                allowedCallingCodes: "63 x",
            },
        })
        expect(validateAccountForm(values)).toEqual({
            "limits.otpReservedPerSecond": EAccountFormError.OTP_ABOVE_TOTAL,
            "limits.allowedCallingCodes": EAccountFormError.INVALID_CALLING_CODE,
        })
        expect(
            validateAccountForm({
                ...values,
                limits: {
                    messagesPerSecond: "-1",
                    otpReservedPerSecond: "1.5",
                    allowedCallingCodes: "",
                },
            })
        ).toEqual({
            "limits.messagesPerSecond": EAccountFormError.NOT_A_COUNT,
            "limits.otpReservedPerSecond": EAccountFormError.NOT_A_COUNT,
        })
    })

    it("requires a template ID and no repeated language per Viber purpose", () => {
        const values = form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
            sender: {base_url: "https://x", sender: "COMELEC"},
            viberTemplates: [
                {purpose: EMessagePurpose.OTP, language: "en", templateId: "1"},
                {purpose: EMessagePurpose.OTP, language: "en", templateId: "2"},
                {purpose: EMessagePurpose.NOTICE, language: "en", templateId: ""},
            ],
        })
        expect(validateAccountForm(values)).toEqual({
            "viberTemplates.1": EAccountFormError.DUPLICATE_LANGUAGE,
            "viberTemplates.2": EAccountFormError.REQUIRED,
        })
    })
})

describe("credentialsPayload", () => {
    it("sends only typed credentials the provider accepts", () => {
        const values = form(EMessagingProvider.WHATSAPP_CLOUD_API, EMessageChannel.WHATSAPP, {
            credentials: {
                ACCESS_TOKEN: "token",
                APP_SECRET: "",
                VERIFY_TOKEN: "typed",
                API_KEY: "other",
            },
        })
        expect(credentialsPayload(values)).toEqual({[ECredentialName.ACCESS_TOKEN]: "token"})
        expect(credentialsPayload({...values, credentials: {}})).toBeNull()
    })
})

describe("accountReadiness", () => {
    it("separates connection from each purpose's readiness", () => {
        const readiness = accountReadiness(account())
        expect(readiness.connected).toBe(true)
        expect(readiness.purposes.OTP.blockers).toEqual([
            EReadinessBlocker.NEEDS_PROVIDER_APPROVAL,
            EReadinessBlocker.NEEDS_APPROVED_TEMPLATE,
        ])
    })

    it("treats an account never checked as not connected", () => {
        const readiness = accountReadiness(
            account({
                channel: EMessageChannel.SMS,
                provider: EMessagingProvider.AWS_SNS,
                sender: {provider: EMessagingProvider.AWS_SNS},
                status: null,
            })
        )
        expect(readiness.connected).toBe(false)
        expect(readiness.purposes.NOTICE.blockers).toEqual([
            EReadinessBlocker.NOT_CONNECTED,
            EReadinessBlocker.NEEDS_PRODUCTION_ACCESS,
        ])
    })
})
