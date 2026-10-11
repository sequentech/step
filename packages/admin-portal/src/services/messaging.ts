// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Port of the readiness and validation rules in
// packages/sequent-core/src/types/messaging.rs. The server stays
// authoritative; these let the screens explain a blocked control before saving.
import {
    EApprovalSource,
    ECredentialName,
    EDeliveryFeedback,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EOutOfWindowPolicy,
    EProviderApproval,
    EReadinessBlocker,
    ERecipientKind,
    EVENT_MESSAGING_CONFIG_VERSION,
    GENERATED_CREDENTIALS,
    IAccountCallbackStatus,
    IAccountCheck,
    IAccountIdentity,
    IAccountSender,
    IAccountSummary,
    IEventChannelConfig,
    IEventMessagingConfig,
    IChannelMessageStats,
    IMessagingAccount,
    IMessagingConfigError,
    IProviderCapabilities,
    IPublicMessagingChannels,
    IPurposeReadiness,
    ITemplateBinding,
    MESSAGE_CHANNELS,
    MESSAGE_PURPOSES,
    UNSUPPORTED_PROVIDERS,
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

/** Whether accounts of `provider` can no longer send and must be replaced. */
export const isUnsupportedProvider = (provider: EMessagingProvider): boolean =>
    UNSUPPORTED_PROVIDERS.includes(provider)

/** The channel a provider sends on; `null` for an unsupported provider. */
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

/** The providers a new account of `channel` can use: an adapter per partner. */
export const providersForChannel = (channel: EMessageChannel): EMessagingProvider[] =>
    Object.values(EMessagingProvider).filter((provider) => providerChannel(provider) === channel)

export const providerCapabilities = (
    provider: EMessagingProvider,
    channel: EMessageChannel
): IProviderCapabilities | null => {
    if (providerChannel(provider) !== channel) {
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
        case EMessagingProvider.SMTP:
        case EMessagingProvider.AWS_SNS:
            // SES reports deliveries only through a notification topic: see senderCapabilities.
            return {...base, delivery_feedback: EDeliveryFeedback.UNAVAILABLE}
        case EMessagingProvider.HTTP_API:
        case EMessagingProvider.CONSOLE:
            return null
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

/**
 * What an account can do on `channel`. SES reports deliveries only when a
 * notification topic is set; without one, delivery is unavailable, never 0.
 */
export const senderCapabilities = (
    sender: IAccountSender,
    channel: EMessageChannel
): IProviderCapabilities | null => {
    const capabilities = providerCapabilities(sender.provider, channel)
    if (
        capabilities &&
        sender.provider === EMessagingProvider.AWS_SES &&
        sender.notification_topic_arn?.trim()
    ) {
        return {...capabilities, delivery_feedback: EDeliveryFeedback.PROVIDER_RECEIPTS}
    }
    return capabilities
}

export const accountCapabilities = (account: IMessagingAccount): IProviderCapabilities | null =>
    senderCapabilities(account.sender, account.channel)

/**
 * Port of `AccountCheck::approval`: the provider approval the check reported,
 * where the provider has an API for it, else the one an administrator entered.
 */
export const accountApproval = (
    entered: EProviderApproval | null | undefined,
    check: IAccountCheck | null | undefined
): {approval: EProviderApproval | null; source: EApprovalSource} => {
    const reported = check?.details?.provider_approval
    return reported
        ? {approval: reported, source: EApprovalSource.PROVIDER_CHECK}
        : {approval: entered ?? null, source: EApprovalSource.ADMIN_ENTERED}
}

/**
 * What identifies a sender to its provider (`AccountSender::identity`): the
 * provider and its identifying ids, trimmed, absent ones left out. Names,
 * labels, API versions and templates are not part of it. Keep in step with
 * the Rust function; `messaging.test.ts` holds the same cases.
 */
export const accountIdentity = (sender: IAccountSender): IAccountIdentity => {
    const fields: Array<[string, string | null | undefined]> = (() => {
        switch (sender.provider) {
            case EMessagingProvider.AWS_SES:
                return [
                    ["from_address", sender.from_address],
                    ["region", sender.region],
                ]
            case EMessagingProvider.SMTP:
                return [
                    ["from_address", sender.from_address],
                    ["server_url", sender.server_url],
                ]
            case EMessagingProvider.AWS_SNS:
                return [
                    ["sender_id", sender.sender_id],
                    ["origination_number", sender.origination_number],
                    ["region", sender.region],
                ]
            case EMessagingProvider.WHATSAPP_CLOUD_API:
                return [
                    ["business_account_id", sender.business_account_id],
                    ["phone_number_id", sender.phone_number_id],
                ]
            case EMessagingProvider.MESSENGER_SEND_API:
                return [["page_id", sender.page_id]]
            case EMessagingProvider.VIBER_INFOBIP:
                return [
                    ["base_url", sender.base_url?.replace(/\/+$/, "")],
                    ["sender", sender.sender],
                ]
            default:
                return []
        }
    })()
    const ids: Record<string, string> = {}
    for (const [name, value] of fields) {
        const trimmed = value?.trim()
        if (trimmed) ids[name] = trimmed
    }
    return {provider: sender.provider, ids}
}

const sameIdentity = (a: IAccountIdentity, b: IAccountIdentity): boolean => {
    const keys = (identity: IAccountIdentity) => Object.keys(identity.ids).sort()
    return (
        a.provider === b.provider &&
        keys(a).join() === keys(b).join() &&
        keys(a).every((key) => a.ids[key] === b.ids[key])
    )
}

/**
 * Whether a check describes an account whose identity is `current`
 * (`AccountCheck::is_for`). A check that doesn't name its identity was
 * stored before identities were recorded and counts.
 */
export const checkIsFor = (
    check: IAccountCheck | null | undefined,
    current: IAccountIdentity
): boolean => {
    const checked = check?.details?.identity
    return !checked || sameIdentity(checked, current)
}

/**
 * Whether the account's last check was of another identity (an earlier
 * number, Page or sender): it sends nothing until checked as it is now.
 */
export const checkedForAnotherIdentity = (account: IMessagingAccount): boolean =>
    !!account.status?.checked_at && !checkIsFor(account.status, accountIdentity(account.sender))

/**
 * Readiness of `purpose` for `language` (any language when null) on an account
 * whose identity is `identity` (null: unknown, the check is taken as the
 * account's). It always comes from the provider's check: no statement of an
 * administrator replaces it. An account of a removed provider is unsupported.
 * A check of another identity readies nothing: the same rule decides at
 * config save and at send time.
 */
export const purposeReadiness = (
    provider: EMessagingProvider,
    capabilities: IProviderCapabilities | null,
    approval: EProviderApproval | null | undefined,
    check: IAccountCheck | null | undefined,
    purpose: EMessagePurpose,
    language: string | null | undefined,
    identity?: IAccountIdentity | null
): IPurposeReadiness => {
    if (isUnsupportedProvider(provider)) {
        return {purpose, blockers: [EReadinessBlocker.UNSUPPORTED_PROVIDER]}
    }
    if (!capabilities) {
        return {purpose, blockers: [EReadinessBlocker.UNSUPPORTED_PURPOSE]}
    }
    const blockers: EReadinessBlocker[] = []
    const unchecked = !!check?.checked_at && !!identity && !checkIsFor(check, identity)
    if (unchecked) {
        blockers.push(EReadinessBlocker.UNCHECKED)
    }
    if (!unchecked && !check?.connected) {
        blockers.push(EReadinessBlocker.NOT_CONNECTED)
    }
    if (!capabilities.purposes.includes(purpose)) {
        blockers.push(EReadinessBlocker.UNSUPPORTED_PURPOSE)
    }
    if (
        capabilities.requires_provider_approval &&
        accountApproval(approval, check).approval !== EProviderApproval.CONFIRMED
    ) {
        blockers.push(EReadinessBlocker.NEEDS_PROVIDER_APPROVAL)
    }
    if (unchecked) {
        // The rest comes from the check, which describes another identity.
        return {purpose, blockers}
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
    const callback = callbackBlocker(capabilities.channel, purpose, callbackConfigured(check))
    if (callback) {
        blockers.push(callback)
    }
    return {purpose, blockers}
}

/**
 * Whether callbacks can reach the account: false only when
 * `messaging_callback_status` says the callback address isn't configured.
 */
export const callbackConfigured = (check: IAccountCheck | null | undefined): boolean =>
    !check?.callback_blockers?.includes(EReadinessBlocker.CALLBACK_NOT_CONFIGURED)

/** Puts each account's callback status on its check, as the screens read it. */
export const withCallbackStatus = (
    accounts: IMessagingAccount[],
    statuses: IAccountCallbackStatus[] | null | undefined
): IMessagingAccount[] => {
    if (!statuses) return accounts
    const byAccount = new Map(statuses.map((status) => [status.account_id, status]))
    return accounts.map((account) => {
        const status = byAccount.get(account.id)
        return status && account.status
            ? {
                  ...account,
                  status: {
                      ...account.status,
                      callback_blockers: status.blockers,
                      callback_reasons: status.reasons,
                  },
              }
            : account
    })
}

/**
 * Port of `callback_blocker`: Messenger codes need the callback address,
 * because a code goes out only after the voter's Get Started arrives as a
 * callback. Other purposes and channels send without one.
 */
export const callbackBlocker = (
    channel: EMessageChannel,
    purpose: EMessagePurpose,
    configured: boolean
): EReadinessBlocker | null =>
    channel === EMessageChannel.MESSENGER && purpose === EMessagePurpose.OTP && !configured
        ? EReadinessBlocker.CALLBACK_NOT_CONFIGURED
        : null

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

/** Credentials an account of `provider` keeps; unsupported providers take none. */

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
        case EMessagingProvider.VIBER_INFOBIP:
            return [
                ECredentialName.API_KEY,
                ECredentialName.REPORT_USERNAME,
                ECredentialName.REPORT_PASSWORD,
            ]
        default:
            return requiredCredentials(provider)
    }
}

/** Credentials an administrator types; the verify token and report credentials are generated. */
export const typedCredentials = (provider: EMessagingProvider): ECredentialName[] =>
    acceptedCredentials(provider).filter((name) => !GENERATED_CREDENTIALS.includes(name))

export const isMetaProvider = (provider: EMessagingProvider): boolean =>
    provider === EMessagingProvider.WHATSAPP_CLOUD_API ||
    provider === EMessagingProvider.MESSENGER_SEND_API

/**
 * Whether the provider posts delivery reports (and, for Messenger, the voter's
 * Get Started) to the account's callback URL. The full URL comes from the
 * backend, which knows the public address of the webhook component.
 */
export const hasWebhook = (provider: EMessagingProvider): boolean =>
    isMetaProvider(provider) ||
    provider === EMessagingProvider.VIBER_INFOBIP ||
    provider === EMessagingProvider.AWS_SES

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
    identity: accountIdentity(account.sender),
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
    election_channels: {},
})

