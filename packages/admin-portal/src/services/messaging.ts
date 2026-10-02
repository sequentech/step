// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Port of the readiness and validation rules in
// packages/sequent-core/src/types/messaging.rs. The server stays
// authoritative; these let the screens explain a blocked control before saving.
import {
    ECredentialName,
    EDeliveryFeedback,
    EMessageAttemptState,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    EReadinessBlocker,
    ERecipientKind,
    EVENT_MESSAGING_CONFIG_VERSION,
    IAccountCheck,
    IAccountSender,
    IAccountSummary,
    IEventChannelConfig,
    IEventMessagingConfig,
    IMessagingConfigError,
    IProviderCapabilities,
    IPurposeReadiness,
    MESSAGE_PURPOSES,
} from "@/types/messaging"

export const recipientKind = (channel: EMessageChannel): ERecipientKind => {
    switch (channel) {
        case EMessageChannel.EMAIL:
            return ERecipientKind.EMAIL_ADDRESS
        case EMessageChannel.SMS:
        case EMessageChannel.WHATSAPP:
        case EMessageChannel.VIBER:
            return ERecipientKind.PHONE_NUMBER
        case EMessageChannel.MESSENGER:
            return ERecipientKind.PAGE_SCOPED_ID
    }
}

/** The channel a provider sends on; `null` for the console, which can stand in for any. */
export const providerChannel = (provider: EMessagingProvider): EMessageChannel | null => {
    switch (provider) {
        case EMessagingProvider.AWS_SES:
        case EMessagingProvider.SMTP:
            return EMessageChannel.EMAIL
        case EMessagingProvider.AWS_SNS:
            return EMessageChannel.SMS
        case EMessagingProvider.WHATSAPP_CLOUD_API:
            return EMessageChannel.WHATSAPP
        case EMessagingProvider.MESSENGER_SEND_API:
            return EMessageChannel.MESSENGER
        case EMessagingProvider.VIBER_INFOBIP:
            return EMessageChannel.VIBER
        case EMessagingProvider.CONSOLE:
            return null
    }
}

export const providersForChannel = (channel: EMessageChannel): EMessagingProvider[] =>
    Object.values(EMessagingProvider).filter((provider) => {
        const own = providerChannel(provider)
        return own === null || own === channel
    })

export const providerCapabilities = (
    provider: EMessagingProvider,
    channel: EMessageChannel
): IProviderCapabilities | null => {
    const own = providerChannel(provider)
    if (own !== null && own !== channel) {
        return null
    }
    const both = [...MESSAGE_PURPOSES]
    const base = {
        channel,
        recipient: recipientKind(channel),
        purposes: both,
        template_required_for: [] as EMessagePurpose[],
        delivery_feedback: EDeliveryFeedback.PROVIDER_RECEIPTS,
        reconciliation: false,
        conversation_window_hours: null as number | null,
        requires_provider_approval: false,
    }
    switch (provider) {
        case EMessagingProvider.AWS_SES:
            return base
        case EMessagingProvider.SMTP:
        case EMessagingProvider.AWS_SNS:
        case EMessagingProvider.CONSOLE:
            return {...base, delivery_feedback: EDeliveryFeedback.UNAVAILABLE}
        case EMessagingProvider.WHATSAPP_CLOUD_API:
            return {
                ...base,
                template_required_for: [...both],
                conversation_window_hours: 24,
                requires_provider_approval: true,
            }
        case EMessagingProvider.MESSENGER_SEND_API:
            return {...base, conversation_window_hours: 24}
        case EMessagingProvider.VIBER_INFOBIP:
            return {...base, template_required_for: [...both], reconciliation: true}
    }
}

