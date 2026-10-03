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
    EPhoneFormat,
    EReadinessBlocker,
    EReadinessPolicy,
    ECredentialName,
    ERecipientKind,
    IAccountCheck,
    IAccountSender,
    IAccountSummary,
    IEventChannelConfig,
    IEventMessagingConfig,
    IMessagingAccount,
    ITemplateBinding,
    MESSAGE_CHANNELS,
} from "@/types/messaging"
import {
    acceptedCredentials,
    accountSenderLabel,
    accountSummary,
    bindingProviderLanguage,
    channelStatisticsKey,
    deliverySummary,
    emptyEventMessagingConfig,
    enabledChannels,
    parseEventMessagingConfig,
    providerCapabilities,
    providerChannel,
    providersForChannel,
    publicProjection,
    purposeReadiness,
    recipientKind,
    requiredCredentials,
    senderCapabilities,
    supportsOutOfWindow,
    templateFor,
    validateEventMessagingConfig,
    webhookPath,
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
    capabilities: providerCapabilities(provider, channel),
    provider_approval: EProviderApproval.PENDING,
    readiness: EReadinessPolicy.PROVIDER_CHECK,
    check,
    public_label: `${id} label`,
    messenger_page: null,
})

const readinessOf = (
    provider: EMessagingProvider,
    channel: EMessageChannel,
    approval: EProviderApproval,
    check: IAccountCheck | null,
    purpose: EMessagePurpose,
    language: string | null
) =>
    purposeReadiness(
        providerCapabilities(provider, channel),
        approval,
        EReadinessPolicy.PROVIDER_CHECK,
        check,
        purpose,
        language
    )

const binding = (key: string | null, language: string, template: string): ITemplateBinding => ({
    purpose: EMessagePurpose.NOTICE,
    key,
    language,
    provider_template: template,
    provider_language: null,
})

const VIBER_PARTNER: IAccountSender = {
    provider: EMessagingProvider.HTTP_API,
    label: "COMELEC",
    send: {
        url: "https://partner.example/v1/viber",
        headers: {Authorization: "Bearer {{credential.API_KEY}}"},
        body: {to: "{{to}}", template: "{{template}}", params: "{{parameters}}"},
    },
    message_id_pointer: "/id",
    phone_format: EPhoneFormat.DIGITS,
    template_required_for: [EMessagePurpose.OTP],
    reports: {
        auth: {kind: "HEADER_SECRET", header: "X-Secret"},
        status: {
            message_id_pointer: "/id",
            state_pointer: "/status",
            states: {
                delivered: EMessageAttemptState.DELIVERED,
                failed: EMessageAttemptState.FAILED,
            },
        },
    },
}

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
            EMessagingProvider.HTTP_API,
            EMessagingProvider.CONSOLE,
        ])
    })

    it("a configured HTTP provider declares its own capabilities", () => {
        const capabilities = senderCapabilities(VIBER_PARTNER, EMessageChannel.VIBER)
        expect(capabilities?.recipient).toBe(ERecipientKind.PHONE_NUMBER)
        expect(capabilities?.template_required_for).toEqual([EMessagePurpose.OTP])
        expect(capabilities?.delivery_feedback).toBe(EDeliveryFeedback.PROVIDER_RECEIPTS)
        expect(capabilities?.reconciliation).toBe(false)
        expect(capabilities?.conversation_window_hours).toBeNull()
        expect(accountSenderLabel(VIBER_PARTNER)).toBe("COMELEC")
        expect(webhookPath(EMessagingProvider.HTTP_API, "key-1")).toBe("/webhooks/http/key-1")
    })

    it("a configured HTTP provider without reports has no receipts and may reconcile", () => {
        const sender: IAccountSender = {
            provider: EMessagingProvider.HTTP_API,
            send: {url: "https://gateway.example/sms"},
            conversation_window_hours: 24,
            reconcile: {
                request: {method: "GET", url: "https://gateway.example/sms/{{message_id}}"},
                status: {message_id_pointer: "/id", state_pointer: "/status", states: {}},
            },
        }
        const capabilities = senderCapabilities(sender, EMessageChannel.SMS)
        expect(capabilities?.template_required_for).toEqual([])
        expect(capabilities?.delivery_feedback).toBe(EDeliveryFeedback.UNAVAILABLE)
        expect(capabilities?.reconciliation).toBe(true)
        expect(capabilities?.conversation_window_hours).toBe(24)
        expect(accountSenderLabel(sender)).toBeNull()
    })

    it("other providers keep their provider's capabilities", () => {
        expect(
            senderCapabilities(
                {provider: EMessagingProvider.AWS_SNS, sender_id: "COUNCIL"},
                EMessageChannel.SMS
            )
        ).toEqual(providerCapabilities(EMessagingProvider.AWS_SNS, EMessageChannel.SMS))
        expect(
            senderCapabilities({provider: EMessagingProvider.AWS_SNS}, EMessageChannel.EMAIL)
        ).toBeNull()
    })

    it("only a channel with a window and free-text notices can send outside it", () => {
        const capabilities = (provider: EMessagingProvider, channel: EMessageChannel) =>
            providerCapabilities(provider, channel)
        expect(
            supportsOutOfWindow(
                capabilities(EMessagingProvider.MESSENGER_SEND_API, EMessageChannel.MESSENGER)
            )
        ).toBe(true)
        expect(
            supportsOutOfWindow(
                capabilities(EMessagingProvider.WHATSAPP_CLOUD_API, EMessageChannel.WHATSAPP)
            )
        ).toBe(false)
        expect(
            supportsOutOfWindow(capabilities(EMessagingProvider.AWS_SNS, EMessageChannel.SMS))
        ).toBe(false)
        expect(
            supportsOutOfWindow(
                senderCapabilities(
                    {...VIBER_PARTNER, conversation_window_hours: 24},
                    EMessageChannel.VIBER
                )
            )
        ).toBe(true)
        expect(supportsOutOfWindow(null)).toBe(false)
    })
})

