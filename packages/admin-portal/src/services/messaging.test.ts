// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EDeliveryFeedback,
    EMessageAttemptState,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    EReadinessBlocker,
    ECredentialName,
    IAccountCheck,
    IAccountSummary,
    IEventChannelConfig,
    IEventMessagingConfig,
    MESSAGE_CHANNELS,
} from "@/types/messaging"
import {
    acceptedCredentials,
    accountSenderLabel,
    channelStatisticsKey,
    deliverySummary,
    emptyEventMessagingConfig,
    enabledChannels,
    parseEventMessagingConfig,
    providerCapabilities,
    providerChannel,
    providersForChannel,
    purposeReadiness,
    recipientKind,
    requiredCredentials,
    validateEventMessagingConfig,
} from "./messaging"

const TENANT = "tenant-1"

const readyCheck = (templates: Array<[EMessagePurpose, string]>): IAccountCheck => {
    const approved_templates: Partial<Record<EMessagePurpose, string[]>> = {}
    for (const [purpose, language] of templates) {
        approved_templates[purpose] = [...(approved_templates[purpose] ?? []), language]
    }
    return {
        connected: true,
        production_access: true,
        approved_templates,
        checked_at: "2026-10-02T00:00:00Z",
        reason: null,
    }
}

const account = (
    id: string,
    channel: EMessageChannel,
    provider: EMessagingProvider,
    check: IAccountCheck
): IAccountSummary => ({
    id,
    tenant_id: TENANT,
    channel,
    provider,
    provider_approval: EProviderApproval.PENDING,
    check,
})

const channelConfig = (
    channel: EMessageChannel,
    account_id: string,
    purposes: EMessagePurpose[]
): IEventChannelConfig => ({
    channel,
    account_id,
    purposes,
    templates: [],
    out_of_window: EOutOfWindowPolicy.DISABLED,
})

const config = (overrides: Partial<IEventMessagingConfig>): IEventMessagingConfig => ({
    ...emptyEventMessagingConfig(),
    ...overrides,
})

describe("provider capabilities", () => {
    it("every provider declares capabilities for its own channel only", () => {
        for (const provider of Object.values(EMessagingProvider)) {
            for (const channel of MESSAGE_CHANNELS) {
                const capabilities = providerCapabilities(provider, channel)
                const own = providerChannel(provider)
                if (own && own !== channel) {
                    expect(capabilities).toBeNull()
                } else {
                    expect(capabilities?.channel).toBe(channel)
                    expect(capabilities?.recipient).toBe(recipientKind(channel))
                }
            }
        }
    })

    it("SMS and SMTP report no delivery receipts", () => {
        expect(
            providerCapabilities(EMessagingProvider.AWS_SNS, EMessageChannel.SMS)?.delivery_feedback
        ).toBe(EDeliveryFeedback.UNAVAILABLE)
        expect(
            providerCapabilities(EMessagingProvider.SMTP, EMessageChannel.EMAIL)?.delivery_feedback
        ).toBe(EDeliveryFeedback.UNAVAILABLE)
        expect(
            providerCapabilities(EMessagingProvider.AWS_SES, EMessageChannel.EMAIL)
                ?.delivery_feedback
        ).toBe(EDeliveryFeedback.PROVIDER_RECEIPTS)
    })

    it("lists the providers of a channel, with the console for any channel", () => {
        expect(providersForChannel(EMessageChannel.WHATSAPP)).toEqual([
            EMessagingProvider.WHATSAPP_CLOUD_API,
            EMessagingProvider.CONSOLE,
        ])
    })
})

