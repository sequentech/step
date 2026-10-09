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
    EReadinessPolicy,
    IMessagingAccount,
    IMessagingConfigError,
    IProviderCapabilities,
    IPublicMessagingChannels,
    IPurposeReadiness,
    ITemplateBinding,
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
        case EMessagingProvider.HTTP_API:
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
        case EMessagingProvider.HTTP_API:
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

/** What an account can do on `channel`; a configured HTTP provider declares its own. */
export const senderCapabilities = (
    sender: IAccountSender,
    channel: EMessageChannel
): IProviderCapabilities | null => {
    const capabilities = providerCapabilities(sender.provider, channel)
    if (!capabilities || sender.provider !== EMessagingProvider.HTTP_API) {
        return capabilities
    }
    return {
        ...capabilities,
        template_required_for: [...(sender.template_required_for ?? [])],
        conversation_window_hours: sender.conversation_window_hours ?? null,
        delivery_feedback: sender.reports
            ? EDeliveryFeedback.PROVIDER_RECEIPTS
            : EDeliveryFeedback.UNAVAILABLE,
        reconciliation: !!sender.reconcile,
    }
}

export const accountCapabilities = (account: IMessagingAccount): IProviderCapabilities | null =>
    senderCapabilities(account.sender, account.channel)

/**
 * Readiness of `purpose` for `language` (any language when null) on an account.
 * With `ADMIN_CONFIRMED` the administrator's statement replaces what the
 * provider's check tells; provider approval is still confirmed separately.
 */
