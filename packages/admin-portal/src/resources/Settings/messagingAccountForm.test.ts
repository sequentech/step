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
    EReadinessPolicy,
    IMessagingAccount,
    MESSAGE_CHANNELS,
} from "@/types/messaging"
import {emptyHttpApiForm, withHttpExample} from "./httpApiSender"
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
    upsertAccountVariables,
    validateAccountForm,
    withChannel,
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
        api_base_url: null,
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
            "api_base_url",
        ])
        expect(senderFields(EMessagingProvider.HTTP_API)).toEqual([{key: "label", required: false}])
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
    it("offers the channel's providers, then the custom HTTP API and the console", () => {
        expect(selectableProviders(EMessageChannel.EMAIL)).toEqual([
            EMessagingProvider.AWS_SES,
            EMessagingProvider.SMTP,
            EMessagingProvider.HTTP_API,
            EMessagingProvider.CONSOLE,
        ])
        for (const channel of MESSAGE_CHANNELS) {
            expect(selectableProviders(channel).slice(-2)).toEqual([
                EMessagingProvider.HTTP_API,
                EMessagingProvider.CONSOLE,
            ])
        }
    })
})

describe("withChannel", () => {
    it("selects the first provider of the new channel", () => {
        const values = withChannel(
            form(EMessagingProvider.AWS_SES, EMessageChannel.EMAIL),
            EMessageChannel.VIBER
        )
        expect(values.channel).toBe(EMessageChannel.VIBER)
        expect(values.provider).toBe(EMessagingProvider.VIBER_INFOBIP)
        expect(Object.keys(values.sender)).toEqual(["base_url", "sender"])
    })

    it("keeps a provider that serves any channel and what was entered for it", () => {
        const custom = form(EMessagingProvider.HTTP_API, EMessageChannel.SMS, {
            sender: {label: "COMELEC"},
            http: withHttpExample(emptyHttpApiForm()),
        })
        const values = withChannel(custom, EMessageChannel.VIBER)
        expect(values).toEqual({...custom, channel: EMessageChannel.VIBER})
    })
})