describe("templateFor", () => {
    const bound = config({
        channels: [
            {
                ...channelConfig(EMessageChannel.WHATSAPP, "wa", []),
                templates: [
                    binding(null, "en", "default_en"),
                    binding(null, "tl", "default_tl"),
                    binding("reminder", "en", "reminder_en"),
                    {...binding("reminder", "tl", "reminder_tl"), provider_language: "fil"},
                ],
            },
        ],
    })
    const template = (key: string | null, language: string | null) =>
        templateFor(bound, EMessageChannel.WHATSAPP, EMessagePurpose.NOTICE, key, language)
            ?.provider_template ?? null

    it("the most specific template binding wins", () => {
        expect(template("reminder", "tl")).toBe("reminder_tl")
        expect(template("reminder", "fil")).toBe("reminder_tl")
        expect(template("reminder", "fr")).toBe("reminder_en")
        expect(template("reminder", null)).toBe("reminder_en")
        expect(template("approved", "tl")).toBe("default_tl")
        expect(template(null, null)).toBe("default_en")
    })

    it("finds nothing for a purpose or channel without bindings", () => {
        expect(
            templateFor(bound, EMessageChannel.WHATSAPP, EMessagePurpose.OTP, null, "en")
        ).toBeNull()
        expect(
            templateFor(bound, EMessageChannel.VIBER, EMessagePurpose.NOTICE, null, "en")
        ).toBeNull()
    })

    it("sends the provider's language code when it differs", () => {
        const reminder = templateFor(
            bound,
            EMessageChannel.WHATSAPP,
            EMessagePurpose.NOTICE,
            "reminder",
            "tl"
        )
        expect(reminder && bindingProviderLanguage(reminder)).toBe("fil")
        expect(bindingProviderLanguage(binding(null, "en", "default_en"))).toBe("en")
    })
})