describe("purposeReadiness", () => {
    it("connected WhatsApp is not ready without approval and templates", () => {
        const readiness = purposeReadiness(
            EMessagingProvider.WHATSAPP_CLOUD_API,
            EMessageChannel.WHATSAPP,
            EProviderApproval.PENDING,
            readyCheck([]),
            EMessagePurpose.OTP,
            "en"
        )
        expect(readiness.blockers).toEqual([
            EReadinessBlocker.NEEDS_PROVIDER_APPROVAL,
            EReadinessBlocker.NEEDS_APPROVED_TEMPLATE,
        ])
    })

    it("checks templates per language", () => {
        const check = readyCheck([[EMessagePurpose.OTP, "en"]])
        const readiness = (language: string) =>
            purposeReadiness(
                EMessagingProvider.VIBER_INFOBIP,
                EMessageChannel.VIBER,
                EProviderApproval.PENDING,
                check,
                EMessagePurpose.OTP,
                language
            )
        expect(readiness("en").blockers).toEqual([])
        expect(readiness("tl").blockers).toEqual([EReadinessBlocker.NEEDS_APPROVED_TEMPLATE])
    })

    it("not connected blocks every purpose", () => {
        const readiness = purposeReadiness(
            EMessagingProvider.AWS_SNS,
            EMessageChannel.SMS,
            EProviderApproval.PENDING,
            {connected: false, production_access: false},
            EMessagePurpose.NOTICE,
            null
        )
        expect(readiness.blockers).toEqual([
            EReadinessBlocker.NOT_CONNECTED,
            EReadinessBlocker.NEEDS_PRODUCTION_ACCESS,
        ])
    })

    it("treats an account never checked as not connected", () => {
        const readiness = purposeReadiness(
            EMessagingProvider.AWS_SES,
            EMessageChannel.EMAIL,
            EProviderApproval.PENDING,
            null,
            EMessagePurpose.OTP,
            null
        )
        expect(readiness.blockers).toContain(EReadinessBlocker.NOT_CONNECTED)
    })

    it("a provider used on another channel is unsupported", () => {
        const readiness = purposeReadiness(
            EMessagingProvider.AWS_SES,
            EMessageChannel.SMS,
            EProviderApproval.CONFIRMED,
            readyCheck([]),
            EMessagePurpose.OTP,
            null
        )
        expect(readiness.blockers).toEqual([EReadinessBlocker.UNSUPPORTED_PURPOSE])
    })

    it("confirmed WhatsApp with an approved template in any language is ready", () => {
        const readiness = purposeReadiness(
            EMessagingProvider.WHATSAPP_CLOUD_API,
            EMessageChannel.WHATSAPP,
            EProviderApproval.CONFIRMED,
            readyCheck([[EMessagePurpose.NOTICE, "tl"]]),
            EMessagePurpose.NOTICE,
            null
        )
        expect(readiness.blockers).toEqual([])
    })
})

