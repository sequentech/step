// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EMessageChannel,
    EMessagePurpose,
    IAccountSummary,
    IChannelMessageStats,
    IEventChannelConfig,
    IEventMessagingConfig,
    IMessagingConfigError,
    ITemplateBinding,
    MESSAGE_CHANNELS,
} from "@/types/messaging"
import {bindingProviderLanguage, outOfWindowPolicy, supportsOutOfWindow} from "@/services/messaging"

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
            {...config, channels: config.channels.filter((entry) => entry !== current)},
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
                    out_of_window: outOfWindowPolicy(channel),
                },
            ],
        }
    }
    if (current.account_id === accountId) return config
    // Readiness and template approvals belong to the account: start over.
    return withoutElectionChannel(
        updateChannel(config, channel, (entry) => ({
            ...entry,
            account_id: accountId,
            purposes: [],
            templates: [],
            out_of_window: outOfWindowPolicy(channel),
        })),
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
        return updateChannel(config, channel, (entry) => ({
            ...entry,
            purposes: [...entry.purposes, purpose],
        }))
    }
    const next = updateChannel(config, channel, (entry) => ({
        ...entry,
        purposes: entry.purposes.filter((entry) => entry !== purpose),
        templates: entry.templates.filter((binding) => binding.purpose !== purpose),
    }))
    const remaining = next.channels.find((entry) => entry.channel === channel)
    return remaining?.purposes.length ? next : withoutElectionChannel(next, channel)
}

/** Adds an empty binding row for a purpose and language of a channel. */
export const addTemplateBinding = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    purpose: EMessagePurpose,
    language: string
): IEventMessagingConfig =>
    updateChannel(config, channel, (entry) => ({
        ...entry,
        templates: [
            ...entry.templates,
            {purpose, key: null, language, provider_template: "", provider_language: null},
        ],
    }))

export const updateTemplateBinding = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    index: number,
    patch: Partial<ITemplateBinding>
): IEventMessagingConfig =>
    updateChannel(config, channel, (entry) => ({
        ...entry,
        templates: entry.templates.map((binding, position) =>
            position === index ? {...binding, ...patch} : binding
        ),
    }))

export const removeTemplateBinding = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    index: number
): IEventMessagingConfig =>
    updateChannel(config, channel, (entry) => ({
        ...entry,
        templates: entry.templates.filter((_binding, position) => position !== index),
    }))

const blank = (value: string | null | undefined): boolean => !value?.trim()

/** A row nothing was entered in besides the purpose and language it was added with. */
const emptyBinding = (binding: ITemplateBinding): boolean =>
    blank(binding.provider_template) && blank(binding.key) && blank(binding.provider_language)

export interface IBindingPosition {
    channel: EMessageChannel
    index: number
}

/** Rows that were started but miss the language or the provider's template. */
export const incompleteBindings = (config: IEventMessagingConfig): IBindingPosition[] =>
    config.channels.flatMap((entry) =>
        entry.templates.flatMap((binding, index) =>
            !emptyBinding(binding) && (blank(binding.language) || blank(binding.provider_template))
                ? [{channel: entry.channel, index}]
                : []
        )
    )

/** The configuration to save: bindings trimmed, empty rows and empty optional values left out. */
export const preparedConfig = (config: IEventMessagingConfig): IEventMessagingConfig => ({
    ...config,
    channels: config.channels.map((entry) => ({
        ...entry,
        templates: entry.templates
            .filter((binding) => !emptyBinding(binding))
            .map((binding) => ({
                purpose: binding.purpose,
                key: binding.key?.trim() || null,
                language: binding.language.trim(),
                provider_template: binding.provider_template.trim(),
                provider_language: binding.provider_language?.trim() || null,
            })),
    })),
})

export enum EBindingApproval {
    APPROVED = "APPROVED",
    NOT_APPROVED = "NOT_APPROVED",
    /** The purpose is sent as free text on this account, so no approval is checked. */
    NOT_CHECKED = "NOT_CHECKED",
}

/** What is known about the provider's approval of a binding's template. */
export const bindingApproval = (
    account: IAccountSummary | undefined,
    binding: ITemplateBinding
): EBindingApproval => {
    if (!account) {
        return EBindingApproval.NOT_CHECKED
    }
    const languages = account.check.approved_templates?.[binding.purpose] ?? []
    if (
        languages.includes(bindingProviderLanguage(binding)) ||
        languages.includes(binding.language)
    ) {
        return EBindingApproval.APPROVED
    }
    return account.capabilities?.template_required_for.includes(binding.purpose)
        ? EBindingApproval.NOT_APPROVED
        : EBindingApproval.NOT_CHECKED
}

/**
 * Channels whose bindings are edited: the account needs approved templates
 * for a purpose, can send them outside its conversation window, or the channel
 * already has bindings.
 */
export const templateChannels = (
    config: IEventMessagingConfig,
    accounts: IAccountSummary[]
): EMessageChannel[] =>
    config.channels
        .filter((entry) => {
            const capabilities = accounts.find(
                (account) => account.id === entry.account_id
            )?.capabilities
            return (
                entry.templates.length > 0 ||
                (capabilities?.template_required_for.length ?? 0) > 0 ||
                supportsOutOfWindow(capabilities ?? null)
            )
        })
        .map((entry) => entry.channel)

/** Channels with at least one purpose enabled, in channel order. */
export const activeChannels = (config: IEventMessagingConfig): EMessageChannel[] => {
    const active = config.channels
        .filter((entry) => entry.purposes.length)
        .map((entry) => entry.channel)
    return MESSAGE_CHANNELS.filter((channel) => active.includes(channel))
}

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

export enum EMessagingErrorArea {
    GENERAL = "GENERAL",
    CHANNEL = "CHANNEL",
    PURPOSE = "PURPOSE",
    TEMPLATE = "TEMPLATE",
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
        case "UTILITY_TEMPLATE_MISSING":
            return {
                area: EMessagingErrorArea.PURPOSE,
                channel: error.channel,
                purpose: EMessagePurpose.NOTICE,
            }
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

/**
 * The stats of each channel by channel, from messaging_event_stats. The row
 * without a channel (voters with no contact channel) is left out: it is never
 * counted under a channel, and `noChannelStats` reads it.
 */
export const statsByChannel = (
    channels: IChannelMessageStats[] | null | undefined
): Partial<Record<EMessageChannel, IChannelMessageStats>> =>
    Object.fromEntries(
        (channels ?? []).filter((entry) => !!entry.channel).map((entry) => [entry.channel, entry])
    )

/** The messages of voters with no contact channel, when there are any. */
export const noChannelStats = (
    channels: IChannelMessageStats[] | null | undefined
): IChannelMessageStats | undefined => (channels ?? []).find((entry) => !entry.channel)
