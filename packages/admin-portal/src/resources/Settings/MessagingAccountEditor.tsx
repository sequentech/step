// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useMutation, useQuery} from "@apollo/client"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    FormControlLabel,
    IconButton,
    InputAdornment,
    MenuItem,
    Switch,
    TextField,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline"
import {ChannelLabel} from "@sequentech/ui-essentials"
import {IPermissions} from "@/types/keycloak"
import {
    ECredentialName,
    EMessageChannel,
    EMessagePurpose,
    EMessagingProvider,
    EProviderApproval,
    IMessagingAccount,
    MESSAGE_CHANNELS,
    MESSAGE_PURPOSES,
} from "@/types/messaging"
import {
    hasWebhook,
    isMetaProvider,
    isUnsupportedProvider,
    typedCredentials,
} from "@/services/messaging"
import {reasonText} from "@/services/messagingReasons"
import {UPSERT_MESSAGING_ACCOUNT} from "@/queries/UpsertMessagingAccount"
import {REPLACE_MESSAGING_ACCOUNT_CREDENTIALS} from "@/queries/ReplaceMessagingAccountCredentials"
import {
    IMessagingAccountCallbackUrlResult,
    MESSAGING_ACCOUNT_CALLBACK_URL,
} from "@/queries/MessagingAccountCallbackUrl"
import {
    EAccountFormError,
    IAccountFormValues,
    credentialsPayload,
    emptyAccountForm,
    formFromAccount,
    requiresProviderApproval,
    selectableProviders,
    senderFields,
    upsertAccountVariables,
    validateAccountForm,
    withChannel,
    withProvider,
} from "./messagingAccountForm"

export const MESSAGING_ACCOUNT_EDITOR_TITLE = "messaging-account-editor-title"

const WRITE_ROLE = {headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE}}

/** The account's full callback URL at the webhook component, or why there is none. */
export interface ICallbackResult {
    callback_url?: string | null
    reason?: string | null
}

interface IUpsertMessagingAccountResult {
    upsert_messaging_account: {
        id: string
        callback_url?: string | null
        callback_reason?: string | null
    } | null
}

interface IReplaceCredentialsResult {
    replace_messaging_account_credentials: {
        replaced: string[]
        verify_token: string | null
        report_username?: string | null
        report_password?: string | null
    } | null
}

const FIELDS_WITH_HELP = new Set([
    "from_address",
    "notification_topic_arn",
    "sender_id",
    "origination_number",
    "phone_number_id",
    "display_name",
    "page_username",
    "api_version",
    "api_base_url",
    "base_url",
    "sender",
])

export const formatDate = (iso: string | null | undefined, language: string): string =>
    iso ? new Date(iso).toLocaleString(language) : ""

const useCopy = () => {
    const {t} = useTranslation()
    const notify = useNotify()
    return async (text: string) => {
        try {
            await navigator.clipboard.writeText(text)
            notify(t("messagingAccounts.copy.success"), {type: "info"})
        } catch {
            notify(t("messagingAccounts.copy.error"), {type: "error"})
        }
    }
}

interface ICredentialInputProps {
    name: ECredentialName
    provider: EMessagingProvider
    replacedAt: string | undefined
    value: string
    disabled: boolean
    onChange: (value: string) => void
}

/** A write-only secret: once set, only the time it was last replaced is shown. */
export const CredentialInput: React.FC<ICredentialInputProps> = ({
    name,
    provider,
    replacedAt,
    value,
    disabled,
    onChange,
}) => {
    const {t, i18n} = useTranslation()
    const [replacing, setReplacing] = useState(false)
    const label = t(`messaging.credential.${name}`)
    if (replacedAt && !replacing) {
        return (
            <Box
                sx={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: 2,
                }}
            >
                <Box>
                    <Typography variant="body2" sx={{fontWeight: 500}}>
                        {label}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                        {t("messagingAccounts.credentials.set", {
                            date: formatDate(replacedAt, i18n.language),
                        })}
                    </Typography>
                </Box>
                {!disabled && (
                    <Button
                        variant="outlined"
                        onClick={() => setReplacing(true)}
                        aria-label={t("messagingAccounts.credentials.replaceNamed", {name: label})}
                    >
                        {t("messagingAccounts.credentials.replace")}
                    </Button>
                )}
            </Box>
        )
    }
    return (
        <TextField
            type="password"
            label={label}
            value={value}
            disabled={disabled}
            autoComplete="new-password"
            helperText={t(`messagingAccounts.credentialHelp.${provider}.${name}`)}
            onChange={(event) => onChange(event.target.value)}
            fullWidth
        />
    )
}