describe("validateEventMessagingConfig", () => {
    it("accepts a valid configuration", () => {
        const accounts = [
            account("sms-1", EMessageChannel.SMS, EMessagingProvider.AWS_SNS, readyCheck([])),
            account(
                "viber-1",
                EMessageChannel.VIBER,
                EMessagingProvider.VIBER_INFOBIP,
                readyCheck([
                    [EMessagePurpose.OTP, "en"],
                    [EMessagePurpose.NOTICE, "en"],
                ])
            ),
        ]
        const valid = config({
            channels: [
                channelConfig(EMessageChannel.SMS, "sms-1", [
                    EMessagePurpose.OTP,
                    EMessagePurpose.NOTICE,
                ]),
                {
                    ...channelConfig(EMessageChannel.VIBER, "viber-1", [
                        EMessagePurpose.OTP,
                        EMessagePurpose.NOTICE,
                    ]),
                    templates: [
                        {purpose: EMessagePurpose.OTP, language: "en", provider_template: "otp_en"},
                    ],
                },
            ],
            notice_fallback: [EMessageChannel.VIBER, EMessageChannel.SMS],
            election_channels: {"election-1": [EMessageChannel.SMS]},
        })
        expect(validateEventMessagingConfig(valid, TENANT, accounts, ["election-1"])).toEqual([])
        expect(enabledChannels(valid, EMessagePurpose.OTP, "election-1")).toEqual([
            EMessageChannel.SMS,
        ])
        expect(enabledChannels(valid, EMessagePurpose.OTP, "election-2")).toEqual([
            EMessageChannel.SMS,
            EMessageChannel.VIBER,
        ])
    })

    it("reports invalid references", () => {
        const foreign = {
            ...account(
                "sms-foreign",
                EMessageChannel.SMS,
                EMessagingProvider.AWS_SNS,
                readyCheck([])
            ),
            tenant_id: "tenant-2",
        }
        const accounts = [
            foreign,
            account("email-1", EMessageChannel.EMAIL, EMessagingProvider.AWS_SES, readyCheck([])),
        ]
        const invalid = config({
            version: 2,
            channels: [
                channelConfig(EMessageChannel.SMS, "sms-foreign", [EMessagePurpose.OTP]),
                channelConfig(EMessageChannel.WHATSAPP, "missing", [EMessagePurpose.OTP]),
                channelConfig(EMessageChannel.VIBER, "email-1", [EMessagePurpose.OTP]),
                channelConfig(EMessageChannel.VIBER, "email-1", [EMessagePurpose.OTP]),
            ],
            notice_fallback: [EMessageChannel.SMS],
            election_channels: {"unknown-election": [EMessageChannel.SMS]},
        })
        expect(validateEventMessagingConfig(invalid, TENANT, accounts, ["election-1"])).toEqual([
            {kind: "UNSUPPORTED_VERSION", version: 2},
            {kind: "ACCOUNT_OF_ANOTHER_TENANT", account_id: "sms-foreign"},
            {kind: "UNKNOWN_ACCOUNT", channel: EMessageChannel.WHATSAPP, account_id: "missing"},
            {
                kind: "ACCOUNT_CHANNEL_MISMATCH",
                channel: EMessageChannel.VIBER,
                account_id: "email-1",
            },
            {kind: "DUPLICATE_CHANNEL", channel: EMessageChannel.VIBER},
            {kind: "FALLBACK_CHANNEL_NOT_ENABLED", channel: EMessageChannel.SMS},
            {kind: "UNKNOWN_ELECTION", election_id: "unknown-election"},
        ])
    })

    it("names the missing prerequisites of an unready purpose", () => {
        const accounts = [
            account(
                "wa-1",
                EMessageChannel.WHATSAPP,
                EMessagingProvider.WHATSAPP_CLOUD_API,
                readyCheck([[EMessagePurpose.NOTICE, "en"]])
            ),
        ]
        const invalid = config({
            channels: [
                {
                    ...channelConfig(EMessageChannel.WHATSAPP, "wa-1", [EMessagePurpose.OTP]),
                    templates: [
                        {purpose: EMessagePurpose.OTP, language: "tl", provider_template: "otp_tl"},
                    ],
                    out_of_window: EOutOfWindowPolicy.UTILITY_MESSAGES,
                },
            ],
        })
        expect(validateEventMessagingConfig(invalid, TENANT, accounts, [])).toEqual([
            {
                kind: "PURPOSE_NOT_READY",
                channel: EMessageChannel.WHATSAPP,
                purpose: EMessagePurpose.OTP,
                blockers: [
                    EReadinessBlocker.NEEDS_PROVIDER_APPROVAL,
                    EReadinessBlocker.NEEDS_APPROVED_TEMPLATE,
                ],
            },
            {
                kind: "TEMPLATE_NOT_APPROVED",
                channel: EMessageChannel.WHATSAPP,
                purpose: EMessagePurpose.OTP,
                language: "tl",
            },
            {kind: "OUT_OF_WINDOW_NOT_SUPPORTED", channel: EMessageChannel.WHATSAPP},
        ])
    })

    it("requires election restrictions to use enabled channels", () => {
        const accounts = [
            account("sms-1", EMessageChannel.SMS, EMessagingProvider.AWS_SNS, readyCheck([])),
        ]
        const invalid = config({
            channels: [channelConfig(EMessageChannel.SMS, "sms-1", [EMessagePurpose.NOTICE])],
            election_channels: {"election-1": [EMessageChannel.VIBER]},
            notice_fallback: [EMessageChannel.SMS, EMessageChannel.SMS],
        })
        expect(validateEventMessagingConfig(invalid, TENANT, accounts, ["election-1"])).toEqual([
            {kind: "DUPLICATE_FALLBACK_CHANNEL", channel: EMessageChannel.SMS},
            {
                kind: "ELECTION_CHANNEL_NOT_ENABLED",
                election_id: "election-1",
                channel: EMessageChannel.VIBER,
            },
        ])
    })
})

