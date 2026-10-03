// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useMutation} from "@apollo/client"
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
    EReadinessPolicy,
    IMessagingAccount,
    MESSAGE_CHANNELS,
    MESSAGE_PURPOSES,
    READINESS_POLICIES,
} from "@/types/messaging"
import {hasWebhook, isMetaProvider, typedCredentials, webhookPath} from "@/services/messaging"
import {UPSERT_MESSAGING_ACCOUNT} from "@/queries/UpsertMessagingAccount"
import {REPLACE_MESSAGING_ACCOUNT_CREDENTIALS} from "@/queries/ReplaceMessagingAccountCredentials"
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
import {HttpApiSenderEditor} from "./HttpApiSenderEditor"

export const MESSAGING_ACCOUNT_EDITOR_TITLE = "messaging-account-editor-title"

const WRITE_ROLE = {headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE}}

interface IUpsertMessagingAccountResult {
    upsert_messaging_account: {id: string} | null
}

interface IReplaceCredentialsResult {
    replace_messaging_account_credentials: {
        replaced: string[]
        verify_token: string | null
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
    "label",
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

interface IVerifyTokenDialogProps {
    token: string
    onClose: () => void
}

/** Shows a generated verify token once; closing it forgets the token. */
export const VerifyTokenDialog: React.FC<IVerifyTokenDialogProps> = ({token, onClose}) => {
    const {t} = useTranslation()
    const copy = useCopy()
    return (
        <Dialog open onClose={onClose} aria-labelledby="verify-token-title">
            <DialogTitle id="verify-token-title">
                {t("messagingAccounts.webhook.tokenTitle")}
            </DialogTitle>
            <DialogContent sx={{display: "flex", flexDirection: "column", gap: 2}}>
                <Typography variant="body2">{t("messagingAccounts.webhook.tokenOnce")}</Typography>
                <TextField
                    value={token}
                    label={t("messaging.credential.VERIFY_TOKEN")}
                    fullWidth
                    slotProps={{
                        input: {
                            readOnly: true,
                            endAdornment: (
                                <InputAdornment position="end">
                                    <IconButton
                                        aria-label={t("messagingAccounts.webhook.copyToken")}
                                        onClick={() => copy(token)}
                                    >
                                        <ContentCopyIcon />
                                    </IconButton>
                                </InputAdornment>
                            ),
                        },
                    }}
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

interface ICallbackSectionProps {
    account: IMessagingAccount | undefined
    provider: EMessagingProvider
    canWrite: boolean
    onChanged: () => void
}

/** The account's webhook for delivery reports and replies, and Meta's verify token. */
export const CallbackSection: React.FC<ICallbackSectionProps> = ({
    account,
    provider,
    canWrite,
    onChanged,
}) => {
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const copy = useCopy()
    const [token, setToken] = useState<string | null>(null)
    const [generating, setGenerating] = useState(false)
    const [replaceCredentials] = useMutation<IReplaceCredentialsResult>(
        REPLACE_MESSAGING_ACCOUNT_CREDENTIALS,
        {context: WRITE_ROLE}
    )
    const path = webhookPath(provider, account?.webhook_key)
    const verifyTokenSet = account?.credentials?.[ECredentialName.VERIFY_TOKEN]?.replaced_at

    const generate = async () => {
        if (!account) {
            return
        }
        setGenerating(true)
        try {
            const {data} = await replaceCredentials({
                variables: {id: account.id, credentials: {}, generateVerifyToken: true},
            })
            const generated = data?.replace_messaging_account_credentials?.verify_token
            if (!generated) {
                throw new Error("No verify token returned")
            }
            setToken(generated)
            onChanged()
        } catch {
            notify(t("messagingAccounts.webhook.tokenError"), {type: "error"})
        } finally {
            setGenerating(false)
        }
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Typography variant="subtitle2" component="h3">
                {t("messagingAccounts.webhook.title")}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t("messagingAccounts.webhook.description")}
                {provider === EMessagingProvider.HTTP_API
                    ? ` ${t("messagingAccounts.webhook.httpHelp")}`
                    : ""}
            </Typography>
            <TextField
                label={t("messagingAccounts.webhook.path")}
                value={path ?? t("messagingAccounts.webhook.afterSaving")}
                helperText={t("messagingAccounts.webhook.pathHelp")}
                fullWidth
                slotProps={{
                    input: {
                        readOnly: true,
                        endAdornment: path ? (
                            <InputAdornment position="end">
                                <IconButton
                                    aria-label={t("messagingAccounts.webhook.copyPath")}
                                    onClick={() => copy(path)}
                                >
                                    <ContentCopyIcon />
                                </IconButton>
                            </InputAdornment>
                        ) : undefined,
                    },
                }}
            />
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
                        <Button variant="outlined" disabled={generating} onClick={generate}>
                            {t("messagingAccounts.webhook.generate")}
                        </Button>
                    )}
                </Box>
            )}
            {token && <VerifyTokenDialog token={token} onClose={() => setToken(null)} />}
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
    /** Called after anything was saved, so the list can be read again. */
    onChanged: () => void
    onClose: () => void
}

export const MessagingAccountEditor: React.FC<IMessagingAccountEditorProps> = ({
    account,
    canWrite,
    onChanged,
    onClose,
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const [values, setValues] = useState<IAccountFormValues>(() =>
        account
            ? formFromAccount(account)
            : emptyAccountForm(EMessageChannel.EMAIL, EMessagingProvider.AWS_SES)
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
    const disabled = !canWrite || saving
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
            const id = data?.upsert_messaging_account?.id ?? account?.id
            if (!id) {
                throw new Error("No account id returned")
            }
            const credentials = credentialsPayload(values)
            if (credentials) {
                await replaceCredentials({variables: {id, credentials}})
            }
            notify(t("messagingAccounts.save.success"), {type: "success"})
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

            {values.provider === EMessagingProvider.HTTP_API && (
                <HttpApiSenderEditor
                    values={values.http}
                    submitted={submitted}
                    disabled={disabled}
                    onChange={(http) => setValues({...values, http})}
                />
            )}

            <TextField
                select
                label={t("messagingAccounts.field.readiness")}
                value={values.readiness}
                disabled={disabled}
                helperText={t("messagingAccounts.fieldHelp.readiness")}
                onChange={(event) =>
                    setValues({...values, readiness: event.target.value as EReadinessPolicy})
                }
                fullWidth
            >
                {READINESS_POLICIES.map((policy) => (
                    <MenuItem key={policy} value={policy}>
                        {t(`messaging.readinessPolicy.${policy}`)}
                    </MenuItem>
                ))}
            </TextField>

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
                    onChanged={onChanged}
                />
            )}

            <Box sx={{display: "flex", gap: 2}}>
                {canWrite && (
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