/** Readiness of `purpose` for `language` (any language when null) on an account. */
export const purposeReadiness = (
    provider: EMessagingProvider,
    channel: EMessageChannel,
    approval: EProviderApproval | null | undefined,
    check: IAccountCheck | null | undefined,
    purpose: EMessagePurpose,
    language: string | null | undefined
): IPurposeReadiness => {
    const capabilities = providerCapabilities(provider, channel)
    if (!capabilities) {
        return {purpose, blockers: [EReadinessBlocker.UNSUPPORTED_PURPOSE]}
    }
    const blockers: EReadinessBlocker[] = []
    if (!check?.connected) {
        blockers.push(EReadinessBlocker.NOT_CONNECTED)
    }
    if (!capabilities.purposes.includes(purpose)) {
        blockers.push(EReadinessBlocker.UNSUPPORTED_PURPOSE)
    }
    if (capabilities.requires_provider_approval && approval !== EProviderApproval.CONFIRMED) {
        blockers.push(EReadinessBlocker.NEEDS_PROVIDER_APPROVAL)
    }
    if (!check?.production_access) {
        blockers.push(EReadinessBlocker.NEEDS_PRODUCTION_ACCESS)
    }
    if (capabilities.template_required_for.includes(purpose)) {
        const languages = check?.approved_templates?.[purpose]
        const approved = languages
            ? language
                ? languages.includes(language)
                : languages.length > 0
            : false
        if (!approved) {
            blockers.push(EReadinessBlocker.NEEDS_APPROVED_TEMPLATE)
        }
    }
    return {purpose, blockers}
}

export const requiredCredentials = (provider: EMessagingProvider): ECredentialName[] => {
    switch (provider) {
        case EMessagingProvider.WHATSAPP_CLOUD_API:
        case EMessagingProvider.MESSENGER_SEND_API:
            return [
                ECredentialName.ACCESS_TOKEN,
                ECredentialName.APP_SECRET,
                ECredentialName.VERIFY_TOKEN,
            ]
        case EMessagingProvider.VIBER_INFOBIP:
            return [ECredentialName.API_KEY]
        case EMessagingProvider.SMTP:
            return [ECredentialName.SMTP_PASSWORD]
        case EMessagingProvider.AWS_SES:
        case EMessagingProvider.AWS_SNS:
        case EMessagingProvider.CONSOLE:
            return []
    }
}

export const acceptedCredentials = (provider: EMessagingProvider): ECredentialName[] => {
    switch (provider) {
        case EMessagingProvider.AWS_SES:
        case EMessagingProvider.AWS_SNS:
            return [ECredentialName.AWS_ACCESS_KEY_ID, ECredentialName.AWS_SECRET_ACCESS_KEY]
        default:
            return requiredCredentials(provider)
    }
}

/** Credentials an administrator types; the verify token is generated by Step. */
export const typedCredentials = (provider: EMessagingProvider): ECredentialName[] =>
    acceptedCredentials(provider).filter((name) => name !== ECredentialName.VERIFY_TOKEN)

export const isMetaProvider = (provider: EMessagingProvider): boolean =>
    provider === EMessagingProvider.WHATSAPP_CLOUD_API ||
    provider === EMessagingProvider.MESSENGER_SEND_API

/** Path of the account's webhook, relative to harvest's public base URL. */
export const webhookPath = (
    provider: EMessagingProvider,
    webhookKey: string | null | undefined
): string | null => {
    if (!webhookKey) {
        return null
    }
    if (isMetaProvider(provider)) {
        return `/webhooks/meta/${webhookKey}`
    }
    if (provider === EMessagingProvider.VIBER_INFOBIP) {
        return `/webhooks/viber/${webhookKey}`
    }
    return null
}

export const channelStatisticsKey = (channel: EMessageChannel): string => {
    switch (channel) {
        case EMessageChannel.EMAIL:
            return "num_emails_sent"
        case EMessageChannel.SMS:
            return "num_sms_sent"
        case EMessageChannel.WHATSAPP:
            return "num_whatsapp_sent"
        case EMessageChannel.VIBER:
            return "num_viber_sent"
        case EMessageChannel.MESSENGER:
            return "num_messenger_sent"
    }
}

