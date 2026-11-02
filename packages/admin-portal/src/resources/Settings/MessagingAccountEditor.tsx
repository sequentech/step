// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
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
    EApprovalSource,
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
    accountApproval,
    hasWebhook,
    isMetaProvider,
    isUnsupportedProvider,
    typedCredentials,
} from "@/services/messaging"
import {actionRefusalReason, reasonText} from "@/services/messagingReasons"
import {UPSERT_MESSAGING_ACCOUNT} from "@/queries/UpsertMessagingAccount"
import {REPLACE_MESSAGING_ACCOUNT_CREDENTIALS} from "@/queries/ReplaceMessagingAccountCredentials"
import {
    IMessagingAccountCallbackUrlResult,
    MESSAGING_ACCOUNT_CALLBACK_URL,
} from "@/queries/MessagingAccountCallbackUrl"
import {
    IMessagingAccountChangeImpactResult,
    MESSAGING_ACCOUNT_CHANGE_IMPACT,
} from "@/queries/MessagingAccountChangeImpact"
import {AccountStatusDetails} from "./MessagingAccountEditorStatus"
import {
    EAccountFormError,
    IAccountFormValues,
    credentialsToReenter,
    emptyAccountForm,
    endpointField,
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
    /** The address it is sent to changed: it must be entered again. */
    mustReenter?: boolean
    onChange: (value: string) => void
}