interface ICopyFieldProps {
    label: string
    value: string
    copyLabel: string
    helperText?: string
}

/** A read-only value with a copy button. */
const CopyField: React.FC<ICopyFieldProps> = ({label, value, copyLabel, helperText}) => {
    const copy = useCopy()
    return (
        <TextField
            value={value}
            label={label}
            helperText={helperText}
            fullWidth
            slotProps={{
                input: {
                    readOnly: true,
                    endAdornment: (
                        <InputAdornment position="end">
                            <IconButton aria-label={copyLabel} onClick={() => copy(value)}>
                                <ContentCopyIcon />
                            </IconButton>
                        </InputAdornment>
                    ),
                },
            }}
        />
    )
}

interface IVerifyTokenDialogProps {
    token: string
    onClose: () => void
}

/** Shows a generated verify token once; closing it forgets the token. */
export const VerifyTokenDialog: React.FC<IVerifyTokenDialogProps> = ({token, onClose}) => {
    const {t} = useTranslation()
    return (
        <Dialog open onClose={onClose} aria-labelledby="verify-token-title">
            <DialogTitle id="verify-token-title">
                {t("messagingAccounts.webhook.tokenTitle")}
            </DialogTitle>
            <DialogContent sx={{display: "flex", flexDirection: "column", gap: 2}}>
                <Typography variant="body2">{t("messagingAccounts.webhook.tokenOnce")}</Typography>
                <CopyField
                    value={token}
                    label={t("messaging.credential.VERIFY_TOKEN")}
                    copyLabel={t("messagingAccounts.webhook.copyToken")}
                />
            </DialogContent>
            <DialogActions>
                <Button variant="contained" onClick={onClose}>
                    {t("messagingAccounts.webhook.tokenDone")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}

export interface IReportCredentials {
    username: string
    password: string
}

/** The steps that subscribe Infobip's Viber reports to the account's callback URL. */
const ViberSubscriptionSteps: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Box component="ol" sx={{margin: 0, paddingLeft: 3}}>
            {(["subscription", "url", "auth"] as const).map((step) => (
                <Typography key={step} component="li" variant="body2">
                    {t(`messagingAccounts.reports.step.${step}`)}
                </Typography>
            ))}
        </Box>
    )
}

interface IReportCredentialsDialogProps {
    credentials: IReportCredentials
    callbackUrl: string | null
    onClose: () => void
}

/**
 * Shows the generated report user name and password once, with the callback
 * URL and the subscription steps; closing it forgets them.
 */
