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
import {
    EAccountFormError,
    IAccountFormValues,
    accountReadiness,
    credentialsPayload,
    credentialsToReenter,
    emptyAccountForm,
    endpointChanged,
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
    name: "Staff Association WhatsApp",
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
    limits: {
        messages_per_second: 20,
        otp_reserved_per_second: 5,
        otp_concurrency: 4,
        allowed_calling_codes: ["63"],
    },
    provider_approval: EProviderApproval.PENDING,
    status: {connected: true, production_access: true, approved_templates: {}},
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
        expect(senderFields(EMessagingProvider.HTTP_API)).toEqual([])
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
    it("offers only the channel's own adapters, never a generic or console provider", () => {
        expect(selectableProviders(EMessageChannel.EMAIL)).toEqual([
            EMessagingProvider.AWS_SES,
            EMessagingProvider.SMTP,
        ])
        for (const channel of MESSAGE_CHANNELS) {
            expect(selectableProviders(channel)).not.toContain(EMessagingProvider.HTTP_API)
            expect(selectableProviders(channel)).not.toContain(EMessagingProvider.CONSOLE)
            expect(selectableProviders(channel).length).toBeGreaterThan(0)
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

    it("keeps the provider and what was entered when it serves the channel", () => {
        const email = form(EMessagingProvider.SMTP, EMessageChannel.EMAIL, {
            sender: {from_address: "no-reply@example.org", from_name: "", server_url: "smtp:25"},
        })
        expect(withChannel(email, EMessageChannel.EMAIL)).toEqual(email)
    })
})

describe("upsertAccountVariables", () => {
    it("sends the chosen channel, approval only where it applies, and never a readiness", () => {
        const viber = form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
            name: " Partner ",
            sender: {base_url: "https://x.api.infobip.com", sender: "Council"},
            providerApproval: EProviderApproval.CONFIRMED,
        })
        expect(upsertAccountVariables(null, viber)).toEqual({
            id: null,
            channel: EMessageChannel.VIBER,
            name: "Partner",
            sender: senderFromForm(viber),
            limits: limitsFromForm(viber),
            providerApproval: null,
            isDefault: false,
            credentials: null,
        })
        expect(
            upsertAccountVariables(null, {...viber, credentials: {API_KEY: "key"}}).credentials
        ).toEqual({API_KEY: "key"})
        const whatsapp = formFromAccount(
            account({
                provider_approval: EProviderApproval.CONFIRMED,
                readiness: EReadinessPolicy.ADMIN_CONFIRMED,
            })
        )
        const variables = upsertAccountVariables("account-1", whatsapp)
        expect(variables).toMatchObject({
            id: "account-1",
            channel: EMessageChannel.WHATSAPP,
            providerApproval: EProviderApproval.CONFIRMED,
        })
        expect("readiness" in variables).toBe(false)
    })

    it("refuses to save an account of a removed provider: it can only be replaced", () => {
        for (const provider of [EMessagingProvider.CONSOLE, EMessagingProvider.HTTP_API]) {
            expect(validateAccountForm(form(provider, EMessageChannel.SMS))).toEqual({
                provider: EAccountFormError.UNSUPPORTED_PROVIDER,
            })
        }
    })
})

describe("senderFromForm", () => {
    it("builds the tagged sender, turning empty optional fields into null", () => {
        const values = form(EMessagingProvider.AWS_SNS, EMessageChannel.SMS, {
            sender: {sender_id: "STAFFASSOC", origination_number: " ", region: "ap-southeast-1"},
        })
        expect(senderFromForm(values)).toEqual({
            provider: EMessagingProvider.AWS_SNS,
            sender_id: "STAFFASSOC",
            origination_number: null,
            region: "ap-southeast-1",
        })
    })

    it("keeps the Viber approved templates per purpose and language", () => {
        const values = form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
            sender: {base_url: "https://x.api.infobip.com", sender: "Staff Association"},
            viberTemplates: [
                {purpose: EMessagePurpose.OTP, language: "en", templateId: "88213"},
                {purpose: EMessagePurpose.OTP, language: "tl", templateId: " 88214 "},
                {purpose: EMessagePurpose.NOTICE, language: "en", templateId: ""},
            ],
        })
        expect(senderFromForm(values)).toEqual({
            provider: EMessagingProvider.VIBER_INFOBIP,
            base_url: "https://x.api.infobip.com",
            sender: "Staff Association",
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
                otpConcurrency: "0",
                allowedCallingCodes: "63 x",
            },
        })
        expect(validateAccountForm(values)).toEqual({
            "limits.otpReservedPerSecond": EAccountFormError.OTP_ABOVE_TOTAL,
            "limits.otpConcurrency": EAccountFormError.AT_LEAST_ONE,
            "limits.allowedCallingCodes": EAccountFormError.INVALID_CALLING_CODE,
        })
        expect(
            validateAccountForm({
                ...values,
                limits: {
                    messagesPerSecond: "-1",
                    otpReservedPerSecond: "1.5",
                    otpConcurrency: "two",
                    allowedCallingCodes: "",
                },
            })
        ).toEqual({
            "limits.messagesPerSecond": EAccountFormError.NOT_A_COUNT,
            "limits.otpReservedPerSecond": EAccountFormError.NOT_A_COUNT,
            "limits.otpConcurrency": EAccountFormError.NOT_A_COUNT,
        })
    })

    it("requires a template ID and no repeated language per Viber purpose", () => {
        const values = form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
            sender: {base_url: "https://x", sender: "Staff Association"},
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
        // Endpoints carry the stored credentials: only https.
        expect(
            validateAccountForm({
                ...values,
                sender: {...values.sender, api_base_url: "http://graph.bsp.example"},
            })
        ).toEqual({"sender.api_base_url": EAccountFormError.NOT_A_URL})
        expect(
            validateAccountForm(
                form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
                    sender: {base_url: "http://x.api.infobip.com", sender: "Council"},
                })
            )
        ).toEqual({"sender.base_url": EAccountFormError.NOT_A_URL})
    })
})

