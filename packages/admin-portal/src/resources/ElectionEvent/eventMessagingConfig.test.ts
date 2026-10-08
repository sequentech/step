// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EMessageAttemptState,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    EReadinessBlocker,
    EReadinessPolicy,
    IAccountSummary,
    IEventMessagingConfig,
    IMessagingConfigError,
} from "@/types/messaging"
import {emptyEventMessagingConfig, providerCapabilities} from "@/services/messaging"
import {
    EBindingApproval,
    EMessagingErrorArea,
    addTemplateBinding,
    bindingApproval,
    incompleteBindings,
    preparedConfig,
    removeTemplateBinding,
    setOutOfWindow,
    templateChannels,
    updateTemplateBinding,
    electionChannelsOf,
    errorLocation,
    errorsAt,
    moveFallback,
    parseDeliveryStats,
    parseMessagingConfigErrors,
    setChannelAccount,
    setReplyText,
    togglePurpose,
    toggleElectionChannel,
} from "./eventMessagingConfig"

const {SMS, VIBER, WHATSAPP, EMAIL} = EMessageChannel
const {OTP, NOTICE} = EMessagePurpose

const withChannels = (
    channels: Array<[EMessageChannel, EMessagePurpose[]]>,
    overrides: Partial<IEventMessagingConfig> = {}
): IEventMessagingConfig => ({
    ...emptyEventMessagingConfig(),
    channels: channels.map(([channel, purposes]) => ({
        channel,
        account_id: `${channel.toLowerCase()}-1`,
        purposes,
        templates: [],
        out_of_window: EOutOfWindowPolicy.DISABLED,
    })),
    ...overrides,
})

describe("setChannelAccount", () => {
    it("adds a channel with no purposes enabled", () => {
        const config = setChannelAccount(emptyEventMessagingConfig(), SMS, "sms-1")
        expect(config.channels).toEqual([
            {
                channel: SMS,
                account_id: "sms-1",
                purposes: [],
                templates: [],
                out_of_window: EOutOfWindowPolicy.DISABLED,
            },
        ])
    })

    it("clears purposes and templates when the account changes", () => {
        const before = withChannels([[VIBER, [OTP, NOTICE]]], {notice_fallback: [VIBER]})
        before.channels[0].templates = [{purpose: OTP, language: "en", provider_template: "t"}]
        const after = setChannelAccount(before, VIBER, "viber-2")
        expect(after.channels[0]).toMatchObject({
            account_id: "viber-2",
            purposes: [],
            templates: [],
        })
        expect(after.notice_fallback).toEqual([])
    })

    it("keeps the channel unchanged when the same account is selected", () => {
        const before = withChannels([[SMS, [OTP]]])
        expect(setChannelAccount(before, SMS, "sms-1")).toEqual(before)
    })

    it("removes the channel everywhere when the account is cleared", () => {
        const before = withChannels(
            [
                [SMS, [NOTICE]],
                [EMAIL, [NOTICE]],
            ],
            {
                notice_fallback: [SMS, EMAIL],
                election_channels: {"election-1": [SMS, EMAIL]},
            }
        )
        const after = setChannelAccount(before, SMS, null)
        expect(after.channels.map(({channel}) => channel)).toEqual([EMAIL])
        expect(after.notice_fallback).toEqual([EMAIL])
        expect(after.election_channels).toEqual({"election-1": [EMAIL]})
    })
})

describe("togglePurpose", () => {
    it("enables a notice and appends the channel to the fallback order", () => {
        const after = togglePurpose(withChannels([[SMS, []]]), SMS, NOTICE, true)
        expect(after.channels[0].purposes).toEqual([NOTICE])
        expect(after.notice_fallback).toEqual([SMS])
    })

    it("enabling OTPs leaves the fallback order alone", () => {
        const after = togglePurpose(withChannels([[SMS, []]]), SMS, OTP, true)
        expect(after.channels[0].purposes).toEqual([OTP])
        expect(after.notice_fallback).toEqual([])
    })

    it("disabling a purpose drops its templates and fallback entry", () => {
        const before = withChannels([[VIBER, [OTP, NOTICE]]], {notice_fallback: [VIBER]})
        before.channels[0].templates = [
            {purpose: OTP, language: "en", provider_template: "otp"},
            {purpose: NOTICE, language: "en", provider_template: "notice"},
        ]
        const after = togglePurpose(before, VIBER, NOTICE, false)
        expect(after.channels[0].purposes).toEqual([OTP])
        expect(after.channels[0].templates).toEqual([
            {purpose: OTP, language: "en", provider_template: "otp"},
        ])
        expect(after.notice_fallback).toEqual([])
    })

    it("ignores a channel without an account", () => {
        const before = emptyEventMessagingConfig()
        expect(togglePurpose(before, SMS, OTP, true)).toEqual(before)
    })

    it("drops the election restriction entries of a channel with no purposes left", () => {
        const before = withChannels(
            [
                [SMS, [OTP]],
                [EMAIL, [OTP]],
            ],
            {
                election_channels: {"election-1": [SMS]},
            }
        )
        const after = togglePurpose(before, SMS, OTP, false)
        expect(after.election_channels).toEqual({"election-1": []})
    })
})

