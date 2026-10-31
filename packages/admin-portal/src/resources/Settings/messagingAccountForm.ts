// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    ECredentialName,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EProviderApproval,
    IAccountLimits,
    IAccountSender,
    IMessagingAccount,
    IPurposeReadiness,
    IViberApprovedTemplates,
    MESSAGE_PURPOSES,
} from "@/types/messaging"
import {
    accountCapabilities,
    accountIdentity,
    checkedForAnotherIdentity,
    isUnsupportedProvider,
    providerCapabilities,
    providersForChannel,
    purposeReadiness,
    requiredCredentials,
    typedCredentials,
} from "@/services/messaging"

export const DEFAULT_META_API_VERSION = "v23.0"

export interface ISenderField {
    key: string
    required: boolean
}

const field = (key: string, required = false): ISenderField => ({key, required})

/** Each provider's non-secret identifiers, in the order the drawer shows them. */
export const senderFields = (provider: EMessagingProvider): ISenderField[] => {
    switch (provider) {
        case EMessagingProvider.AWS_SES:
            return [
                field("from_address", true),
                field("from_name"),
                field("region"),
                field("notification_topic_arn"),
            ]
        case EMessagingProvider.SMTP:
            return [field("from_address", true), field("from_name"), field("server_url", true)]
        case EMessagingProvider.AWS_SNS:
            return [field("sender_id"), field("origination_number"), field("region")]
        case EMessagingProvider.WHATSAPP_CLOUD_API:
            return [
                field("business_account_id", true),
                field("phone_number_id", true),
                field("display_phone_number", true),
                field("display_name"),
                field("api_version", true),
                field("api_base_url"),
            ]
        case EMessagingProvider.MESSENGER_SEND_API:
            return [
                field("page_id", true),
                field("page_name"),
                field("page_username"),
                field("api_version", true),
                field("api_base_url"),
            ]
        case EMessagingProvider.VIBER_INFOBIP:
            return [field("base_url", true), field("sender", true)]
        case EMessagingProvider.HTTP_API:
        case EMessagingProvider.CONSOLE:
            // Unsupported: nothing to edit; the account is shown to be replaced.
            return []
    }
}

/** Providers an administrator can pick for a new account of a channel: its own adapters. */
export const selectableProviders = (channel: EMessageChannel): EMessagingProvider[] =>
    providersForChannel(channel)

export interface IViberTemplateRow {
    purpose: EMessagePurpose
    language: string
    templateId: string
}

export interface IAccountFormLimits {
    messagesPerSecond: string
    otpReservedPerSecond: string
    otpConcurrency: string
    allowedCallingCodes: string
}

export interface IAccountFormValues {
    channel: EMessageChannel
    provider: EMessagingProvider
    name: string
    sender: Record<string, string>
    viberTemplates: IViberTemplateRow[]
    limits: IAccountFormLimits
    providerApproval: EProviderApproval
    isDefault: boolean
    credentials: Partial<Record<ECredentialName, string>>
}

const initialSender = (provider: EMessagingProvider): Record<string, string> =>
    Object.fromEntries(
        senderFields(provider).map(({key}) => [
            key,
            key === "api_version" ? DEFAULT_META_API_VERSION : "",
        ])
    )

export const emptyAccountForm = (
    channel: EMessageChannel,
    provider: EMessagingProvider
): IAccountFormValues => ({
    channel,
    provider,
    name: "",
    sender: initialSender(provider),
    viberTemplates: [],
    limits: {
        messagesPerSecond: "",
        otpReservedPerSecond: "",
        otpConcurrency: "",
        allowedCallingCodes: "",
    },
    providerApproval: EProviderApproval.PENDING,
    isDefault: false,
    credentials: {},
})

/** Switches the form to another provider, keeping what does not depend on it. */
export const withProvider = (
    values: IAccountFormValues,
    provider: EMessagingProvider
): IAccountFormValues => ({
    ...values,
    provider,
    sender: initialSender(provider),
    viberTemplates: [],
    credentials: {},
})

/**
 * Switches a new account to another channel. A provider of that channel is
 * kept with what was entered; otherwise the channel's first provider is
 * selected.
 */
export const withChannel = (
    values: IAccountFormValues,
    channel: EMessageChannel
): IAccountFormValues =>
    providerCapabilities(values.provider, channel)
        ? {...values, channel}
        : withProvider({...values, channel}, selectableProviders(channel)[0])