describe("purposeReadiness", () => {
    it("connected WhatsApp is not ready without approval and templates", () => {
        const readiness = readinessOf(
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
            readinessOf(
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
        const readiness = readinessOf(
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
        const readiness = readinessOf(
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
        const readiness = readinessOf(
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
        const readiness = readinessOf(
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

describe("readiness confirmed by an administrator", () => {
    const whatsapp = providerCapabilities(
        EMessagingProvider.WHATSAPP_CLOUD_API,
        EMessageChannel.WHATSAPP
    )

    it("replaces the provider check only", () => {
        const readiness = (approval: EProviderApproval) =>
            purposeReadiness(
                whatsapp,
                approval,
                EReadinessPolicy.ADMIN_CONFIRMED,
                {connected: false, production_access: false},
                EMessagePurpose.OTP,
                "en"
            )
        expect(readiness(EProviderApproval.PENDING).blockers).toEqual([
            EReadinessBlocker.NEEDS_PROVIDER_APPROVAL,
        ])
        expect(readiness(EProviderApproval.CONFIRMED).blockers).toEqual([])
    })

    it("never makes a provider serve another channel", () => {
        expect(
            purposeReadiness(
                providerCapabilities(EMessagingProvider.AWS_SES, EMessageChannel.SMS),
                EProviderApproval.CONFIRMED,
                EReadinessPolicy.ADMIN_CONFIRMED,
                null,
                EMessagePurpose.OTP,
                null
            ).blockers
        ).toEqual([EReadinessBlocker.UNSUPPORTED_PURPOSE])
    })

    it("treats an account without a policy as checked by the provider", () => {
        expect(
            purposeReadiness(
                providerCapabilities(EMessagingProvider.AWS_SNS, EMessageChannel.SMS),
                null,
                null,
                null,
                EMessagePurpose.OTP,
                null
            ).blockers
        ).toEqual([EReadinessBlocker.NOT_CONNECTED, EReadinessBlocker.NEEDS_PRODUCTION_ACCESS])
    })

    it("skips the template approvals of the check when validating", () => {
        const confirmed: IAccountSummary = {
            ...account("wa-1", EMessageChannel.WHATSAPP, EMessagingProvider.WHATSAPP_CLOUD_API, {
                connected: false,
                production_access: false,
            }),
            provider_approval: EProviderApproval.CONFIRMED,
            readiness: EReadinessPolicy.ADMIN_CONFIRMED,
        }
        const valid = config({
            channels: [
                {
                    ...channelConfig(EMessageChannel.WHATSAPP, "wa-1", [EMessagePurpose.OTP]),
                    templates: [{...binding(null, "tl", "otp_tl"), purpose: EMessagePurpose.OTP}],
                },
            ],
        })
        expect(validateEventMessagingConfig(valid, TENANT, [confirmed], [])).toEqual([])
        expect(
            validateEventMessagingConfig(
                valid,
                TENANT,
                [{...confirmed, provider_approval: EProviderApproval.PENDING}],
                []
            )
        ).toEqual([
            {
                kind: "PURPOSE_NOT_READY",
                channel: EMessageChannel.WHATSAPP,
                purpose: EMessagePurpose.OTP,
                blockers: [EReadinessBlocker.NEEDS_PROVIDER_APPROVAL],
            },
        ])
    })
})

describe("template and out-of-window validation", () => {
    it("accepts a template approved under the provider's language code", () => {
        const viber = account(
            "viber-1",
            EMessageChannel.VIBER,
            EMessagingProvider.VIBER_INFOBIP,
            readyCheck([[EMessagePurpose.NOTICE, "en_US"]])
        )
        const bound = (providerLanguage: string | null) =>
            config({
                channels: [
                    {
                        ...channelConfig(EMessageChannel.VIBER, "viber-1", [
                            EMessagePurpose.NOTICE,
                        ]),
                        templates: [
                            {...binding(null, "en", "notice"), provider_language: providerLanguage},
                        ],
                    },
                ],
                notice_fallback: [EMessageChannel.VIBER],
            })
        expect(validateEventMessagingConfig(bound("en_US"), TENANT, [viber], [])).toEqual([])
        expect(validateEventMessagingConfig(bound(null), TENANT, [viber], [])).toEqual([
            {
                kind: "TEMPLATE_NOT_APPROVED",
                channel: EMessageChannel.VIBER,
                purpose: EMessagePurpose.NOTICE,
                language: "en",
            },
        ])
    })

    it("lets Messenger send utility messages outside the window", () => {
        const messenger = account(
            "page-1",
            EMessageChannel.MESSENGER,
            EMessagingProvider.MESSENGER_SEND_API,
            readyCheck([])
        )
        const utility = config({
            channels: [
                {
                    ...channelConfig(EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]),
                    templates: [binding(null, "en", "vote_reminder")],
                    out_of_window: EOutOfWindowPolicy.UTILITY_MESSAGES,
                },
            ],
        })
        expect(validateEventMessagingConfig(utility, TENANT, [messenger], [])).toEqual([])
    })

    it("uses the capabilities a configured provider declares", () => {
        const partner = (sender: IAccountSender): IAccountSummary =>
            accountSummary({
                id: "http-1",
                tenant_id: TENANT,
                channel: EMessageChannel.VIBER,
                provider: EMessagingProvider.HTTP_API,
                name: "Partner",
                sender,
                status: readyCheck([]),
                is_default: false,
            })
        const utility = config({
            channels: [
                {
                    ...channelConfig(EMessageChannel.VIBER, "http-1", [
                        EMessagePurpose.OTP,
                        EMessagePurpose.NOTICE,
                    ]),
                    out_of_window: EOutOfWindowPolicy.UTILITY_MESSAGES,
                },
            ],
        })
        expect(validateEventMessagingConfig(utility, TENANT, [partner(VIBER_PARTNER)], [])).toEqual(
            [
                {
                    kind: "PURPOSE_NOT_READY",
                    channel: EMessageChannel.VIBER,
                    purpose: EMessagePurpose.OTP,
                    blockers: [EReadinessBlocker.NEEDS_APPROVED_TEMPLATE],
                },
                {kind: "OUT_OF_WINDOW_NOT_SUPPORTED", channel: EMessageChannel.VIBER},
            ]
        )
        expect(
            validateEventMessagingConfig(
                utility,
                TENANT,
                [
                    partner({
                        ...VIBER_PARTNER,
                        template_required_for: [],
                        conversation_window_hours: 24,
                    }),
                ],
                []
            )
        ).toEqual([])
    })
})

describe("accountSummary", () => {
    const row: IMessagingAccount = {
        id: "page-1",
        tenant_id: TENANT,
        channel: EMessageChannel.MESSENGER,
        provider: EMessagingProvider.MESSENGER_SEND_API,
        name: "Page",
        sender: {
            provider: EMessagingProvider.MESSENGER_SEND_API,
            page_id: "1234567890",
            page_name: "COMELEC",
            page_username: "comelec",
            api_version: "v23.0",
        },
        is_default: true,
    }

    it("fills the defaults of an account that was never checked", () => {
        const summary = accountSummary(row)
        expect(summary.readiness).toBe(EReadinessPolicy.PROVIDER_CHECK)
        expect(summary.provider_approval).toBe(EProviderApproval.PENDING)
        expect(summary.check).toEqual({connected: false, production_access: false})
        expect(summary.capabilities?.conversation_window_hours).toBe(24)
        expect(summary.public_label).toBe("COMELEC")
        expect(summary.messenger_page).toEqual({
            page_id: "1234567890",
            username: "comelec",
            name: "COMELEC",
        })
    })

    it("keeps the readiness the administrator chose", () => {
        expect(
            accountSummary({...row, readiness: EReadinessPolicy.ADMIN_CONFIRMED}).readiness
        ).toBe(EReadinessPolicy.ADMIN_CONFIRMED)
    })
})

describe("publicProjection", () => {
    it("contains no account references and labels only restricted elections", () => {
        const messenger: IAccountSummary = {
            ...account(
                "messenger-account-secret-id",
                EMessageChannel.MESSENGER,
                EMessagingProvider.MESSENGER_SEND_API,
                readyCheck([])
            ),
            public_label: "COMELEC",
            messenger_page: {page_id: "1234567890", username: "comelec", name: "COMELEC"},
        }
        const projected = publicProjection(
            config({
                channels: [
                    channelConfig(EMessageChannel.MESSENGER, "messenger-account-secret-id", [
                        EMessagePurpose.OTP,
                    ]),
                    channelConfig(EMessageChannel.SMS, "sms-1", []),
                ],
                election_channels: {"election-1": [EMessageChannel.MESSENGER]},
            }),
            [messenger],
            {"election-1": ["Manila", "PH-MNL"], "election-2": ["Madrid"]}
        )
        expect(JSON.stringify(projected)).not.toContain("messenger-account-secret-id")
        expect(projected.channels).toEqual([
            {
                channel: EMessageChannel.MESSENGER,
                purposes: [EMessagePurpose.OTP],
                sender_label: "COMELEC",
                messenger_page: {page_id: "1234567890", username: "comelec", name: "COMELEC"},
            },
        ])
        expect(projected.election_labels).toEqual({"election-1": ["Manila", "PH-MNL"]})
    })
})

describe("credentials of the v2 providers", () => {
    it("WhatsApp and Messenger need only the access token to send", () => {
        for (const provider of [
            EMessagingProvider.WHATSAPP_CLOUD_API,
            EMessagingProvider.MESSENGER_SEND_API,
        ]) {
            expect(requiredCredentials(provider)).toEqual([ECredentialName.ACCESS_TOKEN])
            expect(acceptedCredentials(provider)).toEqual([
                ECredentialName.ACCESS_TOKEN,
                ECredentialName.APP_SECRET,
                ECredentialName.VERIFY_TOKEN,
            ])
        }
    })

    it("a configured HTTP provider requires none and accepts its own list", () => {
        expect(requiredCredentials(EMessagingProvider.HTTP_API)).toEqual([])
        expect(acceptedCredentials(EMessagingProvider.HTTP_API)).toEqual([
            ECredentialName.API_KEY,
            ECredentialName.API_SECRET,
            ECredentialName.ACCESS_TOKEN,
            ECredentialName.USERNAME,
            ECredentialName.PASSWORD,
            ECredentialName.WEBHOOK_SECRET,
        ])
    })
})