/** A write-only secret: once set, only the time it was last replaced is shown. */
export const CredentialInput: React.FC<ICredentialInputProps> = ({
    name,
    provider,
    replacedAt,
    value,
    disabled,
    mustReenter = false,
    onChange,
}) => {
    const {t, i18n} = useTranslation()
    const [replacing, setReplacing] = useState(false)
    const label = t(`messaging.credential.${name}`)
    if (replacedAt && !replacing && !mustReenter) {
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
            error={mustReenter && !value}
            helperText={
                mustReenter && !value
                    ? t("messagingAccounts.credentials.reenter")
                    : t(`messagingAccounts.credentialHelp.${provider}.${name}`)
            }
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
    const [confirmingReplace, setConfirmingReplace] = useState(false)
    const [replaceCredentials] = useMutation<IReplaceCredentialsResult>(
        REPLACE_MESSAGING_ACCOUNT_CREDENTIALS,
        {context: WRITE_ROLE}
    )
    const {data, loading, error} = useQuery<IMessagingAccountCallbackUrlResult>(
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
            : error && !callback
              ? t("messagingAccounts.webhook.loadError")
              : (callbackUrl ?? reasonText(t, callback?.reason))

    // Replacing report credentials that are set breaks Infobip's subscription
    // until it is updated with the new ones: asked first.
    const generateReportCredentials = () =>
        reportSet ? setConfirmingReplace(true) : generate("REPORT_CREDENTIALS")

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
                                onClick={generateReportCredentials}
                            >
                                {t("messagingAccounts.reports.generate")}
                            </Button>
                        )}
                    </Box>
                </>
            )}
            {token && <VerifyTokenDialog token={token} onClose={() => setToken(null)} />}
            {confirmingReplace && (
                <Dialog
                    open
                    onClose={() => setConfirmingReplace(false)}
                    aria-labelledby="replace-report-credentials-title"
                >
                    <DialogTitle id="replace-report-credentials-title">
                        {t("messagingAccounts.reports.confirmTitle")}
                    </DialogTitle>
                    <DialogContent>
                        <Typography variant="body2">
                            {t("messagingAccounts.reports.confirmBody")}
                        </Typography>
                    </DialogContent>
                    <DialogActions>
                        <Button variant="outlined" onClick={() => setConfirmingReplace(false)}>
                            {t("messagingAccounts.editor.cancel")}
                        </Button>
                        <Button
                            variant="contained"
                            color="warning"
                            onClick={() => {
                                setConfirmingReplace(false)
                                generate("REPORT_CREDENTIALS")
                            }}
                        >
                            {t("messagingAccounts.reports.confirm")}
                        </Button>
                    </DialogActions>
                </Dialog>
            )}
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
        <Box sx={{display: "flex", flexDirection: "column", gap: 2, containerType: "inline-size"}}>
            <Typography variant="subtitle2" component="h3">
                {t("messagingAccounts.viber.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t("messagingAccounts.viber.description")}
            </Typography>
            {rows.map((row, index) => {
                const error = errors[`viberTemplates.${index}`]
                return (
                    <Box
                        key={index}
                        sx={{
                            "display": "grid",
                            "gridTemplateColumns": "minmax(0, 1fr) minmax(0, 1fr) auto",
                            "gap": 1.5,
                            "alignItems": "start",
                            "@container (min-width: 480px)": {
                                gridTemplateColumns:
                                    "minmax(0, 1fr) minmax(0, 0.8fr) minmax(0, 1.6fr) auto",
                            },
                        }}
                    >
                        <TextField
                            select
                            label={t("messagingAccounts.viber.purpose")}
                            value={row.purpose}
                            disabled={disabled}
                            onChange={(event) =>
                                update(index, {purpose: event.target.value as EMessagePurpose})
                            }
                            fullWidth
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
                            fullWidth
                        />
                        <TextField
                            label={t("messagingAccounts.viber.templateId")}
                            value={row.templateId}
                            disabled={disabled}
                            error={!!error}
                            helperText={error ? t(`messagingAccounts.error.${error}`) : undefined}
                            onChange={(event) => update(index, {templateId: event.target.value})}
                            fullWidth
                            sx={{
                                "gridColumn": "1 / 3",
                                "@container (min-width: 480px)": {gridColumn: "auto"},
                            }}
                        />
                        {!disabled && (
                            <IconButton
                                aria-label={t("messagingAccounts.viber.remove")}
                                sx={{
                                    "gridColumn": 3,
                                    "gridRow": "1 / 3",
                                    "alignSelf": "start",
                                    "mt": 1,
                                    "@container (min-width: 480px)": {
                                        gridColumn: "auto",
                                        gridRow: "auto",
                                    },
                                }}
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
    /** Why the last save was refused, as the backend said it. */
    const [refusal, setRefusal] = useState<string | null>(null)
    // Once a new account is created, saving again updates it: never a second account.
    const [savedId, setSavedId] = useState<string | null>(account?.id ?? null)
    const [upsertAccount] = useMutation<IUpsertMessagingAccountResult>(UPSERT_MESSAGING_ACCOUNT, {
        context: WRITE_ROLE,
    })
    const originalForm = useMemo(() => (account ? formFromAccount(account) : undefined), [account])
    const errors = submitted ? validateAccountForm(values, originalForm) : {}
    // Credentials that would go to a new address unless entered again.
    const reenter = credentialsToReenter(originalForm, values)
    const endpoint = endpointField(values.provider)
    // A removed provider's account can only be replaced: nothing is edited or saved.
    const unsupported = isUnsupportedProvider(values.provider)
    const disabled = !canWrite || saving || unsupported
    const channelName = t(`messaging.channel.${values.channel}`)
    const needsApproval = requiresProviderApproval(values.provider, values.channel)
    const original = originalForm?.sender
    const changed = (key: string) => !!original && (values.sender[key] ?? "") !== original[key]
    const pageChanged =
        values.provider === EMessagingProvider.MESSENGER_SEND_API && changed("page_id")
    const numberChanged =
        values.provider === EMessagingProvider.WHATSAPP_CLOUD_API &&
        (changed("phone_number_id") || changed("business_account_id"))
    // The voters who must connect again after a Page change, counted by the
    // backend across the events that use the account (BC-F13).
    const impact = useQuery<IMessagingAccountChangeImpactResult>(MESSAGING_ACCOUNT_CHANGE_IMPACT, {
        variables: {accountId: account?.id},
        skip: !account || !canWrite || !pageChanged,
        context: WRITE_ROLE,
        fetchPolicy: "network-only",
    })
    const affectedVoters = impact.data?.messaging_account_change_impact?.affected_voters
    // Where the provider's check reports the approval, it is used and shown;
    // the administrator's entry only where it doesn't.
    const approvalSource = account
        ? accountApproval(account.provider_approval, account.status).source
        : EApprovalSource.ADMIN_ENTERED

    const errorText = (path: string) =>
        errors[path] ? t(`messagingAccounts.error.${errors[path]}`) : undefined

    // Said as soon as the address changes, not only when saving.
    const fieldError = (key: string): string | undefined =>
        errorText(`sender.${key}`) ??
        (key === endpoint && reenter.length
            ? t(`messagingAccounts.error.${EAccountFormError.CREDENTIALS_FOLLOW_ENDPOINT}`)
            : undefined)

    const setSender = (key: string, value: string) =>
        setValues({...values, sender: {...values.sender, [key]: value}})

    const setLimit = (key: keyof IAccountFormValues["limits"], value: string) =>
        setValues({...values, limits: {...values.limits, [key]: value}})

    const save = async () => {
        setSubmitted(true)
        setRefusal(null)
        if (Object.keys(validateAccountForm(values, originalForm)).length) {
            return
        }
        setSaving(true)
        const creating = !savedId
        try {
            // One call: the account and the credentials typed with it are
            // stored in one transaction, or neither is.
            const {data} = await upsertAccount({
                variables: upsertAccountVariables(savedId, values),
            })
            const saved = data?.upsert_messaging_account
            const id = saved?.id ?? savedId
            if (!id) {
                throw new Error("No account id returned")
            }
            setSavedId(id)
            notify(t("messagingAccounts.save.success"), {type: "success"})
            if (creating && hasWebhook(values.provider) && onCreated) {
                onCreated(id, {callback_url: saved?.callback_url, reason: saved?.callback_reason})
                return
            }
            onChanged()
            onClose()
        } catch (error) {
            const reason = actionRefusalReason(error)
            setRefusal(
                reason
                    ? t("messagingAccounts.save.refused", {reason: reasonText(t, reason)})
                    : t("messagingAccounts.save.error")
            )
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
                    error={!!fieldError(key)}
                    helperText={
                        fieldError(key) ??
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
                    })}{" "}
                    {affectedVoters !== undefined && affectedVoters !== null
                        ? t("messagingAccounts.warning.pageChangeCount", {
                              count: affectedVoters,
                              page: original?.page_name || original?.page_id,
                          })
                        : impact.error
                          ? t("messagingAccounts.warning.pageChangeCountError")
                          : impact.loading
                            ? t("messagingAccounts.warning.pageChangeCounting")
                            : null}{" "}
                    {t("messagingAccounts.warning.recheck")}
                </Alert>
            )}
            {numberChanged && (
                <Alert severity="warning">
                    {t("messagingAccounts.warning.numberChange")}{" "}
                    {t("messagingAccounts.warning.recheck")}
                </Alert>
            )}

            {values.provider === EMessagingProvider.VIBER_INFOBIP && (
                <ViberTemplatesEditor
                    values={values}
                    errors={errors}
                    disabled={disabled}
                    onChange={setValues}
                />
            )}

            {needsApproval && approvalSource === EApprovalSource.ADMIN_ENTERED && (
                <TextField
                    select
                    label={t("messagingAccounts.field.provider_approval")}
                    value={values.providerApproval}
                    disabled={disabled}
                    helperText={`${t("messagingAccounts.fieldHelp.provider_approval")} ${t(
                        "messagingAccounts.status.approvalEnteredHelp"
                    )}`}
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
                    helperText={
                        errorText("limits.messagesPerSecond") ??
                        t("messagingAccounts.limits.messagesPerSecondHelp")
                    }
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
                <TextField
                    label={t("messagingAccounts.limits.otpConcurrency")}
                    value={values.limits.otpConcurrency}
                    disabled={disabled}
                    error={!!errors["limits.otpConcurrency"]}
                    helperText={
                        errorText("limits.otpConcurrency") ??
                        t("messagingAccounts.limits.otpConcurrencyHelp")
                    }
                    onChange={(event) => setLimit("otpConcurrency", event.target.value)}
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
                            mustReenter={reenter.includes(name) || !!values.credentials[name]}
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

            {account && !unsupported ? <AccountStatusDetails account={account} /> : null}

            {hasWebhook(values.provider) && (
                <CallbackSection
                    account={account}
                    provider={values.provider}
                    canWrite={canWrite}
                    initialCallback={initialCallback}
                    onChanged={onChanged}
                />
            )}

            {refusal ? <Alert severity="error">{refusal}</Alert> : null}

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