const senderValues = (sender: IAccountSender): Record<string, string> => {
    const values: Record<string, string> = {}
    const record = sender as unknown as Record<string, unknown>
    for (const {key} of senderFields(sender.provider)) {
        const value = record[key]
        values[key] = typeof value === "string" ? value : ""
    }
    return values
}

const viberRows = (templates: IViberApprovedTemplates | undefined): IViberTemplateRow[] =>
    MESSAGE_PURPOSES.flatMap((purpose) =>
        Object.entries(templates?.[purpose] ?? {}).map(([language, templateId]) => ({
            purpose,
            language,
            templateId,
        }))
    )

const countText = (value: number | null | undefined): string =>
    value === null || value === undefined ? "" : String(value)

export const formFromAccount = (account: IMessagingAccount): IAccountFormValues => ({
    channel: account.channel,
    provider: account.provider,
    name: account.name,
    sender: senderValues(account.sender),
    viberTemplates:
        account.sender.provider === EMessagingProvider.VIBER_INFOBIP
            ? viberRows(account.sender.approved_templates)
            : [],
    limits: {
        messagesPerSecond: countText(account.limits?.messages_per_second),
        otpReservedPerSecond: countText(account.limits?.otp_reserved_per_second),
        otpConcurrency: countText(account.limits?.otp_concurrency),
        allowedCallingCodes: (account.limits?.allowed_calling_codes ?? []).join(", "),
    },
    providerApproval: account.provider_approval ?? EProviderApproval.PENDING,
    isDefault: account.is_default,
    credentials: {},
})

const CALLING_CODE = /^\+?(\d{1,3})$/

/** Country calling codes, as digits; empty means any destination. */
export const parseCallingCodes = (text: string): {codes: string[]; invalid: string[]} => {
    const codes: string[] = []
    const invalid: string[] = []
    for (const entry of text.split(/[\s,;]+/).filter(Boolean)) {
        const match = CALLING_CODE.exec(entry)
        if (!match) {
            invalid.push(entry)
        } else if (!codes.includes(match[1])) {
            codes.push(match[1])
        }
    }
    return {codes, invalid}
}

const required = (values: IAccountFormValues, key: string): string =>
    (values.sender[key] ?? "").trim()

const optional = (values: IAccountFormValues, key: string): string | null =>
    required(values, key) || null

const viberTemplates = (rows: IViberTemplateRow[]): IViberApprovedTemplates => {
    const templates: IViberApprovedTemplates = {}
    for (const row of rows) {
        const language = row.language.trim()
        const templateId = row.templateId.trim()
        if (!language || !templateId) {
            continue
        }
        templates[row.purpose] = {...(templates[row.purpose] ?? {}), [language]: templateId}
    }
    return templates
}

export const senderFromForm = (values: IAccountFormValues): IAccountSender => {
    switch (values.provider) {
        case EMessagingProvider.AWS_SES:
            return {
                provider: EMessagingProvider.AWS_SES,
                from_address: required(values, "from_address"),
                from_name: optional(values, "from_name"),
                region: optional(values, "region"),
                notification_topic_arn: optional(values, "notification_topic_arn"),
            }
        case EMessagingProvider.SMTP:
            return {
                provider: EMessagingProvider.SMTP,
                from_address: required(values, "from_address"),
                from_name: optional(values, "from_name"),
                server_url: required(values, "server_url"),
            }
        case EMessagingProvider.AWS_SNS:
            return {
                provider: EMessagingProvider.AWS_SNS,
                sender_id: optional(values, "sender_id"),
                origination_number: optional(values, "origination_number"),
                region: optional(values, "region"),
            }
        case EMessagingProvider.WHATSAPP_CLOUD_API:
            return {
                provider: EMessagingProvider.WHATSAPP_CLOUD_API,
                business_account_id: required(values, "business_account_id"),
                phone_number_id: required(values, "phone_number_id"),
                display_phone_number: required(values, "display_phone_number"),
                display_name: optional(values, "display_name"),
                api_version: required(values, "api_version"),
                api_base_url: optional(values, "api_base_url"),
            }
        case EMessagingProvider.MESSENGER_SEND_API:
            return {
                provider: EMessagingProvider.MESSENGER_SEND_API,
                page_id: required(values, "page_id"),
                page_name: optional(values, "page_name"),
                page_username: optional(values, "page_username"),
                api_version: required(values, "api_version"),
                api_base_url: optional(values, "api_base_url"),
            }
        case EMessagingProvider.VIBER_INFOBIP:
            return {
                provider: EMessagingProvider.VIBER_INFOBIP,
                base_url: required(values, "base_url"),
                sender: required(values, "sender"),
                approved_templates: viberTemplates(values.viberTemplates),
            }
        case EMessagingProvider.HTTP_API:
            return {provider: EMessagingProvider.HTTP_API}
        case EMessagingProvider.CONSOLE:
            return {provider: EMessagingProvider.CONSOLE}
    }
}

