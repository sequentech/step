// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EMessageChannel,
    EMessagePurpose,
    IEventMessagingConfig,
    IInstantMessageConfig,
    IMessagingAccount,
} from "@/types/messaging"
import {enabledChannels} from "@/services/messaging"
import {IEmail, IMethods, ISmsConfig, MESSAGE_TEMPLATE_METHODS} from "@/types/templates"
import {templateOffersMethod} from "@/services/templateMethods"

export interface ISendContent {
    email?: IEmail
    sms?: ISmsConfig
    whatsapp?: IInstantMessageConfig
    viber?: IInstantMessageConfig
    messenger?: IInstantMessageConfig
}

export interface ISendTemplateState<Audience extends string, VoterId> {
    audienceSelection: Audience
    voterIds?: VoterId[]
    scheduleNow: boolean
    /** RFC 3339 instant. */
    scheduleDate?: string
    /** Wall time and IANA zone entered alongside the instant. */
    scheduleLocal?: string
    scheduleTimezone?: string
    /** Alias of the chosen template: the key its approved template is bound under. */
    alias?: string
    content: ISendContent
    secretAttributeNames: string[]
}

/**
 * What the send task takes. Each voter gets the notification on their own
 * channel, so it carries the content for every channel and no channel choice:
 * `channel_selection` and `communication_method` are no longer written.
 */
export interface ISendTemplatePayload<Audience extends string, VoterId> extends ISendContent {
    audience_selection: Audience
    audience_voter_ids?: VoterId[]
    schedule_now: boolean
    /** RFC 3339 instant. */
    schedule_date?: string
    /** Wall time and IANA zone entered alongside the notification instant. */
    schedule_local?: string
    schedule_timezone?: string
    alias?: string
    secret_attribute_names: string[]
}

/**
 * An instant message as it is sent: the provider's template and language only
 * when set, so that without them the event's binding is used.
 */
const instantMessage = (
    config: IInstantMessageConfig | undefined
): IInstantMessageConfig | undefined => {
    if (!config) {
        return undefined
    }
    const providerTemplate = config.provider_template?.trim()
    const providerLanguage = config.provider_language?.trim()
    return {
        message: config.message,
        parameters: config.parameters,
        ...(providerTemplate ? {provider_template: providerTemplate} : {}),
        ...(providerLanguage ? {provider_language: providerLanguage} : {}),
    }
}

const sendContent = (content: ISendContent): ISendContent => ({
    ...content,
    whatsapp: instantMessage(content.whatsapp),
    viber: instantMessage(content.viber),
    messenger: instantMessage(content.messenger),
})

export const buildSendTemplatePayload = <Audience extends string, VoterId>(
    state: ISendTemplateState<Audience, VoterId>
): ISendTemplatePayload<Audience, VoterId> => {
    return {
        audience_selection: state.audienceSelection,
        audience_voter_ids: state.voterIds,
        schedule_now: state.scheduleNow,
        schedule_date: state.scheduleDate,
        schedule_local: state.scheduleLocal,
        schedule_timezone: state.scheduleTimezone,
        ...(state.alias ? {alias: state.alias} : {}),
        ...sendContent(state.content),
        secret_attribute_names: state.secretAttributeNames,
    }
}

interface ITemplateRow {
    communication_method?: string | null
    template: {alias?: string; selected_methods?: IMethods | null}
}

/** Templates that send a message on at least one channel. */
export const sendableTemplates = <Row extends ITemplateRow>(rows: Row[]): Row[] =>
    rows.filter((row) =>
        MESSAGE_TEMPLATE_METHODS.some((candidate) => templateOffersMethod(row, candidate))
    )

const hasContent = (content: ISendContent, channel: EMessageChannel): boolean => {
    switch (channel) {
        case EMessageChannel.EMAIL:
            return !!content.email?.subject?.trim() || !!content.email?.plaintext_body?.trim()
        case EMessageChannel.SMS:
            return !!content.sms?.message?.trim()
        case EMessageChannel.WHATSAPP:
            return !!content.whatsapp?.message?.trim()
        case EMessageChannel.VIBER:
            return !!content.viber?.message?.trim()
        case EMessageChannel.MESSENGER:
            return !!content.messenger?.message?.trim()
    }
}

/** Channels among `channels` the content has nothing to send on. */
export const missingChannelContent = (
    content: ISendContent,
    channels: EMessageChannel[]
): EMessageChannel[] => channels.filter((channel) => !hasContent(content, channel))

/**
 * The channels a send reaches voters on, as the backend decides it
 * (`eligible_channels`): the event's notice channels, or email and SMS for an
 * event without a configuration. An event whose configuration sends no
 * notices reaches nobody; it is never shown as email and SMS.
 */
export type TSendNoticeChannels =
    /** The event's configuration can't be read: nothing is guessed. */
    | {kind: "NO_PERMISSION"}
    /** The configuration turns notices on for no channel: every voter would fail. */
    | {kind: "NONE"}
    | {kind: "CHANNELS"; channels: EMessageChannel[]}

/** Email and SMS, as before messaging: the backend's channels without a configuration. */
const UNCONFIGURED_CHANNELS = [EMessageChannel.SMS, EMessageChannel.EMAIL]

export const sendNoticeChannels = (
    canRead: boolean,
    /** The stored configuration; `null` when the event has none. */
    config: IEventMessagingConfig | null
): TSendNoticeChannels => {
    if (!canRead) return {kind: "NO_PERMISSION"}
    if (!config) return {kind: "CHANNELS", channels: UNCONFIGURED_CHANNELS}
    const channels = enabledChannels(config, EMessagePurpose.NOTICE)
    return channels.length ? {kind: "CHANNELS", channels} : {kind: "NONE"}
}

/**
 * Who sends a channel's notices, as the backend picks it (`account_for`): the
 * event's account, else the tenant's default account of the channel. Email
 * and SMS without an account go out through the deployment's own sender
 * (`"ENVIRONMENT"`); any other channel without one can't send (`null`).
 */
export const sendingAccountFor = (
    config: IEventMessagingConfig | null,
    accounts: IMessagingAccount[],
    channel: EMessageChannel
): IMessagingAccount | "ENVIRONMENT" | null => {
    const chosen = config?.channels.find((entry) => entry.channel === channel)
    const account = chosen
        ? accounts.find((candidate) => candidate.id === chosen.account_id)
        : accounts.find((candidate) => candidate.channel === channel && candidate.is_default)
    if (account) return account
    return !chosen && UNCONFIGURED_CHANNELS.includes(channel) ? "ENVIRONMENT" : null
}
