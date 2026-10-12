// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EDeliveryFeedback,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    EReadinessBlocker,
    EReadinessPolicy,
    ECredentialName,
    IAccountCheck,
    IAccountSummary,
    IEventChannelConfig,
    IEventMessagingConfig,
    IMessagingAccount,
    ITemplateBinding,
    MESSAGE_CHANNELS,
} from "@/types/messaging"
import {
    acceptedCredentials,
    accountDeliveryFeedback,
    accountIdentity,
    checkedForAnotherIdentity,
    checkIsFor,
    accountSenderLabel,
    accountSummary,
    bindingProviderLanguage,
    withCallbackStatus,
    channelStatisticsKey,
    deliverySummary,
    emptyEventMessagingConfig,
    enabledChannels,
    hasWebhook,
    isUnsupportedProvider,
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
    typedCredentials,
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
    capabilities: providerCapabilities(provider, channel),
    provider_approval: EProviderApproval.PENDING,
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
        provider,
        providerCapabilities(provider, channel),
        approval,
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
                if (own !== channel) {
                    expect(capabilities).toBeNull()
                } else {
                    expect(capabilities?.channel).toBe(channel)
                    expect(capabilities?.recipient).toBe(recipientKind(channel))
                }
            }
        }
    })

    it("SMS, SMTP and SES without a notification topic report no delivery receipts", () => {
        expect(
            providerCapabilities(EMessagingProvider.AWS_SNS, EMessageChannel.SMS)?.delivery_feedback
        ).toBe(EDeliveryFeedback.UNAVAILABLE)
        expect(
            providerCapabilities(EMessagingProvider.SMTP, EMessageChannel.EMAIL)?.delivery_feedback
        ).toBe(EDeliveryFeedback.UNAVAILABLE)
        // AP-B4: SES reports deliveries only through its notification topic.
        const ses = (topic: string | null) =>
            senderCapabilities(
                {
                    provider: EMessagingProvider.AWS_SES,
                    from_address: "no-reply@example.org",
                    notification_topic_arn: topic,
                },
                EMessageChannel.EMAIL
            )?.delivery_feedback
        expect(ses(null)).toBe(EDeliveryFeedback.UNAVAILABLE)
        expect(ses("  ")).toBe(EDeliveryFeedback.UNAVAILABLE)
        expect(ses("arn:aws:sns:eu-west-1:000000000000:ses-events")).toBe(
            EDeliveryFeedback.PROVIDER_RECEIPTS
        )
    })

    it("lists only the channel's own adapters for a new account", () => {
        expect(providersForChannel(EMessageChannel.WHATSAPP)).toEqual([
            EMessagingProvider.WHATSAPP_CLOUD_API,
        ])
        expect(providersForChannel(EMessageChannel.EMAIL)).toEqual([
            EMessagingProvider.AWS_SES,
            EMessagingProvider.SMTP,
        ])
    })

    it("treats the removed providers as unsupported on every channel", () => {
        for (const provider of [EMessagingProvider.HTTP_API, EMessagingProvider.CONSOLE]) {
            expect(isUnsupportedProvider(provider)).toBe(true)
            expect(providerChannel(provider)).toBeNull()
            expect(hasWebhook(provider)).toBe(false)
            for (const channel of MESSAGE_CHANNELS) {
                expect(providerCapabilities(provider, channel)).toBeNull()
                expect(providersForChannel(channel)).not.toContain(provider)
            }
        }
        expect(accountSenderLabel({provider: EMessagingProvider.HTTP_API, label: "Old"})).toBeNull()
    })

    it("has a callback URL for Meta, Viber and SES accounts", () => {
        expect(
            Object.values(EMessagingProvider).filter((provider) => hasWebhook(provider))
        ).toEqual([
            EMessagingProvider.AWS_SES,
            EMessagingProvider.WHATSAPP_CLOUD_API,
            EMessagingProvider.MESSENGER_SEND_API,
            EMessagingProvider.VIBER_INFOBIP,
        ])
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

    it("an account of a removed provider is unsupported, whatever its check says", () => {
        for (const provider of [EMessagingProvider.HTTP_API, EMessagingProvider.CONSOLE]) {
            expect(
                readinessOf(
                    provider,
                    EMessageChannel.SMS,
                    EProviderApproval.CONFIRMED,
                    readyCheck([[EMessagePurpose.OTP, "en"]]),
                    EMessagePurpose.OTP,
                    null
                ).blockers
            ).toEqual([EReadinessBlocker.UNSUPPORTED_PROVIDER])
        }
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
            election_channels: {"election-1": [EMessageChannel.SMS]},
        })
        expect(validateEventMessagingConfig(valid, TENANT, accounts, ["election-1"])).toEqual([])
        expect(enabledChannels(valid, EMessagePurpose.OTP, "election-1")).toEqual([
            EMessageChannel.SMS,
        ])
        // In channel order, whatever the configuration's order.
        expect(enabledChannels(valid, EMessagePurpose.OTP, "election-2")).toEqual([
            EMessageChannel.VIBER,
            EMessageChannel.SMS,
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
        })
        expect(validateEventMessagingConfig(invalid, TENANT, accounts, ["election-1"])).toEqual([
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
        expect(parsed.election_channels).toEqual({})
    })

    it("drops the removed fallback order and replies, never writing them again", () => {
        const parsed = parseEventMessagingConfig({
            version: 1,
            channels: [],
            notice_fallback: ["SMS", "EMAIL"],
            reply_text: {en: "Thanks"},
        })
        expect(parsed).toEqual(emptyEventMessagingConfig())
        expect(Object.keys(parsed).sort()).toEqual(["channels", "election_channels", "version"])
    })

    it("sends Messenger notices outside the window with the utility template, never Do not send", () => {
        const parsed = parseEventMessagingConfig({
            version: 1,
            channels: [
                {channel: "MESSENGER", account_id: "page-1", out_of_window: "DISABLED"},
                {channel: "WHATSAPP", account_id: "wa-1", out_of_window: "UTILITY_MESSAGES"},
            ],
        })
        expect(parsed.channels.map((entry) => entry.out_of_window)).toEqual([
            EOutOfWindowPolicy.UTILITY_MESSAGES,
            EOutOfWindowPolicy.DISABLED,
        ])
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

    it("Viber's report credentials are generated, never typed", () => {
        expect(acceptedCredentials(EMessagingProvider.VIBER_INFOBIP)).toEqual([
            ECredentialName.API_KEY,
            ECredentialName.REPORT_USERNAME,
            ECredentialName.REPORT_PASSWORD,
        ])
        expect(typedCredentials(EMessagingProvider.VIBER_INFOBIP)).toEqual([
            ECredentialName.API_KEY,
        ])
        expect(typedCredentials(EMessagingProvider.WHATSAPP_CLOUD_API)).toEqual([
            ECredentialName.ACCESS_TOKEN,
            ECredentialName.APP_SECRET,
        ])
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
                page_name: "STAFFASSOC",
                api_version: "v23.0",
            })
        ).toBe("STAFFASSOC")
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
    const stats = {
        channel: EMessageChannel.SMS,
        voters: 120,
        sent_last_24h: 7,
        queued: 2,
        accepted: 4,
        delivered: 0,
        failed: 1,
        unknown: 3,
        unresolved: 5,
    }

    it("never reports zero delivered when the account has no receipts", () => {
        const summary = deliverySummary(stats, EDeliveryFeedback.UNAVAILABLE)
        expect(summary.delivered).toBeNull()
        expect(summary.accepted).toBe(4)
        expect(summary.failed).toBe(1)
    })

    it("keeps voters, the last 24 hours, and Unresolved apart from Unknown", () => {
        expect(
            deliverySummary({...stats, delivered: 9}, EDeliveryFeedback.PROVIDER_RECEIPTS)
        ).toEqual({
            voters: 120,
            sentLast24h: 7,
            queued: 2,
            accepted: 4,
            delivered: 9,
            failed: 1,
            unknown: 3,
            unresolved: 5,
        })
    })

    it("says nothing is known rather than 0 when the counts are missing", () => {
        const summary = deliverySummary(undefined, EDeliveryFeedback.PROVIDER_RECEIPTS)
        expect(summary.voters).toBeNull()
        expect(summary.sentLast24h).toBeNull()
        expect(summary.delivered).toBe(0)
    })
})

describe("no readiness confirmed by an administrator", () => {
    it("reads a stored ADMIN_CONFIRMED account from the provider's check", () => {
        const confirmed = accountSummary({
            id: "wa-1",
            tenant_id: TENANT,
            channel: EMessageChannel.WHATSAPP,
            provider: EMessagingProvider.WHATSAPP_CLOUD_API,
            name: "WhatsApp",
            sender: {
                provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                business_account_id: "1",
                phone_number_id: "2",
                display_phone_number: "+34600111214",
                api_version: "v23.0",
            },
            provider_approval: EProviderApproval.CONFIRMED,
            readiness: EReadinessPolicy.ADMIN_CONFIRMED,
            status: {connected: false, production_access: false},
            is_default: false,
        })
        const valid = config({
            channels: [
                {
                    ...channelConfig(EMessageChannel.WHATSAPP, "wa-1", [EMessagePurpose.OTP]),
                    templates: [{...binding(null, "tl", "otp_tl"), purpose: EMessagePurpose.OTP}],
                },
            ],
        })
        expect(validateEventMessagingConfig(valid, TENANT, [confirmed], [])).toEqual([
            {
                kind: "PURPOSE_NOT_READY",
                channel: EMessageChannel.WHATSAPP,
                purpose: EMessagePurpose.OTP,
                blockers: [
                    EReadinessBlocker.NOT_CONNECTED,
                    EReadinessBlocker.NEEDS_PRODUCTION_ACCESS,
                    EReadinessBlocker.NEEDS_APPROVED_TEMPLATE,
                ],
            },
            {
                kind: "TEMPLATE_NOT_APPROVED",
                channel: EMessageChannel.WHATSAPP,
                purpose: EMessagePurpose.OTP,
                language: "tl",
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

    it("blocks Messenger notices without a utility template, with the reason", () => {
        const messenger = account(
            "page-1",
            EMessageChannel.MESSENGER,
            EMessagingProvider.MESSENGER_SEND_API,
            readyCheck([])
        )
        const unbound = config({
            channels: [
                {
                    ...channelConfig(EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.NOTICE]),
                    out_of_window: EOutOfWindowPolicy.UTILITY_MESSAGES,
                },
            ],
        })
        expect(validateEventMessagingConfig(unbound, TENANT, [messenger], [])).toEqual([
            {kind: "UTILITY_TEMPLATE_MISSING", channel: EMessageChannel.MESSENGER},
        ])
        // Codes are sent after Get Started, inside the window: no template is needed.
        const codes = config({
            channels: [
                {
                    ...channelConfig(EMessageChannel.MESSENGER, "page-1", [EMessagePurpose.OTP]),
                    out_of_window: EOutOfWindowPolicy.UTILITY_MESSAGES,
                },
            ],
        })
        expect(validateEventMessagingConfig(codes, TENANT, [messenger], [])).toEqual([])
    })

    it("never enables a purpose on an account of a removed provider", () => {
        const old = accountSummary({
            id: "console-1",
            tenant_id: TENANT,
            channel: EMessageChannel.SMS,
            provider: EMessagingProvider.CONSOLE,
            name: "Console",
            sender: {provider: EMessagingProvider.CONSOLE},
            status: readyCheck([]),
            is_default: false,
        })
        expect(
            validateEventMessagingConfig(
                config({
                    channels: [
                        channelConfig(EMessageChannel.SMS, "console-1", [EMessagePurpose.OTP]),
                    ],
                }),
                TENANT,
                [old],
                []
            )
        ).toEqual([
            {
                kind: "PURPOSE_NOT_READY",
                channel: EMessageChannel.SMS,
                purpose: EMessagePurpose.OTP,
                blockers: [EReadinessBlocker.UNSUPPORTED_PROVIDER],
            },
        ])
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
            page_name: "STAFFASSOC",
            page_username: "staffassociation",
            api_version: "v23.0",
        },
        is_default: true,
    }

    it("fills the defaults of an account that was never checked", () => {
        const summary = accountSummary(row)
        expect(summary.provider_approval).toBe(EProviderApproval.PENDING)
        expect(summary.check).toEqual({connected: false, production_access: false})
        expect(summary.capabilities?.conversation_window_hours).toBe(24)
        expect(summary.public_label).toBe("STAFFASSOC")
        expect(summary.messenger_page).toEqual({
            page_id: "1234567890",
            username: "staffassociation",
            name: "STAFFASSOC",
        })
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
            public_label: "STAFFASSOC",
            messenger_page: {
                page_id: "1234567890",
                username: "staffassociation",
                name: "STAFFASSOC",
            },
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
                sender_label: "STAFFASSOC",
                messenger_page: {
                    page_id: "1234567890",
                    username: "staffassociation",
                    name: "STAFFASSOC",
                },
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

    it("an unsupported provider takes no credentials", () => {
        for (const provider of [EMessagingProvider.HTTP_API, EMessagingProvider.CONSOLE]) {
            expect(requiredCredentials(provider)).toEqual([])
            expect(acceptedCredentials(provider)).toEqual([])
        }
    })
})

describe("callback readiness (R4-04)", () => {
    // As messaging_callback_status reports it, merged into the account's check.
    const noCallback = {
        ...readyCheck([]),
        callback_blockers: [EReadinessBlocker.CALLBACK_NOT_CONFIGURED],
        callback_reasons: ["Callback address not configured"],
    }
    const messengerAccount = (status: IAccountCheck): IMessagingAccount => ({
        id: "page-1",
        tenant_id: TENANT,
        channel: EMessageChannel.MESSENGER,
        provider: EMessagingProvider.MESSENGER_SEND_API,
        name: "Staff Association Page",
        sender: {
            provider: EMessagingProvider.MESSENGER_SEND_API,
            page_id: "1",
            api_version: "v23.0",
        },
        status,
        is_default: true,
    })

    it("keeps Messenger from codes without a callback address: they need Get Started", () => {
        expect(
            readinessOf(
                EMessagingProvider.MESSENGER_SEND_API,
                EMessageChannel.MESSENGER,
                EProviderApproval.PENDING,
                noCallback,
                EMessagePurpose.OTP,
                null
            ).blockers
            // The literal, so the enum value itself is checked.
        ).toStrictEqual(["CALLBACK_NOT_CONFIGURED"])
        expect(
            readinessOf(
                EMessagingProvider.MESSENGER_SEND_API,
                EMessageChannel.MESSENGER,
                EProviderApproval.PENDING,
                noCallback,
                EMessagePurpose.NOTICE,
                null
            ).blockers
        ).toEqual([])
        expect(
            readinessOf(
                EMessagingProvider.MESSENGER_SEND_API,
                EMessageChannel.MESSENGER,
                EProviderApproval.PENDING,
                {...noCallback, callback_blockers: [], callback_reasons: []},
                EMessagePurpose.OTP,
                null
            ).blockers
        ).toEqual([])
    })

    it("shows receipts as unavailable without a callback address, never 0 delivered", () => {
        expect(accountDeliveryFeedback(messengerAccount(noCallback))).toEqual({
            feedback: EDeliveryFeedback.UNAVAILABLE,
            reason: "Callback address not configured",
        })
        expect(
            accountDeliveryFeedback(
                messengerAccount({...noCallback, callback_blockers: [], callback_reasons: []})
            )
        ).toEqual({feedback: EDeliveryFeedback.PROVIDER_RECEIPTS})
        expect(accountDeliveryFeedback(undefined)).toEqual({
            feedback: EDeliveryFeedback.UNAVAILABLE,
        })
    })
})

describe("withCallbackStatus (R4-04)", () => {
    const account = {
        id: "page-1",
        status: {connected: true, production_access: true},
    } as IMessagingAccount
    it("puts each account's callback blockers and reasons on its check", () => {
        expect(
            withCallbackStatus(
                [account, {...account, id: "page-2", status: null}],
                [
                    {
                        account_id: "page-1",
                        blockers: ["CALLBACK_NOT_CONFIGURED"],
                        reasons: ["Callback address not configured"],
                    },
                ]
            ).map((entry) => entry.status)
        ).toEqual([
            {
                connected: true,
                production_access: true,
                callback_blockers: ["CALLBACK_NOT_CONFIGURED"],
                callback_reasons: ["Callback address not configured"],
            },
            null,
        ])
    })

    it("leaves the accounts as they are when the status can't be read", () => {
        expect(withCallbackStatus([account], undefined)).toEqual([account])
    })
})

describe("check identity (R5-09)", () => {
    const sender = (phone_number_id: string) => ({
        provider: EMessagingProvider.WHATSAPP_CLOUD_API as const,
        business_account_id: " 1 ",
        phone_number_id,
        display_phone_number: "+63 917 000 0000",
        api_version: "v23.0",
    })

    it("is the sender's identifying ids, trimmed, without names or labels", () => {
        expect(accountIdentity(sender("2"))).toEqual({
            provider: EMessagingProvider.WHATSAPP_CLOUD_API,
            ids: {business_account_id: "1", phone_number_id: "2"},
        })
        expect(
            accountIdentity({
                provider: EMessagingProvider.VIBER_INFOBIP,
                base_url: "https://api.example.test/",
                sender: "Elections",
                templates: {},
            } as unknown as Parameters<typeof accountIdentity>[0]).ids
        ).toEqual({base_url: "https://api.example.test", sender: "Elections"})
    })

    it("counts a check only for the identity it names; an older check names none", () => {
        const check = {...readyCheck([]), details: {identity: accountIdentity(sender("2"))}}
        expect(checkIsFor(check, accountIdentity(sender("2")))).toBe(true)
        expect(checkIsFor(check, accountIdentity(sender("3")))).toBe(false)
        expect(checkIsFor(readyCheck([]), accountIdentity(sender("3")))).toBe(true)
    })

    it("makes an account whose check was of another number ready for nothing", () => {
        const check = {...readyCheck([]), details: {identity: accountIdentity(sender("2"))}}
        // Only the sender and the status matter here.
        const moved = {sender: sender("3"), status: check} as unknown as IMessagingAccount
        expect(checkedForAnotherIdentity(moved)).toBe(true)
        expect(checkedForAnotherIdentity({...moved, sender: sender("2")})).toBe(false)
        const readiness = purposeReadiness(
            EMessagingProvider.WHATSAPP_CLOUD_API,
            providerCapabilities(EMessagingProvider.WHATSAPP_CLOUD_API, EMessageChannel.WHATSAPP),
            EProviderApproval.CONFIRMED,
            check,
            EMessagePurpose.NOTICE,
            null,
            accountIdentity(sender("3"))
        )
        expect(readiness.blockers).toEqual([EReadinessBlocker.UNCHECKED])
    })
})