const COUNT = /^\d+$/

const count = (text: string): number | null => (COUNT.test(text.trim()) ? Number(text) : null)

export const limitsFromForm = (values: IAccountFormValues): IAccountLimits => ({
    messages_per_second: count(values.limits.messagesPerSecond),
    otp_reserved_per_second: count(values.limits.otpReservedPerSecond),
    otp_concurrency: count(values.limits.otpConcurrency),
    allowed_calling_codes: parseCallingCodes(values.limits.allowedCallingCodes).codes,
})

export enum EAccountFormError {
    REQUIRED = "REQUIRED",
    NOT_A_COUNT = "NOT_A_COUNT",
    OTP_ABOVE_TOTAL = "OTP_ABOVE_TOTAL",
    /** Codes sent at once: 0 would never send a code. */
    AT_LEAST_ONE = "AT_LEAST_ONE",
    INVALID_CALLING_CODE = "INVALID_CALLING_CODE",
    DUPLICATE_LANGUAGE = "DUPLICATE_LANGUAGE",
    NOT_A_URL = "NOT_A_URL",
    /** A removed provider: the account can't be saved, only replaced. */
    UNSUPPORTED_PROVIDER = "UNSUPPORTED_PROVIDER",
    /**
     * The address the credentials are sent to changed, and the credentials
     * weren't entered again. The backend refuses it with the same reason.
     */
    CREDENTIALS_FOLLOW_ENDPOINT = "CREDENTIALS_FOLLOW_ENDPOINT",
}

/**
 * The sender field holding the address an account's credentials are sent to,
 * like `account_endpoint` in windmill: the Graph API base, the Infobip base or
 * the SMTP server. `null` for providers that name none.
 */
export const endpointField = (provider: EMessagingProvider): string | null => {
    switch (provider) {
        case EMessagingProvider.WHATSAPP_CLOUD_API:
        case EMessagingProvider.MESSENGER_SEND_API:
            return "api_base_url"
        case EMessagingProvider.VIBER_INFOBIP:
            return "base_url"
        case EMessagingProvider.SMTP:
            return "server_url"
        default:
            return null
    }
}

const endpointOf = (sender: Record<string, string>, key: string): string | null =>
    (sender[key] ?? "").trim().replace(/\/+$/, "") || null

/** Whether the form moves the account's credentials to another address. */
export const endpointChanged = (
    original: IAccountFormValues | undefined,
    values: IAccountFormValues
): boolean => {
    const key = endpointField(values.provider)
    return (
        !!original &&
        !!key &&
        original.provider === values.provider &&
        endpointOf(original.sender, key) !== endpointOf(values.sender, key)
    )
}

/**
 * Credentials that must be entered again before the form can be saved: all
 * the provider needs to send, when the address they go to changed (the same
 * rule as `endpoint_change_refusal`). Empty otherwise.
 */
export const credentialsToReenter = (
    original: IAccountFormValues | undefined,
    values: IAccountFormValues
): ECredentialName[] =>
    endpointChanged(original, values)
        ? requiredCredentials(values.provider).filter((name) => !values.credentials[name])
        : []

// Provider endpoints carry the stored credentials, so they must be https.
const HTTPS_URL = /^https:\/\/\S+$/