export const purposeReadiness = (
    capabilities: IProviderCapabilities | null,
    approval: EProviderApproval | null | undefined,
    policy: EReadinessPolicy | null | undefined,
    check: IAccountCheck | null | undefined,
    purpose: EMessagePurpose,
    language: string | null | undefined
): IPurposeReadiness => {
    if (!capabilities) {
        return {purpose, blockers: [EReadinessBlocker.UNSUPPORTED_PURPOSE]}
    }
    const checked = policy !== EReadinessPolicy.ADMIN_CONFIRMED
    const blockers: EReadinessBlocker[] = []
    if (checked && !check?.connected) {
        blockers.push(EReadinessBlocker.NOT_CONNECTED)
    }
    if (!capabilities.purposes.includes(purpose)) {
        blockers.push(EReadinessBlocker.UNSUPPORTED_PURPOSE)
    }
    if (capabilities.requires_provider_approval && approval !== EProviderApproval.CONFIRMED) {
        blockers.push(EReadinessBlocker.NEEDS_PROVIDER_APPROVAL)
    }
    if (checked && !check?.production_access) {
        blockers.push(EReadinessBlocker.NEEDS_PRODUCTION_ACCESS)
    }
    if (checked && capabilities.template_required_for.includes(purpose)) {
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

/**
 * Credentials a provider needs before it can send. Webhook credentials are not
 * among them: sending works before callbacks are set up.
 */
export const requiredCredentials = (provider: EMessagingProvider): ECredentialName[] => {
    switch (provider) {
        case EMessagingProvider.WHATSAPP_CLOUD_API:
        case EMessagingProvider.MESSENGER_SEND_API:
            return [ECredentialName.ACCESS_TOKEN]
        case EMessagingProvider.VIBER_INFOBIP:
            return [ECredentialName.API_KEY]
        case EMessagingProvider.SMTP:
            return [ECredentialName.SMTP_PASSWORD]
        case EMessagingProvider.AWS_SES:
        case EMessagingProvider.AWS_SNS:
        case EMessagingProvider.HTTP_API:
        case EMessagingProvider.CONSOLE:
            return []
    }
}

export const acceptedCredentials = (provider: EMessagingProvider): ECredentialName[] => {
    switch (provider) {
        case EMessagingProvider.AWS_SES:
        case EMessagingProvider.AWS_SNS:
            return [ECredentialName.AWS_ACCESS_KEY_ID, ECredentialName.AWS_SECRET_ACCESS_KEY]
        case EMessagingProvider.WHATSAPP_CLOUD_API:
        case EMessagingProvider.MESSENGER_SEND_API:
            return [
                ECredentialName.ACCESS_TOKEN,
                ECredentialName.APP_SECRET,
                ECredentialName.VERIFY_TOKEN,
            ]
        case EMessagingProvider.HTTP_API:
            return [
                ECredentialName.API_KEY,
                ECredentialName.API_SECRET,
                ECredentialName.ACCESS_TOKEN,
                ECredentialName.USERNAME,
                ECredentialName.PASSWORD,
                ECredentialName.WEBHOOK_SECRET,
            ]
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
    if (provider === EMessagingProvider.HTTP_API) {
        return `/webhooks/http/${webhookKey}`
    }
    return null
}

/** Whether the provider posts delivery reports or replies to a callback of the account. */
export const hasWebhook = (provider: EMessagingProvider): boolean =>
    isMetaProvider(provider) ||
    provider === EMessagingProvider.VIBER_INFOBIP ||
    provider === EMessagingProvider.HTTP_API

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
        case EMessagingProvider.HTTP_API:
            return sender.label || null
        case EMessagingProvider.CONSOLE:
            return null
    }
}

/** What validation and the public projection need to know about an account. */
export const accountSummary = (account: IMessagingAccount): IAccountSummary => ({
    id: account.id,
    tenant_id: account.tenant_id,
    channel: account.channel,
    provider: account.provider,
    capabilities: accountCapabilities(account),
    provider_approval: account.provider_approval ?? EProviderApproval.PENDING,
    readiness: account.readiness ?? EReadinessPolicy.PROVIDER_CHECK,
    check: account.status ?? {connected: false, production_access: false},
    public_label: accountSenderLabel(account.sender),
    messenger_page:
        account.sender.provider === EMessagingProvider.MESSENGER_SEND_API
            ? {
                  page_id: account.sender.page_id,
                  username: account.sender.page_username ?? null,
                  name: account.sender.page_name ?? null,
              }
            : null,
})

/**
 * Whether notices may be sent with an approved template outside the
 * conversation window: only on a channel with a window whose notices are
 * otherwise free text.
 */
export const supportsOutOfWindow = (capabilities: IProviderCapabilities | null): boolean =>
    !!capabilities &&
    capabilities.conversation_window_hours !== null &&
    !capabilities.template_required_for.includes(EMessagePurpose.NOTICE)

/** The language code to send to the provider. */
export const bindingProviderLanguage = (binding: ITemplateBinding): string =>
    binding.provider_language || binding.language

/**
 * The approved template for a message on `channel`. The most specific binding
 * wins: the message's key and language, then the key in any language, then the
 * purpose's default in the language, then any default for the purpose.
 */
export const templateFor = (
    config: IEventMessagingConfig,
    channel: EMessageChannel,
    purpose: EMessagePurpose,
    key: string | null | undefined,
    language: string | null | undefined
): ITemplateBinding | null => {
    const bindings = (
        config.channels.find((entry) => entry.channel === channel)?.templates ?? []
    ).filter((binding) => binding.purpose === purpose)
    const keyed = (binding: ITemplateBinding) => !!key && binding.key === key
    const isDefault = (binding: ITemplateBinding) => !binding.key
    const inLanguage = (binding: ITemplateBinding) =>
        !!language &&
        (binding.language === language || bindingProviderLanguage(binding) === language)
    return (
        bindings.find((binding) => keyed(binding) && inLanguage(binding)) ??
        bindings.find(keyed) ??
        bindings.find((binding) => isDefault(binding) && inLanguage(binding)) ??
        bindings.find(isDefault) ??
        null
    )
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
                account.capabilities,
                account.provider_approval,
                account.readiness,
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
        const templatePurposes = account.capabilities?.template_required_for ?? []
        for (const binding of channelConfig.templates) {
            if (
                !templatePurposes.includes(binding.purpose) ||
                account.readiness === EReadinessPolicy.ADMIN_CONFIRMED
            ) {
                continue
            }
            const languages = account.check.approved_templates?.[binding.purpose] ?? []
            const approved =
                languages.includes(bindingProviderLanguage(binding)) ||
                languages.includes(binding.language)
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
            !supportsOutOfWindow(account.capabilities)
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

/**
 * What Keycloak pages may see: channel labels and purposes, never account
 * identifiers. `electionLabels` are the names a voter record may hold for each
 * election; only restricted elections keep theirs.
 */
export const publicProjection = (
    config: IEventMessagingConfig,
    accounts: IAccountSummary[],
    electionLabels: Record<string, string[]>
): IPublicMessagingChannels => {
    const accountsById = new Map(accounts.map((account) => [account.id, account]))
    return {
        version: EVENT_MESSAGING_CONFIG_VERSION,
        channels: config.channels
            .filter((entry) => entry.purposes.length > 0)
            .flatMap((entry) => {
                const account = accountsById.get(entry.account_id)
                return account
                    ? [
                          {
                              channel: entry.channel,
                              purposes: [...entry.purposes],
                              sender_label: account.public_label ?? null,
                              messenger_page:
                                  entry.channel === EMessageChannel.MESSENGER
                                      ? (account.messenger_page ?? null)
                                      : null,
                          },
                      ]
                    : []
            }),
        election_channels: {...config.election_channels},
        election_labels: Object.fromEntries(
            Object.entries(electionLabels).filter(([id]) => id in config.election_channels)
        ),
    }
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