/**
 * How a channel sends notices outside its conversation window. Messenger
 * always uses the approved utility template: there is no "do not send", and a
 * notice never moves to another channel. Other channels have no such choice.
 */
export const outOfWindowPolicy = (channel: EMessageChannel): EOutOfWindowPolicy =>
    channel === EMessageChannel.MESSENGER
        ? EOutOfWindowPolicy.UTILITY_MESSAGES
        : EOutOfWindowPolicy.DISABLED

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const asArray = <T>(value: unknown): T[] => (Array.isArray(value) ? (value as T[]) : [])

const asRecord = <T>(value: unknown): Record<string, T> =>
    isRecord(value) ? (value as Record<string, T>) : {}

/**
 * Reads a stored configuration, filling the defaults serde applies. The
 * removed `notice_fallback` and `reply_text` are dropped, and each channel
 * gets its out-of-window policy.
 */
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
                out_of_window: outOfWindowPolicy(channel.channel as EMessageChannel),
            })
        )
    return {
        version: typeof value.version === "number" ? value.version : EVENT_MESSAGING_CONFIG_VERSION,
        channels,
        election_channels: asRecord<EMessageChannel[]>(value.election_channels),
    }
}

/** Channels enabled for `purpose`, optionally restricted to an election, in channel order. */
export const enabledChannels = (
    config: IEventMessagingConfig,
    purpose: EMessagePurpose,
    electionId?: string | null
): EMessageChannel[] => {
    const restriction = electionId ? config.election_channels[electionId] : undefined
    const enabled = config.channels
        .filter((channel) => channel.purposes.includes(purpose))
        .map((channel) => channel.channel)
    return MESSAGE_CHANNELS.filter(
        (channel) => enabled.includes(channel) && (!restriction || restriction.includes(channel))
    )
}