/** Errors by field path; empty when the form can be saved. */
export const validateAccountForm = (
    values: IAccountFormValues,
    /** The stored account, when editing one. */
    original?: IAccountFormValues
): Record<string, EAccountFormError> => {
    const errors: Record<string, EAccountFormError> = {}
    if (isUnsupportedProvider(values.provider)) {
        errors.provider = EAccountFormError.UNSUPPORTED_PROVIDER
    }
    if (!values.name.trim()) {
        errors.name = EAccountFormError.REQUIRED
    }
    for (const {key, required: isRequired} of senderFields(values.provider)) {
        if (isRequired && !required(values, key)) {
            errors[`sender.${key}`] = EAccountFormError.REQUIRED
        }
    }
    const {messagesPerSecond, otpReservedPerSecond, otpConcurrency, allowedCallingCodes} =
        values.limits
    for (const [key, text] of [
        ["messagesPerSecond", messagesPerSecond],
        ["otpReservedPerSecond", otpReservedPerSecond],
        ["otpConcurrency", otpConcurrency],
    ]) {
        if (text.trim() && count(text) === null) {
            errors[`limits.${key}`] = EAccountFormError.NOT_A_COUNT
        }
    }
    const total = count(messagesPerSecond)
    const reserved = count(otpReservedPerSecond)
    if (total !== null && reserved !== null && reserved > total) {
        errors["limits.otpReservedPerSecond"] = EAccountFormError.OTP_ABOVE_TOTAL
    }
    if (count(otpConcurrency) === 0) {
        errors["limits.otpConcurrency"] = EAccountFormError.AT_LEAST_ONE
    }
    if (parseCallingCodes(allowedCallingCodes).invalid.length) {
        errors["limits.allowedCallingCodes"] = EAccountFormError.INVALID_CALLING_CODE
    }
    if (values.provider === EMessagingProvider.VIBER_INFOBIP) {
        const seen = new Set<string>()
        values.viberTemplates.forEach((row, index) => {
            const language = row.language.trim()
            const key = `${row.purpose}:${language}`
            if (!language || !row.templateId.trim()) {
                errors[`viberTemplates.${index}`] = EAccountFormError.REQUIRED
            } else if (seen.has(key)) {
                errors[`viberTemplates.${index}`] = EAccountFormError.DUPLICATE_LANGUAGE
            }
            seen.add(key)
        })
    }
    for (const key of ["api_base_url", "base_url"]) {
        const url = optional(values, key)
        if (
            senderFields(values.provider).some((candidate) => candidate.key === key) &&
            url &&
            !HTTPS_URL.test(url)
        ) {
            errors[`sender.${key}`] = EAccountFormError.NOT_A_URL
        }
    }
    const endpoint = endpointField(values.provider)
    if (
        endpoint &&
        !errors[`sender.${endpoint}`] &&
        credentialsToReenter(original, values).length
    ) {
        errors[`sender.${endpoint}`] = EAccountFormError.CREDENTIALS_FOLLOW_ENDPOINT
    }
    return errors
}

export interface IUpsertAccountVariables {
    id: string | null
    channel: EMessageChannel
    name: string
    sender: IAccountSender
    limits: IAccountLimits
    providerApproval: EProviderApproval | null
    isDefault: boolean
    /**
     * Credentials typed in the form, stored in the same transaction as the
     * account, so an address change and its credentials are saved together.
     */
    credentials: Partial<Record<ECredentialName, string>> | null
}

/** The variables of `upsert_messaging_account` for the form. */
export const upsertAccountVariables = (
    id: string | null,
    values: IAccountFormValues
): IUpsertAccountVariables => ({
    id,
    channel: values.channel,
    name: values.name.trim(),
    sender: senderFromForm(values),
    limits: limitsFromForm(values),
    providerApproval: requiresProviderApproval(values.provider, values.channel)
        ? values.providerApproval
        : null,
    isDefault: values.isDefault,
    credentials: credentialsPayload(values),
})

/** Typed credentials to replace, or null when none was typed. */
export const credentialsPayload = (
    values: IAccountFormValues
): Partial<Record<ECredentialName, string>> | null => {
    const entries = typedCredentials(values.provider)
        .map((name) => [name, values.credentials[name] ?? ""] as const)
        .filter(([, value]) => value.length > 0)
    return entries.length ? Object.fromEntries(entries) : null
}

export interface IAccountReadiness {
    /** What the last check told. */
    connected: boolean
    purposes: Record<EMessagePurpose, IPurposeReadiness>
}

/** Readiness always comes from the provider's check. */
export const accountReadiness = (account: IMessagingAccount): IAccountReadiness => {
    const capabilities = accountCapabilities(account)
    const readiness = (purpose: EMessagePurpose) =>
        purposeReadiness(
            account.provider,
            capabilities,
            account.provider_approval,
            account.status,
            purpose,
            null,
            accountIdentity(account.sender)
        )
    return {
        // A check of another identity (an earlier number or Page) says
        // nothing about the account as it is now.
        connected:
            (account.status?.connected ?? false) &&
            !isUnsupportedProvider(account.provider) &&
            !checkedForAnotherIdentity(account),
        purposes: {
            [EMessagePurpose.OTP]: readiness(EMessagePurpose.OTP),
            [EMessagePurpose.NOTICE]: readiness(EMessagePurpose.NOTICE),
        },
    }
}

export const requiresProviderApproval = (
    provider: EMessagingProvider,
    channel: EMessageChannel
): boolean => providerCapabilities(provider, channel)?.requires_provider_approval ?? false