describe("template bindings", () => {
    it("adds a row for a purpose and language, edits it and removes it", () => {
        let config = withChannels([[WHATSAPP, [OTP, NOTICE]]])
        config = addTemplateBinding(config, WHATSAPP, OTP, "en")
        config = addTemplateBinding(config, WHATSAPP, NOTICE, "tl")
        expect(config.channels[0].templates).toEqual([
            {
                purpose: OTP,
                key: null,
                language: "en",
                provider_template: "",
                provider_language: null,
            },
            {
                purpose: NOTICE,
                key: null,
                language: "tl",
                provider_template: "",
                provider_language: null,
            },
        ])
        config = updateTemplateBinding(config, WHATSAPP, 1, {
            key: "reminder",
            provider_template: "reminder_tl",
            provider_language: "fil",
        })
        expect(config.channels[0].templates[1]).toEqual({
            purpose: NOTICE,
            key: "reminder",
            language: "tl",
            provider_template: "reminder_tl",
            provider_language: "fil",
        })
        config = removeTemplateBinding(config, WHATSAPP, 0)
        expect(config.channels[0].templates.map(({purpose}) => purpose)).toEqual([NOTICE])
    })

    it("leaves a channel that is not configured alone", () => {
        const config = withChannels([[SMS, [OTP]]])
        expect(addTemplateBinding(config, WHATSAPP, OTP, "en")).toEqual(config)
    })

    it("saves trimmed bindings, without empty rows or empty optional values", () => {
        const config = withChannels([[WHATSAPP, [OTP, NOTICE]]])
        config.channels[0].templates = [
            {
                purpose: OTP,
                key: " ",
                language: " en ",
                provider_template: " otp_en ",
                provider_language: " en_US ",
            },
            {purpose: NOTICE, key: null, language: "en", provider_template: " "},
            {purpose: NOTICE, key: " reminder ", language: "tl", provider_template: "reminder_tl"},
        ]
        expect(preparedConfig(config).channels[0].templates).toEqual([
            {
                purpose: OTP,
                key: null,
                language: "en",
                provider_template: "otp_en",
                provider_language: "en_US",
            },
            {
                purpose: NOTICE,
                key: "reminder",
                language: "tl",
                provider_template: "reminder_tl",
                provider_language: null,
            },
        ])
    })

    it("names the rows that miss the language or the provider template", () => {
        const config = withChannels([
            [WHATSAPP, [NOTICE]],
            [VIBER, [NOTICE]],
        ])
        config.channels[0].templates = [
            {purpose: NOTICE, language: "en", provider_template: "ok"},
            {purpose: NOTICE, key: "reminder", language: "en", provider_template: ""},
            {purpose: NOTICE, language: "en", provider_template: ""},
        ]
        config.channels[1].templates = [{purpose: NOTICE, language: " ", provider_template: "x"}]
        expect(incompleteBindings(config)).toEqual([
            {channel: WHATSAPP, index: 1},
            {channel: VIBER, index: 0},
        ])
    })
})

describe("setOutOfWindow", () => {
    it("chooses how a channel sends outside its conversation window", () => {
        const config = withChannels([[EMessageChannel.MESSENGER, [NOTICE]]])
        const after = setOutOfWindow(
            config,
            EMessageChannel.MESSENGER,
            EOutOfWindowPolicy.UTILITY_MESSAGES
        )
        expect(after.channels[0].out_of_window).toBe(EOutOfWindowPolicy.UTILITY_MESSAGES)
        expect(config.channels[0].out_of_window).toBe(EOutOfWindowPolicy.DISABLED)
    })
})