describe("upsertAccountVariables", () => {
    it("sends the chosen channel and readiness, and approval only where it applies", () => {
        const custom = form(EMessagingProvider.HTTP_API, EMessageChannel.VIBER, {
            name: " Partner ",
            sender: {label: "COMELEC"},
            http: withHttpExample(emptyHttpApiForm()),
            readiness: EReadinessPolicy.ADMIN_CONFIRMED,
            providerApproval: EProviderApproval.CONFIRMED,
        })
        expect(upsertAccountVariables(null, custom)).toEqual({
            id: null,
            channel: EMessageChannel.VIBER,
            name: "Partner",
            sender: senderFromForm(custom),
            limits: limitsFromForm(custom),
            providerApproval: null,
            readiness: EReadinessPolicy.ADMIN_CONFIRMED,
            isDefault: false,
        })
        expect(senderFromForm(custom)).toMatchObject({
            provider: EMessagingProvider.HTTP_API,
            label: "COMELEC",
            message_id_pointer: "/message_id",
        })
        const whatsapp = formFromAccount(account({provider_approval: EProviderApproval.CONFIRMED}))
        expect(upsertAccountVariables("account-1", whatsapp)).toMatchObject({
            id: "account-1",
            channel: EMessageChannel.WHATSAPP,
            providerApproval: EProviderApproval.CONFIRMED,
            readiness: EReadinessPolicy.PROVIDER_CHECK,
        })
    })

    it("saves the console provider for the chosen channel", () => {
        expect(
            upsertAccountVariables(null, form(EMessagingProvider.CONSOLE, EMessageChannel.SMS))
        ).toMatchObject({
            channel: EMessageChannel.SMS,
            sender: {provider: EMessagingProvider.CONSOLE},
        })
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

describe("the v2 fields", () => {
    it("keeps the Graph API base URL when it is not Meta's own", () => {
        const values = formFromAccount(
            account({
                sender: {
                    provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                    business_account_id: "1001",
                    phone_number_id: "2002",
                    display_phone_number: "+63 917 000 0000",
                    api_version: "v23.0",
                    api_base_url: "https://graph.bsp.example",
                },
            })
        )
        expect(values.sender.api_base_url).toBe("https://graph.bsp.example")
        expect(senderFromForm(values)).toMatchObject({api_base_url: "https://graph.bsp.example"})
        expect(
            validateAccountForm({
                ...values,
                sender: {...values.sender, api_base_url: "graph.bsp.example"},
            })
        ).toEqual({"sender.api_base_url": EAccountFormError.NOT_A_URL})
    })

    it("reads the readiness of an account, the provider's check by default", () => {
        expect(formFromAccount(account()).readiness).toBe(EReadinessPolicy.PROVIDER_CHECK)
        expect(
            formFromAccount(account({readiness: EReadinessPolicy.ADMIN_CONFIRMED})).readiness
        ).toBe(EReadinessPolicy.ADMIN_CONFIRMED)
    })

    it("refuses a custom HTTP API whose requests are not valid", () => {
        const values = form(EMessagingProvider.HTTP_API, EMessageChannel.SMS)
        expect(validateAccountForm(values)).toEqual({
            "http.SEND": EAccountFormError.INVALID_HTTP_CONFIG,
        })
        expect(validateAccountForm({...values, http: withHttpExample(emptyHttpApiForm())})).toEqual(
            {}
        )
    })

    it("round-trips a custom HTTP API account through the form", () => {
        const sender = senderFromForm(
            form(EMessagingProvider.HTTP_API, EMessageChannel.VIBER, {
                sender: {label: "COMELEC"},
                http: {
                    ...withHttpExample(emptyHttpApiForm()),
                    templateRequiredFor: [EMessagePurpose.OTP],
                    approvedLanguages: {OTP: "en, tl", NOTICE: ""},
                },
            })
        )
        const stored = account({
            channel: EMessageChannel.VIBER,
            provider: EMessagingProvider.HTTP_API,
            sender,
        })
        expect(senderFromForm(formFromAccount(stored))).toEqual(sender)
    })
})

describe("credentialsPayload", () => {
    it("offers a custom HTTP API its own credentials", () => {
        const values = form(EMessagingProvider.HTTP_API, EMessageChannel.SMS, {
            credentials: {API_KEY: "key", WEBHOOK_SECRET: "secret", SMTP_PASSWORD: "other"},
        })
        expect(credentialsPayload(values)).toEqual({API_KEY: "key", WEBHOOK_SECRET: "secret"})
    })

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

    it("an account confirmed by an administrator is ready once the provider approved it", () => {
        const confirmed = account({
            readiness: EReadinessPolicy.ADMIN_CONFIRMED,
            status: null,
        })
        expect(accountReadiness(confirmed).policy).toBe(EReadinessPolicy.ADMIN_CONFIRMED)
        expect(accountReadiness(confirmed).purposes.OTP.blockers).toEqual([
            EReadinessBlocker.NEEDS_PROVIDER_APPROVAL,
        ])
        expect(
            accountReadiness({...confirmed, provider_approval: EProviderApproval.CONFIRMED})
                .purposes.OTP.blockers
        ).toEqual([])
    })

    it("uses the capabilities a custom HTTP API declares", () => {
        const custom = account({
            channel: EMessageChannel.VIBER,
            provider: EMessagingProvider.HTTP_API,
            sender: {
                provider: EMessagingProvider.HTTP_API,
                send: {url: "https://partner.example"},
                template_required_for: [EMessagePurpose.OTP],
            },
            status: {
                connected: true,
                production_access: true,
                approved_templates: {NOTICE: ["en"]},
            },
        })
        const readiness = accountReadiness(custom)
        expect(readiness.purposes.OTP.blockers).toEqual([EReadinessBlocker.NEEDS_APPROVED_TEMPLATE])
        expect(readiness.purposes.NOTICE.blockers).toEqual([])
    })
})
