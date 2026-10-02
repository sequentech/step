// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EMessageAttemptState,
    EMessageChannel,
    EMessagePurpose,
    EOutOfWindowPolicy,
    IEventChannelConfig,
    IEventMessagingConfig,
    IMessagingConfigError,
    MESSAGE_ATTEMPT_STATES,
    MESSAGE_CHANNELS,
} from "@/types/messaging"
import {IStateCount} from "@/services/messaging"

const updateChannel = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    update: (current: IEventChannelConfig) => IEventChannelConfig
): IEventMessagingConfig => ({
    ...config,
    channels: config.channels.map((current) =>
        current.channel === channel ? update(current) : current
    ),
})

const withoutFallback = (config: IEventMessagingConfig, channel: EMessageChannel) => ({
    ...config,
    notice_fallback: config.notice_fallback.filter((entry) => entry !== channel),
})

const withoutElectionChannel = (
    config: IEventMessagingConfig,
    channel: EMessageChannel
): IEventMessagingConfig => ({
    ...config,
    election_channels: Object.fromEntries(
        Object.entries(config.election_channels).map(([electionId, channels]) => [
            electionId,
            channels.filter((entry) => entry !== channel),
        ])
    ),
})

/** Selects the account of a channel; `null` stops using the channel. */
export const setChannelAccount = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    accountId: string | null
): IEventMessagingConfig => {
    const current = config.channels.find((entry) => entry.channel === channel)
    if (!accountId) {
        if (!current) return config
        return withoutElectionChannel(
            withoutFallback(
                {...config, channels: config.channels.filter((entry) => entry !== current)},
                channel
            ),
            channel
        )
    }
    if (!current) {
        return {
            ...config,
            channels: [
                ...config.channels,
                {
                    channel,
                    account_id: accountId,
                    purposes: [],
                    templates: [],
                    out_of_window: EOutOfWindowPolicy.DISABLED,
                },
            ],
        }
    }
    if (current.account_id === accountId) return config
    // Readiness and template approvals belong to the account: start over.
    return withoutElectionChannel(
        withoutFallback(
            updateChannel(config, channel, (entry) => ({
                ...entry,
                account_id: accountId,
                purposes: [],
                templates: [],
                out_of_window: EOutOfWindowPolicy.DISABLED,
            })),
            channel
        ),
        channel
    )
}

export const togglePurpose = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    purpose: EMessagePurpose,
    enabled: boolean
): IEventMessagingConfig => {
    const current = config.channels.find((entry) => entry.channel === channel)
    if (!current) return config
    if (enabled) {
        if (current.purposes.includes(purpose)) return config
        const next = updateChannel(config, channel, (entry) => ({
            ...entry,
            purposes: [...entry.purposes, purpose],
        }))
        return purpose === EMessagePurpose.NOTICE && !next.notice_fallback.includes(channel)
            ? {...next, notice_fallback: [...next.notice_fallback, channel]}
            : next
    }
    let next = updateChannel(config, channel, (entry) => ({
        ...entry,
        purposes: entry.purposes.filter((entry) => entry !== purpose),
        templates: entry.templates.filter((binding) => binding.purpose !== purpose),
    }))
    if (purpose === EMessagePurpose.NOTICE) {
        next = withoutFallback(next, channel)
    }
    const remaining = next.channels.find((entry) => entry.channel === channel)
    return remaining?.purposes.length ? next : withoutElectionChannel(next, channel)
}

/** Binds a purpose and language to a provider template; blank removes the binding. */
export const setTemplateBinding = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    purpose: EMessagePurpose,
    language: string,
    providerTemplate: string
): IEventMessagingConfig => {
    const value = providerTemplate.trim()
    return updateChannel(config, channel, (entry) => {
        const exists = entry.templates.some(
            (binding) => binding.purpose === purpose && binding.language === language
        )
        const templates = value
            ? exists
                ? entry.templates.map((binding) =>
                      binding.purpose === purpose && binding.language === language
                          ? {...binding, provider_template: value}
                          : binding
                  )
                : [...entry.templates, {purpose, language, provider_template: value}]
            : entry.templates.filter(
                  (binding) => !(binding.purpose === purpose && binding.language === language)
              )
        return {...entry, templates}
    })
}

export const templateBinding = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    purpose: EMessagePurpose,
    language: string
): string =>
    config.channels
        .find((entry) => entry.channel === channel)
        ?.templates.find((binding) => binding.purpose === purpose && binding.language === language)
        ?.provider_template ?? ""

export const moveFallback = (
    config: IEventMessagingConfig,
    index: number,
    delta: -1 | 1
): IEventMessagingConfig => {
    const target = index + delta
    if (index < 0 || target < 0 || target >= config.notice_fallback.length) return config
    const notice_fallback = [...config.notice_fallback]
    ;[notice_fallback[index], notice_fallback[target]] = [
        notice_fallback[target],
        notice_fallback[index],
    ]
    return {...config, notice_fallback}
}

/** Channels with at least one purpose enabled, in configuration order. */
export const activeChannels = (config: IEventMessagingConfig): EMessageChannel[] =>
    config.channels.filter((entry) => entry.purposes.length).map((entry) => entry.channel)

/** Channels an election offers: its restriction, or every active channel. */
export const electionChannelsOf = (
    config: IEventMessagingConfig,
    electionId: string
): EMessageChannel[] => {
    const active = activeChannels(config)
    const restriction = config.election_channels[electionId]
    return restriction ? active.filter((channel) => restriction.includes(channel)) : active
}