/** Sender as voters see it: sender name, number or Page name. */
export const accountSenderLabel = (sender: IAccountSender): string | null => {
    switch (sender.provider) {
        case EMessagingProvider.AWS_SES:
        case EMessagingProvider.SMTP:
            return sender.from_name || sender.from_address
        case EMessagingProvider.AWS_SNS:
            return sender.sender_id || sender.origination_number || null
        case EMessagingProvider.WHATSAPP_CLOUD_API:
            return sender.display_name || sender.display_phone_number
        case EMessagingProvider.MESSENGER_SEND_API:
            return sender.page_name || sender.page_id
        case EMessagingProvider.VIBER_INFOBIP:
            return sender.sender
        case EMessagingProvider.CONSOLE:
            return null
    }
}

export const emptyEventMessagingConfig = (): IEventMessagingConfig => ({
    version: EVENT_MESSAGING_CONFIG_VERSION,
    channels: [],
    notice_fallback: [],
    election_channels: {},
    reply_text: {},
})

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const asArray = <T>(value: unknown): T[] => (Array.isArray(value) ? (value as T[]) : [])

const asRecord = <T>(value: unknown): Record<string, T> =>
    isRecord(value) ? (value as Record<string, T>) : {}

/** Reads the `messaging:config` annotation, filling the defaults serde applies. */
export const parseEventMessagingConfig = (raw: unknown): IEventMessagingConfig => {
    let value: unknown = raw
    if (typeof raw === "string") {
        try {
            value = JSON.parse(raw)
        } catch {
            return emptyEventMessagingConfig()
        }
    }
    if (!isRecord(value)) {
        return emptyEventMessagingConfig()
    }
    const channels = asArray<Record<string, unknown>>(value.channels)
        .filter(isRecord)
        .map(
            (channel): IEventChannelConfig => ({
                channel: channel.channel as EMessageChannel,
                account_id: String(channel.account_id ?? ""),
                purposes: asArray<EMessagePurpose>(channel.purposes),
                templates: asArray(channel.templates),
                out_of_window:
                    (channel.out_of_window as EOutOfWindowPolicy | undefined) ??
                    EOutOfWindowPolicy.DISABLED,
            })
        )
    return {
        version: typeof value.version === "number" ? value.version : EVENT_MESSAGING_CONFIG_VERSION,
        channels,
        notice_fallback: asArray<EMessageChannel>(value.notice_fallback),
        election_channels: asRecord<EMessageChannel[]>(value.election_channels),
        reply_text: asRecord<string>(value.reply_text),
    }
}

/** Channels enabled for `purpose`, optionally restricted to an election. */
export const enabledChannels = (
    config: IEventMessagingConfig,
    purpose: EMessagePurpose,
    electionId?: string | null
): EMessageChannel[] => {
    const restriction = electionId ? config.election_channels[electionId] : undefined
    return config.channels
        .filter((channel) => channel.purposes.includes(purpose))
        .map((channel) => channel.channel)
        .filter((channel) => !restriction || restriction.includes(channel))
}