export const ReportCredentialsDialog: React.FC<IReportCredentialsDialogProps> = ({
    credentials,
    callbackUrl,
    onClose,
}) => {
    const {t} = useTranslation()
    return (
        <Dialog open onClose={onClose} aria-labelledby="report-credentials-title">
            <DialogTitle id="report-credentials-title">
                {t("messagingAccounts.reports.title")}
            </DialogTitle>
            <DialogContent sx={{display: "flex", flexDirection: "column", gap: 2}}>
                <Typography variant="body2">{t("messagingAccounts.reports.once")}</Typography>
                <ViberSubscriptionSteps />
                {callbackUrl ? (
                    <CopyField
                        value={callbackUrl}
                        label={t("messagingAccounts.webhook.url")}
                        copyLabel={t("messagingAccounts.webhook.copyUrl")}
                    />
                ) : null}
                <CopyField
                    value={credentials.username}
                    label={t("messaging.credential.REPORT_USERNAME")}
                    copyLabel={t("messagingAccounts.reports.copyUsername")}
                />
                <CopyField
                    value={credentials.password}
                    label={t("messaging.credential.REPORT_PASSWORD")}
                    copyLabel={t("messagingAccounts.reports.copyPassword")}
                />
            </DialogContent>
            <DialogActions>
                <Button variant="contained" onClick={onClose}>
                    {t("messagingAccounts.webhook.tokenDone")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}

/** What the provider sends to the callback, per provider. */
const callbackDescriptionKey = (provider: EMessagingProvider): string => {
    switch (provider) {
        case EMessagingProvider.WHATSAPP_CLOUD_API:
            return "messagingAccounts.webhook.description.WHATSAPP_CLOUD_API"
        case EMessagingProvider.MESSENGER_SEND_API:
            return "messagingAccounts.webhook.description.MESSENGER_SEND_API"
        case EMessagingProvider.VIBER_INFOBIP:
            return "messagingAccounts.webhook.description.VIBER_INFOBIP"
        default:
            return "messagingAccounts.webhook.description.AWS_SES"
    }
}

interface ICallbackSectionProps {
    account: IMessagingAccount | undefined
    provider: EMessagingProvider
    canWrite: boolean
    /** The callback the save that created the account returned. */
    initialCallback?: ICallbackResult | null
    onChanged: () => void
}

/**
 * The account's callback URL at the public webhook component, for delivery
 * reports (and Messenger's Get Started), with Meta's verify token or Viber's
 * report credentials. The URL and the reason when there is none come from
 * the backend.
 */
export const CallbackSection: React.FC<ICallbackSectionProps> = ({
    account,
    provider,
    canWrite,
    initialCallback,
    onChanged,
}) => {
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const [token, setToken] = useState<string | null>(null)
    const [reportCredentials, setReportCredentials] = useState<IReportCredentials | null>(null)
    const [generating, setGenerating] = useState(false)
    const [replaceCredentials] = useMutation<IReplaceCredentialsResult>(
        REPLACE_MESSAGING_ACCOUNT_CREDENTIALS,
        {context: WRITE_ROLE}
    )
    const {data, loading} = useQuery<IMessagingAccountCallbackUrlResult>(
        MESSAGING_ACCOUNT_CALLBACK_URL,
        {
            variables: {accountId: account?.id},
            skip: !account || !canWrite,
            context: WRITE_ROLE,
            fetchPolicy: "cache-and-network",
        }
    )
    const callback: ICallbackResult | null =
        data?.messaging_account_callback_url ?? initialCallback ?? null
    const callbackUrl = callback?.callback_url ?? null
    const verifyTokenSet = account?.credentials?.[ECredentialName.VERIFY_TOKEN]?.replaced_at
    const reportSet = account?.credentials?.[ECredentialName.REPORT_PASSWORD]?.replaced_at
    const isViber = provider === EMessagingProvider.VIBER_INFOBIP

    const generate = async (kind: "VERIFY_TOKEN" | "REPORT_CREDENTIALS") => {
        if (!account) {
            return
        }
        setGenerating(true)
        try {
            const {data: generated} = await replaceCredentials({
                variables: {
                    id: account.id,
                    credentials: {},
                    generateVerifyToken: kind === "VERIFY_TOKEN",
                    generateReportCredentials: kind === "REPORT_CREDENTIALS",
                },
            })
            const result = generated?.replace_messaging_account_credentials
            if (kind === "VERIFY_TOKEN") {
                if (!result?.verify_token) {
                    throw new Error("No verify token returned")
                }
                setToken(result.verify_token)
            } else {
                if (!result?.report_username || !result?.report_password) {
                    throw new Error("No report credentials returned")
                }
                setReportCredentials({
                    username: result.report_username,
                    password: result.report_password,
                })
            }
            onChanged()
        } catch {
            notify(
                t(
                    kind === "VERIFY_TOKEN"
                        ? "messagingAccounts.webhook.tokenError"
                        : "messagingAccounts.reports.error"
                ),
                {type: "error"}
            )
        } finally {
            setGenerating(false)
        }
    }

    const urlText = !account
        ? t("messagingAccounts.webhook.afterSaving")
        : !canWrite
          ? t("messagingAccounts.webhook.needsWrite")
          : loading && !callback
            ? t("messagingAccounts.webhook.loading")
            : (callbackUrl ?? reasonText(t, callback?.reason))

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Typography variant="subtitle2" component="h3">
                {t("messagingAccounts.webhook.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t(callbackDescriptionKey(provider))}
            </Typography>
            {callbackUrl ? (
                <CopyField
                    value={callbackUrl}
                    label={t("messagingAccounts.webhook.url")}
                    copyLabel={t("messagingAccounts.webhook.copyUrl")}
                />
            ) : (
                <TextField
                    label={t("messagingAccounts.webhook.url")}
                    value={urlText || t("messagingAccounts.webhook.unavailable")}
                    fullWidth
                    slotProps={{input: {readOnly: true}}}
                />
            )}
            {isMetaProvider(provider) && (
                <Box
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        gap: 2,
                    }}
                >
                    <Box>
                        <Typography variant="body2" sx={{fontWeight: 500}}>
                            {t("messaging.credential.VERIFY_TOKEN")}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                            {verifyTokenSet
                                ? t("messagingAccounts.webhook.tokenSet", {
                                      date: formatDate(verifyTokenSet, i18n.language),
                                  })
                                : account
                                  ? t("messagingAccounts.webhook.tokenMissing")
                                  : t("messagingAccounts.webhook.tokenAfterSaving")}
                        </Typography>
                    </Box>
                    {canWrite && account && (
                        <Button
                            variant="outlined"
                            disabled={generating}
                            onClick={() => generate("VERIFY_TOKEN")}
                        >
                            {t("messagingAccounts.webhook.generate")}
                        </Button>
                    )}
                </Box>
            )}
            {isViber && (
                <>
                    <ViberSubscriptionSteps />
                    <Box
                        sx={{
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "space-between",
                            gap: 2,
                        }}
                    >
                        <Box>
                            <Typography variant="body2" sx={{fontWeight: 500}}>
                                {t("messagingAccounts.reports.label")}
                            </Typography>
                            <Typography variant="caption" color="text.secondary">
                                {reportSet
                                    ? t("messagingAccounts.webhook.tokenSet", {
                                          date: formatDate(reportSet, i18n.language),
                                      })
                                    : account
                                      ? t("messagingAccounts.reports.missing")
                                      : t("messagingAccounts.webhook.tokenAfterSaving")}
                            </Typography>
                        </Box>
                        {canWrite && account && (
                            <Button
                                variant="outlined"
                                disabled={generating}
                                onClick={() => generate("REPORT_CREDENTIALS")}
                            >
                                {t("messagingAccounts.reports.generate")}
                            </Button>
                        )}
                    </Box>
                </>
            )}
            {token && <VerifyTokenDialog token={token} onClose={() => setToken(null)} />}
            {reportCredentials && (
                <ReportCredentialsDialog
                    credentials={reportCredentials}
                    callbackUrl={callbackUrl}
                    onClose={() => setReportCredentials(null)}
                />
            )}
        </Box>
    )
}

interface IViberTemplatesEditorProps {
    values: IAccountFormValues
    errors: Record<string, EAccountFormError>
    disabled: boolean
    onChange: (values: IAccountFormValues) => void
}

/** The partner's approved templates, entered by hand: language and template ID per purpose. */
export const ViberTemplatesEditor: React.FC<IViberTemplatesEditorProps> = ({
    values,
    errors,
    disabled,
    onChange,
}) => {
    const {t} = useTranslation()
    const rows = values.viberTemplates
    const update = (index: number, patch: Partial<(typeof rows)[number]>) =>
        onChange({
            ...values,
            viberTemplates: rows.map((row, i) => (i === index ? {...row, ...patch} : row)),
        })
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Typography variant="subtitle2" component="h3">
                {t("messagingAccounts.viber.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t("messagingAccounts.viber.description")}
            </Typography>
            {rows.map((row, index) => {
                const error = errors[`viberTemplates.${index}`]
                return (
                    <Box key={index} sx={{display: "flex", gap: 1, alignItems: "flex-start"}}>
                        <TextField
                            select
                            label={t("messagingAccounts.viber.purpose")}
                            value={row.purpose}
                            disabled={disabled}
                            onChange={(event) =>
                                update(index, {purpose: event.target.value as EMessagePurpose})
                            }
                            sx={{minWidth: 140}}
                        >
                            {MESSAGE_PURPOSES.map((purpose) => (
                                <MenuItem key={purpose} value={purpose}>
                                    {t(`messaging.purpose.${purpose}`)}
                                </MenuItem>
                            ))}
                        </TextField>
                        <TextField
                            label={t("messagingAccounts.viber.language")}
                            value={row.language}
                            disabled={disabled}
                            error={!!error}
                            onChange={(event) => update(index, {language: event.target.value})}
                            sx={{width: 120}}
                        />
                        <TextField
                            label={t("messagingAccounts.viber.templateId")}
                            value={row.templateId}
                            disabled={disabled}
                            error={!!error}
                            helperText={error ? t(`messagingAccounts.error.${error}`) : undefined}
                            onChange={(event) => update(index, {templateId: event.target.value})}
                            sx={{flex: 1}}
                        />
                        {!disabled && (
                            <IconButton
                                aria-label={t("messagingAccounts.viber.remove")}
                                onClick={() =>
                                    onChange({
                                        ...values,
                                        viberTemplates: rows.filter((_, i) => i !== index),
                                    })
                                }
                            >
                                <DeleteOutlineIcon />
                            </IconButton>
                        )}
                    </Box>
                )
            })}
            {!disabled && (
                <Box>
                    <Button
                        startIcon={<AddIcon />}
                        onClick={() =>
                            onChange({
                                ...values,
                                viberTemplates: [
                                    ...rows,
                                    {purpose: EMessagePurpose.OTP, language: "", templateId: ""},
                                ],
                            })
                        }
                    >
                        {t("messagingAccounts.viber.add")}
                    </Button>
                </Box>
            )}
        </Box>
    )
}

export interface IMessagingAccountEditorProps {
    /** The account to edit; a new account when absent. */
    account?: IMessagingAccount
    canWrite: boolean
    /** The callback the save that created this account returned. */
    initialCallback?: ICallbackResult | null
    /** Called after anything was saved, so the list can be read again. */
    onChanged: () => void
    /**
     * Called instead of closing when a new account that receives callbacks was
     * created, with the callback the save returned, so it can be shown.
     */
    onCreated?: (id: string, callback: ICallbackResult | null) => void
    onClose: () => void
}

export const MessagingAccountEditor: React.FC<IMessagingAccountEditorProps> = ({
    account,
    canWrite,
    initialCallback,
    onChanged,
    onCreated,
    onClose,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const [values, setValues] = useState<IAccountFormValues>(() =>
        account
            ? formFromAccount(account)
            : emptyAccountForm(EMessageChannel.WHATSAPP, EMessagingProvider.WHATSAPP_CLOUD_API)
    )
    const [submitted, setSubmitted] = useState(false)
    const [saving, setSaving] = useState(false)
    const [upsertAccount] = useMutation<IUpsertMessagingAccountResult>(UPSERT_MESSAGING_ACCOUNT, {
        context: WRITE_ROLE,
    })
    const [replaceCredentials] = useMutation<IReplaceCredentialsResult>(
        REPLACE_MESSAGING_ACCOUNT_CREDENTIALS,
        {context: WRITE_ROLE}
    )
    const errors = submitted ? validateAccountForm(values) : {}
    // A removed provider's account can only be replaced: nothing is edited or saved.
    const unsupported = isUnsupportedProvider(values.provider)
    const disabled = !canWrite || saving || unsupported
    const channelName = t(`messaging.channel.${values.channel}`)
    const needsApproval = requiresProviderApproval(values.provider, values.channel)
    const original = account ? formFromAccount(account).sender : undefined
    const changed = (key: string) => !!original && (values.sender[key] ?? "") !== original[key]
    const pageChanged =
        values.provider === EMessagingProvider.MESSENGER_SEND_API && changed("page_id")
    const numberChanged =
        values.provider === EMessagingProvider.WHATSAPP_CLOUD_API &&
        (changed("phone_number_id") || changed("business_account_id"))

    const errorText = (path: string) =>
        errors[path] ? t(`messagingAccounts.error.${errors[path]}`) : undefined

    const setSender = (key: string, value: string) =>
        setValues({...values, sender: {...values.sender, [key]: value}})

    const setLimit = (key: keyof IAccountFormValues["limits"], value: string) =>
        setValues({...values, limits: {...values.limits, [key]: value}})

    const save = async () => {
        setSubmitted(true)
        if (Object.keys(validateAccountForm(values)).length) {
            return
        }
        setSaving(true)
        try {
            const {data} = await upsertAccount({
                variables: upsertAccountVariables(account?.id ?? null, values),
            })
            const saved = data?.upsert_messaging_account
            const id = saved?.id ?? account?.id
            if (!id) {
                throw new Error("No account id returned")
            }
            const credentials = credentialsPayload(values)
            if (credentials) {
                await replaceCredentials({variables: {id, credentials}})
            }
            notify(t("messagingAccounts.save.success"), {type: "success"})
            if (!account && hasWebhook(values.provider) && onCreated) {
                onCreated(id, {callback_url: saved?.callback_url, reason: saved?.callback_reason})
                return
            }
            onChanged()
            onClose()
        } catch {
            notify(t("messagingAccounts.save.error"), {type: "error"})
        } finally {
            setSaving(false)
        }
    }

    return (
        <Box sx={{padding: 3, display: "flex", flexDirection: "column", gap: 2}}>
            <Box>
                <Typography variant="h5" component="h2" id={MESSAGING_ACCOUNT_EDITOR_TITLE}>
                    {account
                        ? t("messagingAccounts.editor.editTitle", {channel: channelName})
                        : t("messagingAccounts.editor.addTitle")}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                    {t("messagingAccounts.editor.subtitle")}
                </Typography>
            </Box>

            {unsupported ? (
                <Alert severity="error">{t("messagingAccounts.unsupported")}</Alert>
            ) : null}

            {account ? (
                <Box sx={{display: "flex", alignItems: "center", gap: 2}}>
                    <ChannelLabel channel={values.channel} label={channelName} />
                    <Typography variant="body2" color="text.secondary">
                        {t(`messaging.provider.${values.provider}`)}
                    </Typography>
                </Box>
            ) : (
                <Box sx={{display: "flex", gap: 2}}>
                    <TextField
                        select
                        label={t("messagingAccounts.editor.channel")}
                        value={values.channel}
                        disabled={disabled}
                        helperText={t("messagingAccounts.editor.channelHelp")}
                        onChange={(event) =>
                            setValues(withChannel(values, event.target.value as EMessageChannel))
                        }
                        sx={{minWidth: 200, maxWidth: 260}}
                    >
                        {MESSAGE_CHANNELS.map((channel) => (
                            <MenuItem key={channel} value={channel}>
                                {t(`messaging.channel.${channel}`)}
                            </MenuItem>
                        ))}
                    </TextField>
                    <TextField
                        select
                        label={t("messagingAccounts.editor.provider")}
                        value={values.provider}
                        disabled={disabled}
                        onChange={(event) =>
                            setValues(
                                withProvider(values, event.target.value as EMessagingProvider)
                            )
                        }
                        sx={{flex: 1}}
                    >
                        {selectableProviders(values.channel).map((provider) => (
                            <MenuItem key={provider} value={provider}>
                                {t(`messaging.provider.${provider}`)}
                            </MenuItem>
                        ))}
                    </TextField>
                </Box>
            )}

            <TextField
                label={t("messagingAccounts.field.name")}
                value={values.name}
                disabled={disabled}
                required
                error={!!errors.name}
                helperText={errorText("name")}
                onChange={(event) => setValues({...values, name: event.target.value})}
                fullWidth
            />
            {senderFields(values.provider).map(({key, required}) => (
                <TextField
                    key={`${values.provider}-${key}`}
                    label={t(`messagingAccounts.field.${key}`)}
                    value={values.sender[key] ?? ""}
                    disabled={disabled}
                    required={required}
                    error={!!errors[`sender.${key}`]}
                    helperText={
                        errorText(`sender.${key}`) ??
                        (FIELDS_WITH_HELP.has(key)
                            ? t(`messagingAccounts.fieldHelp.${key}`)
                            : undefined)
                    }
                    onChange={(event) => setSender(key, event.target.value)}
                    fullWidth
                />
            ))}
            {pageChanged && (
                <Alert severity="warning">
                    {t("messagingAccounts.warning.pageChange", {
                        page: original?.page_name || original?.page_id,
                    })}
                </Alert>
            )}
            {numberChanged && (
                <Alert severity="warning">{t("messagingAccounts.warning.numberChange")}</Alert>
            )}

            {values.provider === EMessagingProvider.VIBER_INFOBIP && (
                <ViberTemplatesEditor
                    values={values}
                    errors={errors}
                    disabled={disabled}
                    onChange={setValues}
                />
            )}

            {needsApproval && (
                <TextField
                    select
                    label={t("messagingAccounts.field.provider_approval")}
                    value={values.providerApproval}
                    disabled={disabled}
                    helperText={t("messagingAccounts.fieldHelp.provider_approval")}
                    onChange={(event) =>
                        setValues({
                            ...values,
                            providerApproval: event.target.value as EProviderApproval,
                        })
                    }
                    fullWidth
                >
                    {Object.values(EProviderApproval).map((approval) => (
                        <MenuItem key={approval} value={approval}>
                            {t(`messaging.approval.${approval}`)}
                        </MenuItem>
                    ))}
                </TextField>
            )}

            <Typography variant="subtitle2" component="h3">
                {t("messagingAccounts.limits.title")}
            </Typography>
            <Box sx={{display: "flex", gap: 2}}>
                <TextField
                    label={t("messagingAccounts.limits.messagesPerSecond")}
                    value={values.limits.messagesPerSecond}
                    disabled={disabled}
                    error={!!errors["limits.messagesPerSecond"]}
                    helperText={errorText("limits.messagesPerSecond")}
                    onChange={(event) => setLimit("messagesPerSecond", event.target.value)}
                    slotProps={{htmlInput: {inputMode: "numeric"}}}
                    fullWidth
                />
                <TextField
                    label={t("messagingAccounts.limits.otpReservedPerSecond")}
                    value={values.limits.otpReservedPerSecond}
                    disabled={disabled}
                    error={!!errors["limits.otpReservedPerSecond"]}
                    helperText={
                        errorText("limits.otpReservedPerSecond") ??
                        t("messagingAccounts.limits.otpReservedHelp")
                    }
                    onChange={(event) => setLimit("otpReservedPerSecond", event.target.value)}
                    slotProps={{htmlInput: {inputMode: "numeric"}}}
                    fullWidth
                />
            </Box>
            <TextField
                label={t("messagingAccounts.limits.allowedCallingCodes")}
                value={values.limits.allowedCallingCodes}
                disabled={disabled}
                error={!!errors["limits.allowedCallingCodes"]}
                helperText={
                    errorText("limits.allowedCallingCodes") ??
                    t("messagingAccounts.limits.allowedCallingCodesHelp")
                }
                onChange={(event) => setLimit("allowedCallingCodes", event.target.value)}
                fullWidth
            />

            <FormControlLabel
                control={
                    <Switch
                        checked={values.isDefault}
                        disabled={disabled}
                        onChange={(event) =>
                            setValues({...values, isDefault: event.target.checked})
                        }
                    />
                }
                label={t("messagingAccounts.field.is_default", {channel: channelName})}
            />

            {typedCredentials(values.provider).length > 0 && (
                <>
                    <Typography variant="subtitle2" component="h3">
                        {t("messagingAccounts.credentials.title")}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                        {t("messagingAccounts.credentials.description")}
                    </Typography>
                    {typedCredentials(values.provider).map((name) => (
                        <CredentialInput
                            key={`${values.provider}-${name}`}
                            name={name}
                            provider={values.provider}
                            replacedAt={account?.credentials?.[name]?.replaced_at}
                            value={values.credentials[name] ?? ""}
                            disabled={disabled}
                            onChange={(value) =>
                                setValues({
                                    ...values,
                                    credentials: {...values.credentials, [name]: value},
                                })
                            }
                        />
                    ))}
                </>
            )}

            {hasWebhook(values.provider) && (
                <CallbackSection
                    account={account}
                    provider={values.provider}
                    canWrite={canWrite}
                    initialCallback={initialCallback}
                    onChanged={onChanged}
                />
            )}

            <Box sx={{display: "flex", gap: 2}}>
                {canWrite && !unsupported && (
                    <Button variant="contained" disabled={saving} onClick={save}>
                        {t("messagingAccounts.editor.save")}
                    </Button>
                )}
                <Button variant="outlined" onClick={onClose}>
                    {canWrite
                        ? t("messagingAccounts.editor.cancel")
                        : t("messagingAccounts.editor.close")}
                </Button>
            </Box>
        </Box>
    )
}