export const toggleElectionChannel = (
    config: IEventMessagingConfig,
    electionId: string,
    channel: EMessageChannel
): IEventMessagingConfig => {
    const active = activeChannels(config)
    const current = electionChannelsOf(config, electionId)
    const next = current.includes(channel)
        ? current.filter((entry) => entry !== channel)
        : active.filter((entry) => entry === channel || current.includes(entry))
    const election_channels = {...config.election_channels}
    if (next.length === active.length) {
        delete election_channels[electionId]
    } else {
        election_channels[electionId] = next
    }
    return {...config, election_channels}
}

export const setReplyText = (
    config: IEventMessagingConfig,
    language: string,
    text: string
): IEventMessagingConfig => {
    const reply_text = {...config.reply_text}
    if (text.trim()) {
        reply_text[language] = text
    } else {
        delete reply_text[language]
    }
    return {...config, reply_text}
}

export enum EMessagingErrorArea {
    GENERAL = "GENERAL",
    CHANNEL = "CHANNEL",
    PURPOSE = "PURPOSE",
    TEMPLATE = "TEMPLATE",
    FALLBACK = "FALLBACK",
    ELECTION = "ELECTION",
}

export type IMessagingErrorLocation =
    | {area: EMessagingErrorArea.GENERAL}
    | {area: EMessagingErrorArea.CHANNEL; channel: EMessageChannel}
    | {area: EMessagingErrorArea.PURPOSE; channel: EMessageChannel; purpose: EMessagePurpose}
    | {
          area: EMessagingErrorArea.TEMPLATE
          channel: EMessageChannel
          purpose: EMessagePurpose
          language: string
      }
    | {area: EMessagingErrorArea.FALLBACK}
    | {area: EMessagingErrorArea.ELECTION; electionId: string}

/** The control a configuration error belongs next to. */
export const errorLocation = (
    error: IMessagingConfigError,
    config: IEventMessagingConfig
): IMessagingErrorLocation => {
    switch (error.kind) {
        case "UNSUPPORTED_VERSION":
            return {area: EMessagingErrorArea.GENERAL}
        case "ACCOUNT_OF_ANOTHER_TENANT": {
            const owner = config.channels.find((entry) => entry.account_id === error.account_id)
            return owner
                ? {area: EMessagingErrorArea.CHANNEL, channel: owner.channel}
                : {area: EMessagingErrorArea.GENERAL}
        }
        case "DUPLICATE_CHANNEL":
        case "UNKNOWN_ACCOUNT":
        case "ACCOUNT_CHANNEL_MISMATCH":
        case "OUT_OF_WINDOW_NOT_SUPPORTED":
            return {area: EMessagingErrorArea.CHANNEL, channel: error.channel}
        case "PURPOSE_NOT_READY":
            return {
                area: EMessagingErrorArea.PURPOSE,
                channel: error.channel,
                purpose: error.purpose,
            }
        case "TEMPLATE_NOT_APPROVED":
            return {
                area: EMessagingErrorArea.TEMPLATE,
                channel: error.channel,
                purpose: error.purpose,
                language: error.language,
            }
        case "FALLBACK_CHANNEL_NOT_ENABLED":
        case "DUPLICATE_FALLBACK_CHANNEL":
            return {area: EMessagingErrorArea.FALLBACK}
        case "ELECTION_CHANNEL_NOT_ENABLED":
        case "UNKNOWN_ELECTION":
            return {area: EMessagingErrorArea.ELECTION, electionId: error.election_id}
    }
}

const sameLocation = (a: IMessagingErrorLocation, b: IMessagingErrorLocation) =>
    JSON.stringify(a) === JSON.stringify(b)

export const errorsAt = (
    errors: IMessagingConfigError[],
    config: IEventMessagingConfig,
    location: IMessagingErrorLocation
): IMessagingConfigError[] =>
    errors.filter((error) => sameLocation(errorLocation(error, config), location))

/** Reads the `errors` jsonb of update_event_messaging_config. */
export const parseMessagingConfigErrors = (raw: unknown): IMessagingConfigError[] => {
    let value: unknown = raw
    if (typeof raw === "string") {
        try {
            value = JSON.parse(raw)
        } catch {
            return []
        }
    }
    if (!Array.isArray(value)) return []
    return value.filter(
        (entry): entry is IMessagingConfigError =>
            typeof entry === "object" &&
            entry !== null &&
            typeof (entry as {kind?: unknown}).kind === "string"
    )
}

export interface IAggregateCount {
    aggregate?: {count?: number | null} | null
}

export type IDeliveryStatsResponse = Record<string, IAggregateCount | null | undefined>

/** Splits the `{CHANNEL}_{STATE}` aggregates of GetMessageDeliveryStats per channel. */
export const parseDeliveryStats = (
    data: IDeliveryStatsResponse | null | undefined
): Record<EMessageChannel, IStateCount[]> =>
    Object.fromEntries(
        MESSAGE_CHANNELS.map((channel) => [
            channel,
            MESSAGE_ATTEMPT_STATES.flatMap((state: EMessageAttemptState) => {
                const count = data?.[`${channel}_${state}`]?.aggregate?.count
                return typeof count === "number" ? [{state, count}] : []
            }),
        ])
    ) as Record<EMessageChannel, IStateCount[]>