describe("parseEventMessagingConfig", () => {
    it("fills defaults", () => {
        const parsed = parseEventMessagingConfig(
            JSON.stringify({version: 1, channels: [{channel: "SMS", account_id: "a"}]})
        )
        expect(parsed.channels[0].purposes).toEqual([])
        expect(parsed.channels[0].templates).toEqual([])
        expect(parsed.channels[0].out_of_window).toBe(EOutOfWindowPolicy.DISABLED)
        expect(parsed.notice_fallback).toEqual([])
        expect(parsed.election_channels).toEqual({})
        expect(parsed.reply_text).toEqual({})
    })

    it("returns an empty configuration for a missing or malformed annotation", () => {
        expect(parseEventMessagingConfig(undefined)).toEqual(emptyEventMessagingConfig())
        expect(parseEventMessagingConfig("{not json")).toEqual(emptyEventMessagingConfig())
        expect(parseEventMessagingConfig("[]")).toEqual(emptyEventMessagingConfig())
    })

    it("accepts an already parsed object", () => {
        expect(parseEventMessagingConfig({version: 1, channels: []}).version).toBe(1)
    })
})

describe("credentials", () => {
    it("required credentials are always accepted", () => {
        for (const provider of Object.values(EMessagingProvider)) {
            const accepted = acceptedCredentials(provider)
            for (const credential of requiredCredentials(provider)) {
                expect(accepted).toContain(credential)
            }
        }
    })

    it("AWS keys are optional", () => {
        expect(requiredCredentials(EMessagingProvider.AWS_SNS)).toEqual([])
        expect(acceptedCredentials(EMessagingProvider.AWS_SNS)).toEqual([
            ECredentialName.AWS_ACCESS_KEY_ID,
            ECredentialName.AWS_SECRET_ACCESS_KEY,
        ])
    })
})

describe("statistics and labels", () => {
    it("keeps the existing statistics keys", () => {
        expect(channelStatisticsKey(EMessageChannel.EMAIL)).toBe("num_emails_sent")
        expect(channelStatisticsKey(EMessageChannel.SMS)).toBe("num_sms_sent")
        expect(new Set(MESSAGE_CHANNELS.map(channelStatisticsKey)).size).toBe(5)
    })

    it("labels a sender like the public label", () => {
        expect(
            accountSenderLabel({
                provider: EMessagingProvider.MESSENGER_SEND_API,
                page_id: "1234",
                page_name: "COMELEC",
                api_version: "v23.0",
            })
        ).toBe("COMELEC")
        expect(
            accountSenderLabel({
                provider: EMessagingProvider.AWS_SES,
                from_address: "no-reply@example.com",
            })
        ).toBe("no-reply@example.com")
        expect(accountSenderLabel({provider: EMessagingProvider.CONSOLE})).toBeNull()
    })
})

describe("deliverySummary", () => {
    it("never reports zero delivered when the provider has no receipts", () => {
        const summary = deliverySummary(
            [
                {state: EMessageAttemptState.ACCEPTED, count: 4},
                {state: EMessageAttemptState.FAILED, count: 1},
            ],
            EDeliveryFeedback.UNAVAILABLE
        )
        expect(summary.delivered).toBeNull()
        expect(summary.accepted).toBe(4)
        expect(summary.failed).toBe(1)
        expect(summary.unknown).toBe(0)
    })

    it("counts every state when receipts are reported", () => {
        const summary = deliverySummary(
            [
                {state: EMessageAttemptState.QUEUED, count: 2},
                {state: EMessageAttemptState.DELIVERED, count: 3},
                {state: EMessageAttemptState.UNKNOWN, count: 1},
                {state: EMessageAttemptState.DELIVERED, count: 1},
            ],
            EDeliveryFeedback.PROVIDER_RECEIPTS
        )
        expect(summary).toEqual({queued: 2, accepted: 0, delivered: 4, failed: 0, unknown: 1})
    })
})