describe("credentialsPayload", () => {
    it("never sends Viber's generated report credentials as typed", () => {
        const values = form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
            credentials: {API_KEY: "key", REPORT_USERNAME: "u", REPORT_PASSWORD: "p"},
        })
        expect(credentialsPayload(values)).toEqual({API_KEY: "key"})
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

    it("reads a stored ADMIN_CONFIRMED account from the provider's check", () => {
        const confirmed = account({
            readiness: EReadinessPolicy.ADMIN_CONFIRMED,
            provider_approval: EProviderApproval.CONFIRMED,
            status: null,
        })
        expect(accountReadiness(confirmed).connected).toBe(false)
        expect(accountReadiness(confirmed).purposes.OTP.blockers).toEqual([
            EReadinessBlocker.NOT_CONNECTED,
            EReadinessBlocker.NEEDS_PRODUCTION_ACCESS,
            EReadinessBlocker.NEEDS_APPROVED_TEMPLATE,
        ])
    })

    it("shows an account of a removed provider as unsupported and not connected", () => {
        const old = account({
            channel: EMessageChannel.VIBER,
            provider: EMessagingProvider.HTTP_API,
            sender: {provider: EMessagingProvider.HTTP_API, label: "Old partner"},
            status: {connected: true, production_access: true, approved_templates: {}},
        })
        const readiness = accountReadiness(old)
        expect(readiness.connected).toBe(false)
        expect(readiness.purposes.OTP.blockers).toEqual([EReadinessBlocker.UNSUPPORTED_PROVIDER])
        expect(readiness.purposes.NOTICE.blockers).toEqual([EReadinessBlocker.UNSUPPORTED_PROVIDER])
    })
})

describe("credentials follow their address (R4-01)", () => {
    const stored = form(EMessagingProvider.VIBER_INFOBIP, EMessageChannel.VIBER, {
        sender: {base_url: "https://x.api.infobip.com", sender: "Staff Association"},
    })
    const moved = {...stored, sender: {...stored.sender, base_url: "https://y.api.infobip.com"}}

    it("sees an address change, but not a trailing slash or spaces", () => {
        expect(endpointChanged(stored, moved)).toBe(true)
        expect(
            endpointChanged(stored, {
                ...stored,
                sender: {...stored.sender, base_url: " https://x.api.infobip.com/ "},
            })
        ).toBe(false)
        expect(endpointChanged(undefined, moved)).toBe(false)
    })

    it("asks for every credential the provider sends with, and refuses the form until typed", () => {
        expect(credentialsToReenter(stored, moved)).toEqual([ECredentialName.API_KEY])
        expect(validateAccountForm(moved, stored)["sender.base_url"]).toBe(
            EAccountFormError.CREDENTIALS_FOLLOW_ENDPOINT
        )
        const typed = {...moved, credentials: {[ECredentialName.API_KEY]: "new-key"}}
        expect(credentialsToReenter(stored, typed)).toEqual([])
        expect(validateAccountForm(typed, stored)).toEqual({})
    })

    it("treats a Graph API base set for the first time, and an SMTP server, as address changes", () => {
        const page = form(EMessagingProvider.MESSENGER_SEND_API, EMessageChannel.MESSENGER, {
            sender: {page_id: "1", api_version: "v23.0", api_base_url: ""},
        })
        const bsp = {...page, sender: {...page.sender, api_base_url: "https://graph.bsp.example"}}
        expect(credentialsToReenter(page, bsp)).toEqual([ECredentialName.ACCESS_TOKEN])
        const smtp = form(EMessagingProvider.SMTP, EMessageChannel.EMAIL, {
            sender: {from_address: "a@b.example", server_url: "smtp.example:587"},
        })
        const moved = {...smtp, sender: {...smtp.sender, server_url: "smtp2.example:587"}}
        expect(credentialsToReenter(smtp, moved)).toEqual([ECredentialName.SMTP_PASSWORD])
    })
})
