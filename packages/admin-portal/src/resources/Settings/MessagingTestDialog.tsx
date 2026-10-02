// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Alert,
    AlertColor,
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    MenuItem,
    TextField,
    Typography,
} from "@mui/material"
import {IPermissions} from "@/types/keycloak"
import {
    EMessageAttemptState,
    EMessagePurpose,
    EMessagingProvider,
    IMessagingAccount,
    MESSAGE_PURPOSES,
} from "@/types/messaging"
import {recipientKind} from "@/services/messaging"
import {TEST_MESSAGING_ACCOUNT} from "@/queries/TestMessagingAccount"

interface ITestMessagingAccountResult {
    test_messaging_account: {
        message_id: string | null
        state: EMessageAttemptState
        reason: string | null
    } | null
}

type ITestOutcome =
    | {kind: "RESULT"; state: EMessageAttemptState; reason: string | null}
    | {kind: "ERROR"}

/** An accepted message is not a delivered one: only DELIVERED reads as success. */
export const testStateSeverity = (state: EMessageAttemptState): AlertColor => {
    switch (state) {
        case EMessageAttemptState.DELIVERED:
            return "success"
        case EMessageAttemptState.ACCEPTED:
        case EMessageAttemptState.QUEUED:
            return "info"
        case EMessageAttemptState.UNKNOWN:
            return "warning"
        case EMessageAttemptState.FAILED:
            return "error"
    }
}

export interface IMessagingTestDialogProps {
    account: IMessagingAccount
    /** The tenant's enabled languages; the first is preselected. */
    languages: string[]
    onClose: () => void
}

export const MessagingTestDialog: React.FC<IMessagingTestDialogProps> = ({
    account,
    languages,
    onClose,
}) => {
    const {t} = useTranslation()
    const [purpose, setPurpose] = useState<EMessagePurpose>(EMessagePurpose.OTP)
    const [destination, setDestination] = useState("")
    const [language, setLanguage] = useState(languages[0] ?? "")
    const [template, setTemplate] = useState("")
    const needsTemplate = account.provider === EMessagingProvider.WHATSAPP_CLOUD_API
    const [sending, setSending] = useState(false)
    const [outcome, setOutcome] = useState<ITestOutcome | null>(null)
    const [testAccount] = useMutation<ITestMessagingAccountResult>(TEST_MESSAGING_ACCOUNT, {
        context: {headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE}},
    })

    const send = async () => {
        setSending(true)
        setOutcome(null)
        try {
            const {data} = await testAccount({
                variables: {
                    id: account.id,
                    purpose,
                    destination: destination.trim(),
                    language: language || null,
                    ...(needsTemplate ? {template: template.trim()} : {}),
                },
            })
            const result = data?.test_messaging_account
            setOutcome(
                result
                    ? {kind: "RESULT", state: result.state, reason: result.reason}
                    : {kind: "ERROR"}
            )
        } catch {
            setOutcome({kind: "ERROR"})
        } finally {
            setSending(false)
        }
    }

    return (
        <Dialog
            open
            onClose={onClose}
            fullWidth
            maxWidth="sm"
            aria-labelledby="messaging-test-title"
        >
            <DialogTitle id="messaging-test-title">
                {t("messagingAccounts.test.title", {name: account.name})}
            </DialogTitle>
            <DialogContent sx={{display: "flex", flexDirection: "column", gap: 2, paddingTop: 1}}>
                <Typography variant="body2" color="text.secondary">
                    {t("messagingAccounts.test.description")}
                </Typography>
                <TextField
                    select
                    label={t("messagingAccounts.test.purpose")}
                    value={purpose}
                    onChange={(event) => setPurpose(event.target.value as EMessagePurpose)}
                    sx={{marginTop: 1}}
                >
                    {MESSAGE_PURPOSES.map((value) => (
                        <MenuItem key={value} value={value}>
                            {t(`messaging.purpose.${value}`)}
                        </MenuItem>
                    ))}
                </TextField>
                <TextField
                    label={t(
                        `messagingAccounts.test.destination.${recipientKind(account.channel)}`
                    )}
                    value={destination}
                    onChange={(event) => setDestination(event.target.value)}
                    fullWidth
                />
                {languages.length > 0 && (
                    <TextField
                        select
                        label={t("messagingAccounts.test.language")}
                        value={language}
                        onChange={(event) => setLanguage(event.target.value)}
                    >
                        {languages.map((code) => (
                            <MenuItem key={code} value={code}>
                                {code}
                            </MenuItem>
                        ))}
                    </TextField>
                )}
                {needsTemplate && (
                    <TextField
                        label={t("messagingAccounts.test.template")}
                        helperText={t("messagingAccounts.test.templateHelp")}
                        value={template}
                        onChange={(event) => setTemplate(event.target.value)}
                        fullWidth
                    />
                )}
                {account.provider === EMessagingProvider.VIBER_INFOBIP && (
                    <Typography variant="body2" color="text.secondary">
                        {t("messagingAccounts.test.viberTemplate")}
                    </Typography>
                )}
                {outcome?.kind === "RESULT" && (
                    <Alert severity={testStateSeverity(outcome.state)}>
                        <strong>{t(`messaging.state.${outcome.state}`)}</strong>
                        {". "}
                        {t(`messaging.stateHelp.${outcome.state}`)}
                        {outcome.reason
                            ? ` ${t("messagingAccounts.test.reason", {reason: outcome.reason})}`
                            : ""}
                    </Alert>
                )}
                {outcome?.kind === "ERROR" && (
                    <Alert severity="error">{t("messagingAccounts.test.error")}</Alert>
                )}
            </DialogContent>
            <DialogActions>
                <Button variant="outlined" onClick={onClose}>
                    {t("messagingAccounts.editor.close")}
                </Button>
                <Button
                    variant="contained"
                    disabled={sending || !destination.trim() || (needsTemplate && !template.trim())}
                    onClick={send}
                >
                    {t("messagingAccounts.test.send")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}