/** Whether a notice template is bound for `channel`: Messenger's utility template. */
export const noticeTemplateBound = (entry: IEventChannelConfig): boolean =>
    entry.templates.some(
        (binding) =>
            binding.purpose === EMessagePurpose.NOTICE && !!binding.provider_template.trim()
    )

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
                account.capabilities,
                account.provider_approval,
                account.check,
                purpose,
                null,
                account.identity
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
            if (!templatePurposes.includes(binding.purpose)) {
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
        } else if (
            channelConfig.out_of_window === EOutOfWindowPolicy.UTILITY_MESSAGES &&
            channelConfig.purposes.includes(EMessagePurpose.NOTICE) &&
            !noticeTemplateBound(channelConfig)
        ) {
            // Outside the window a notice is sent only with the utility template.
            errors.push({kind: "UTILITY_TEMPLATE_MISSING", channel})
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

/**
 * Message counts of a channel, per message (its latest attempt). `delivered`
 * is null when the account gets no receipts: unavailable, never 0.
 */
export interface IDeliverySummary {
    voters: number | null
    sentLast24h: number | null
    queued: number
    accepted: number
    delivered: number | null
    failed: number
    unknown: number
    unresolved: number
}

export const deliverySummary = (
    stats: IChannelMessageStats | undefined,
    feedback: EDeliveryFeedback
): IDeliverySummary => ({
    voters: stats?.voters ?? null,
    sentLast24h: stats?.sent_last_24h ?? null,
    queued: stats?.queued ?? 0,
    accepted: stats?.accepted ?? 0,
    delivered: feedback === EDeliveryFeedback.UNAVAILABLE ? null : (stats?.delivered ?? 0),
    failed: stats?.failed ?? 0,
    unknown: stats?.unknown ?? 0,
    unresolved: stats?.unresolved ?? 0,
})

/**
 * Whether the event tab can show deliveries for an account, and why not:
 * without receipts from the provider (`UNAVAILABLE` alone), or because
 * callbacks can't reach the account (`reason`: the backend's reason, such as
 * "Callback address not configured"). Either way delivery reads
 * "Unavailable", never 0.
 */
export const accountDeliveryFeedback = (
    account: IMessagingAccount | undefined
): {feedback: EDeliveryFeedback; reason?: string} => {
    const feedback = account
        ? (accountCapabilities(account)?.delivery_feedback ?? EDeliveryFeedback.UNAVAILABLE)
        : EDeliveryFeedback.UNAVAILABLE
    const blocked = account?.status?.callback_blockers ?? []
    if (feedback === EDeliveryFeedback.PROVIDER_RECEIPTS && blocked.length) {
        // Reports can't arrive: the backend's reason, or the blocker itself.
        return {
            feedback: EDeliveryFeedback.UNAVAILABLE,
            reason: account?.status?.callback_reasons?.[0] ?? String(blocked[0]),
        }
    }
    return {feedback}
}

/** Whether `account` can no longer send because its provider was removed. */
export const accountUnsupported = (account: Pick<IMessagingAccount, "provider">): boolean =>
    isUnsupportedProvider(account.provider)