/** Same checks, in the same order, as `EventMessagingConfig::validate`. */
export const validateEventMessagingConfig = (
    config: IEventMessagingConfig,
    tenantId: string,
    accounts: IAccountSummary[],
    electionIds: string[]
): IMessagingConfigError[] => {
    const errors: IMessagingConfigError[] = []
    if (config.version !== EVENT_MESSAGING_CONFIG_VERSION) {
        errors.push({kind: "UNSUPPORTED_VERSION", version: config.version})
    }
    const accountsById = new Map(accounts.map((account) => [account.id, account]))
    const seen = new Set<EMessageChannel>()
    for (const channelConfig of config.channels) {
        const channel = channelConfig.channel
        if (seen.has(channel)) {
            errors.push({kind: "DUPLICATE_CHANNEL", channel})
            continue
        }
        seen.add(channel)
        const account = accountsById.get(channelConfig.account_id)
        if (!account) {
            errors.push({kind: "UNKNOWN_ACCOUNT", channel, account_id: channelConfig.account_id})
            continue
        }
        if (account.tenant_id !== tenantId) {
            errors.push({kind: "ACCOUNT_OF_ANOTHER_TENANT", account_id: account.id})
            continue
        }
        if (account.channel !== channel) {
            errors.push({kind: "ACCOUNT_CHANNEL_MISMATCH", channel, account_id: account.id})
            continue
        }
        for (const purpose of channelConfig.purposes) {
            const readiness = purposeReadiness(
                account.provider,
                channel,
                account.provider_approval,
                account.check,
                purpose,
                null
            )
            if (readiness.blockers.length) {
                errors.push({
                    kind: "PURPOSE_NOT_READY",
                    channel,
                    purpose,
                    blockers: readiness.blockers,
                })
            }
        }
        const templatePurposes =
            providerCapabilities(account.provider, channel)?.template_required_for ?? []
        for (const binding of channelConfig.templates) {
            if (!templatePurposes.includes(binding.purpose)) {
                continue
            }
            const approved =
                account.check.approved_templates?.[binding.purpose]?.includes(binding.language) ??
                false
            if (!approved) {
                errors.push({
                    kind: "TEMPLATE_NOT_APPROVED",
                    channel,
                    purpose: binding.purpose,
                    language: binding.language,
                })
            }
        }
        if (
            channelConfig.out_of_window !== EOutOfWindowPolicy.DISABLED &&
            channel !== EMessageChannel.MESSENGER
        ) {
            errors.push({kind: "OUT_OF_WINDOW_NOT_SUPPORTED", channel})
        }
    }
    const noticeChannels = new Set(enabledChannels(config, EMessagePurpose.NOTICE))
    const seenFallback = new Set<EMessageChannel>()
    for (const channel of config.notice_fallback) {
        if (seenFallback.has(channel)) {
            errors.push({kind: "DUPLICATE_FALLBACK_CHANNEL", channel})
        } else {
            seenFallback.add(channel)
            if (!noticeChannels.has(channel)) {
                errors.push({kind: "FALLBACK_CHANNEL_NOT_ENABLED", channel})
            }
        }
    }
    const enabled = new Set(config.channels.map((channel) => channel.channel))
    for (const electionId of Object.keys(config.election_channels).sort()) {
        if (!electionIds.includes(electionId)) {
            errors.push({kind: "UNKNOWN_ELECTION", election_id: electionId})
            continue
        }
        for (const channel of config.election_channels[electionId]) {
            if (!enabled.has(channel)) {
                errors.push({
                    kind: "ELECTION_CHANNEL_NOT_ENABLED",
                    election_id: electionId,
                    channel,
                })
            }
        }
    }
    return errors
}

export interface IStateCount {
    state: EMessageAttemptState
    count: number
}

/** Message counts per state; `delivered` is null when the provider sends no receipts. */
export interface IDeliverySummary {
    queued: number
    accepted: number
    delivered: number | null
    failed: number
    unknown: number
}

export const deliverySummary = (
    counts: IStateCount[],
    feedback: EDeliveryFeedback
): IDeliverySummary => {
    const total = (state: EMessageAttemptState) =>
        counts.filter((row) => row.state === state).reduce((sum, row) => sum + row.count, 0)
    return {
        queued: total(EMessageAttemptState.QUEUED),
        accepted: total(EMessageAttemptState.ACCEPTED),
        delivered:
            feedback === EDeliveryFeedback.UNAVAILABLE
                ? null
                : total(EMessageAttemptState.DELIVERED),
        failed: total(EMessageAttemptState.FAILED),
        unknown: total(EMessageAttemptState.UNKNOWN),
    }
}