describe("bindingApproval", () => {
    const whatsapp: IAccountSummary = {
        id: "wa-1",
        tenant_id: "tenant-1",
        channel: WHATSAPP,
        provider: EMessagingProvider.WHATSAPP_CLOUD_API,
        capabilities: providerCapabilities(EMessagingProvider.WHATSAPP_CLOUD_API, WHATSAPP),
        provider_approval: EProviderApproval.CONFIRMED,
        readiness: EReadinessPolicy.PROVIDER_CHECK,
        check: {connected: true, production_access: true, approved_templates: {NOTICE: ["en_US"]}},
    }
    const notice = (language: string, providerLanguage: string | null = null) => ({
        purpose: NOTICE,
        language,
        provider_template: "t",
        provider_language: providerLanguage,
    })

    it("reads the provider's check, under the voter's or the provider's language code", () => {
        expect(bindingApproval(whatsapp, notice("en", "en_US"))).toBe(EBindingApproval.APPROVED)
        expect(bindingApproval(whatsapp, notice("en_US"))).toBe(EBindingApproval.APPROVED)
        expect(bindingApproval(whatsapp, notice("en"))).toBe(EBindingApproval.NOT_APPROVED)
    })

    it("shows the administrator's confirmation instead of a check result", () => {
        expect(
            bindingApproval(
                {...whatsapp, readiness: EReadinessPolicy.ADMIN_CONFIRMED},
                notice("tl")
            )
        ).toBe(EBindingApproval.ADMIN_CONFIRMED)
    })

    it("does not ask for an approval the purpose does not need", () => {
        const messenger: IAccountSummary = {
            ...whatsapp,
            channel: EMessageChannel.MESSENGER,
            provider: EMessagingProvider.MESSENGER_SEND_API,
            capabilities: providerCapabilities(
                EMessagingProvider.MESSENGER_SEND_API,
                EMessageChannel.MESSENGER
            ),
            check: {connected: true, production_access: true},
        }
        expect(bindingApproval(messenger, notice("en"))).toBe(EBindingApproval.NOT_CHECKED)
        expect(bindingApproval(undefined, notice("en"))).toBe(EBindingApproval.NOT_CHECKED)
    })
})

describe("templateChannels", () => {
    const summary = (
        id: string,
        channel: EMessageChannel,
        provider: EMessagingProvider
    ): IAccountSummary => ({
        id,
        tenant_id: "tenant-1",
        channel,
        provider,
        capabilities: providerCapabilities(provider, channel),
        provider_approval: EProviderApproval.PENDING,
        readiness: EReadinessPolicy.PROVIDER_CHECK,
        check: {connected: true, production_access: true},
    })

    it("lists the channels whose account uses approved templates", () => {
        const config = withChannels([
            [SMS, [OTP]],
            [WHATSAPP, []],
            [EMessageChannel.MESSENGER, [NOTICE]],
            [EMAIL, [NOTICE]],
        ])
        config.channels[3].templates = [{purpose: NOTICE, language: "en", provider_template: "x"}]
        expect(
            templateChannels(config, [
                summary("sms-1", SMS, EMessagingProvider.AWS_SNS),
                summary("whatsapp-1", WHATSAPP, EMessagingProvider.WHATSAPP_CLOUD_API),
                summary(
                    "messenger-1",
                    EMessageChannel.MESSENGER,
                    EMessagingProvider.MESSENGER_SEND_API
                ),
                summary("email-1", EMAIL, EMessagingProvider.AWS_SES),
            ])
        ).toEqual([WHATSAPP, EMessageChannel.MESSENGER, EMAIL])
    })
})

describe("moveFallback", () => {
    it("swaps neighbours and ignores moves past either end", () => {
        const config = withChannels([], {notice_fallback: [SMS, VIBER, EMAIL]})
        expect(moveFallback(config, 1, -1).notice_fallback).toEqual([VIBER, SMS, EMAIL])
        expect(moveFallback(config, 1, 1).notice_fallback).toEqual([SMS, EMAIL, VIBER])
        expect(moveFallback(config, 0, -1).notice_fallback).toEqual([SMS, VIBER, EMAIL])
        expect(moveFallback(config, 2, 1).notice_fallback).toEqual([SMS, VIBER, EMAIL])
    })
})

describe("election channels", () => {
    const config = withChannels([
        [SMS, [OTP]],
        [EMAIL, [NOTICE]],
        [VIBER, []],
    ])

    it("an election without an entry offers every enabled channel", () => {
        expect(electionChannelsOf(config, "election-1")).toEqual([SMS, EMAIL])
    })

    it("unticking writes the explicit list and ticking everything again inherits", () => {
        const restricted = toggleElectionChannel(config, "election-1", SMS)
        expect(restricted.election_channels).toEqual({"election-1": [EMAIL]})
        expect(electionChannelsOf(restricted, "election-1")).toEqual([EMAIL])
        const inherited = toggleElectionChannel(restricted, "election-1", SMS)
        expect(inherited.election_channels).toEqual({})
    })
})

describe("setReplyText", () => {
    it("stores text per language and removes empty text", () => {
        let config = setReplyText(emptyEventMessagingConfig(), "en", "Not read")
        expect(config.reply_text).toEqual({en: "Not read"})
        config = setReplyText(config, "en", "")
        expect(config.reply_text).toEqual({})
    })
})

describe("errors", () => {
    const errors: IMessagingConfigError[] = [
        {kind: "UNSUPPORTED_VERSION", version: 2},
        {kind: "DUPLICATE_CHANNEL", channel: SMS},
        {kind: "UNKNOWN_ACCOUNT", channel: SMS, account_id: "x"},
        {kind: "ACCOUNT_OF_ANOTHER_TENANT", account_id: "viber-1"},
        {kind: "ACCOUNT_CHANNEL_MISMATCH", channel: VIBER, account_id: "viber-1"},
        {
            kind: "PURPOSE_NOT_READY",
            channel: WHATSAPP,
            purpose: OTP,
            blockers: [EReadinessBlocker.NEEDS_PROVIDER_APPROVAL],
        },
        {kind: "TEMPLATE_NOT_APPROVED", channel: WHATSAPP, purpose: OTP, language: "tl"},
        {kind: "OUT_OF_WINDOW_NOT_SUPPORTED", channel: SMS},
        {kind: "FALLBACK_CHANNEL_NOT_ENABLED", channel: EMAIL},
        {kind: "DUPLICATE_FALLBACK_CHANNEL", channel: EMAIL},
        {kind: "ELECTION_CHANNEL_NOT_ENABLED", election_id: "e-1", channel: VIBER},
        {kind: "UNKNOWN_ELECTION", election_id: "e-2"},
    ]
    const config = withChannels([[VIBER, []]])

    it("places each error next to its control", () => {
        expect(errors.map((error) => errorLocation(error, config))).toEqual([
            {area: EMessagingErrorArea.GENERAL},
            {area: EMessagingErrorArea.CHANNEL, channel: SMS},
            {area: EMessagingErrorArea.CHANNEL, channel: SMS},
            {area: EMessagingErrorArea.CHANNEL, channel: VIBER},
            {area: EMessagingErrorArea.CHANNEL, channel: VIBER},
            {area: EMessagingErrorArea.PURPOSE, channel: WHATSAPP, purpose: OTP},
            {area: EMessagingErrorArea.TEMPLATE, channel: WHATSAPP, purpose: OTP, language: "tl"},
            {area: EMessagingErrorArea.CHANNEL, channel: SMS},
            {area: EMessagingErrorArea.FALLBACK},
            {area: EMessagingErrorArea.FALLBACK},
            {area: EMessagingErrorArea.ELECTION, electionId: "e-1"},
            {area: EMessagingErrorArea.ELECTION, electionId: "e-2"},
        ])
    })

    it("an account of another tenant not used by any channel is general", () => {
        expect(
            errorLocation(
                {kind: "ACCOUNT_OF_ANOTHER_TENANT", account_id: "elsewhere"},
                emptyEventMessagingConfig()
            )
        ).toEqual({area: EMessagingErrorArea.GENERAL})
    })

    it("filters the errors of one location", () => {
        expect(
            errorsAt(errors, config, {area: EMessagingErrorArea.CHANNEL, channel: SMS}).map(
                ({kind}) => kind
            )
        ).toEqual(["DUPLICATE_CHANNEL", "UNKNOWN_ACCOUNT", "OUT_OF_WINDOW_NOT_SUPPORTED"])
        expect(errorsAt(errors, config, {area: EMessagingErrorArea.FALLBACK})).toHaveLength(2)
    })

    it("parses the action's errors, keeping only tagged objects", () => {
        expect(parseMessagingConfigErrors([{kind: "UNKNOWN_ELECTION", election_id: "a"}])).toEqual([
            {kind: "UNKNOWN_ELECTION", election_id: "a"},
        ])
        expect(parseMessagingConfigErrors(JSON.stringify([{kind: "DUPLICATE_CHANNEL"}]))).toEqual([
            {kind: "DUPLICATE_CHANNEL"},
        ])
        expect(parseMessagingConfigErrors([null, 3, {nope: 1}])).toEqual([])
        expect(parseMessagingConfigErrors(undefined)).toEqual([])
        expect(parseMessagingConfigErrors("{oops")).toEqual([])
    })
})

describe("parseDeliveryStats", () => {
    it("reads the aliased aggregates per channel", () => {
        const stats = parseDeliveryStats({
            SMS_ACCEPTED: {aggregate: {count: 4}},
            SMS_FAILED: {aggregate: {count: 1}},
            VIBER_DELIVERED: {aggregate: {count: 7}},
            VIBER_UNKNOWN: {aggregate: null},
            unrelated: {aggregate: {count: 9}},
        })
        expect(stats[SMS]).toEqual([
            {state: EMessageAttemptState.ACCEPTED, count: 4},
            {state: EMessageAttemptState.FAILED, count: 1},
        ])
        expect(stats[VIBER]).toEqual([{state: EMessageAttemptState.DELIVERED, count: 7}])
        expect(stats[EMAIL]).toEqual([])
    })

    it("handles a missing response", () => {
        expect(parseDeliveryStats(undefined)[WHATSAPP]).toEqual([])
    })
})
