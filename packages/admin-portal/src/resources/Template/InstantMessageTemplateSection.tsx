// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {
    Alert,
    Box,
    Button,
    Chip,
    IconButton,
    MenuItem,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline"
import {useGetOne} from "react-admin"
import {useFormContext, useWatch} from "react-hook-form"
import {useTranslation} from "react-i18next"
import {ITenantSettings} from "@sequentech/ui-core"
import {FormStyles} from "@/components/styles/FormStyles"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {templateApprovalRows, templatePurpose} from "@/services/templateApproval"
import {EMessageChannel} from "@/types/messaging"

export type InstantMessageChannel =
    | EMessageChannel.WHATSAPP
    | EMessageChannel.VIBER
    | EMessageChannel.MESSENGER

interface InstantMessageTemplateSectionProps {
    channel: InstantMessageChannel
}

const contentKey = (channel: InstantMessageChannel) => channel.toLowerCase()

const ParametersInput: React.FC<{source: string}> = ({source}) => {
    const {t} = useTranslation()
    const {setValue} = useFormContext()
    const watched = useWatch({name: source}) as string[] | undefined
    const parameters = Array.isArray(watched) ? watched : []
    const update = (next: string[]) => setValue(source, next, {shouldDirty: true})

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 1}}>
            <Typography variant="body2" sx={{fontWeight: 500}}>
                {t("messaging.templates.parameters")}
            </Typography>
            <Typography variant="caption" color="text.secondary">
                {t("messaging.templates.parametersHelp")}
            </Typography>
            {parameters.map((parameter, index) => (
                <Box key={index} sx={{display: "flex", gap: 1, alignItems: "center"}}>
                    <TextField
                        size="small"
                        fullWidth
                        label={t("messaging.templates.parameter", {position: index + 1})}
                        value={parameter}
                        onChange={(event) =>
                            update(
                                parameters.map((value, position) =>
                                    position === index ? event.target.value : value
                                )
                            )
                        }
                    />
                    <IconButton
                        aria-label={String(
                            t("messaging.templates.removeParameter", {position: index + 1})
                        )}
                        onClick={() =>
                            update(parameters.filter((_value, position) => position !== index))
                        }
                    >
                        <DeleteOutlineIcon />
                    </IconButton>
                </Box>
            ))}
            <Box>
                <Button startIcon={<AddIcon />} onClick={() => update([...parameters, ""])}>
                    {t("messaging.templates.addParameter")}
                </Button>
            </Box>
        </Box>
    )
}

const ApprovalStatus: React.FC<{channel: InstantMessageChannel}> = ({channel}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const type = useWatch({name: "type"}) as string | undefined
    const purpose = templatePurpose(type)
    const {accounts, loading} = useMessagingAccounts()
    const {data: tenant} = useGetOne("sequent_backend_tenant", {id: tenantId ?? ""})
    const channelAccounts = useMemo(
        () => accounts.filter((account) => account.channel === channel),
        [accounts, channel]
    )
    const [selectedId, setSelectedId] = useState<string>()
    const account =
        channelAccounts.find((candidate) => candidate.id === selectedId) ??
        channelAccounts.find((candidate) => candidate.is_default) ??
        channelAccounts[0]
    const languages = (tenant?.settings as ITenantSettings | undefined)?.language_conf
        ?.enabled_language_codes ?? ["en"]

    if (loading) {
        return null
    }
    if (!account) {
        return (
            <Alert severity="info">
                {t("messaging.templates.noAccount", {
                    channel: t(`messaging.channel.${channel}`),
                })}
            </Alert>
        )
    }
    const rows = templateApprovalRows(account.status, purpose, languages)
    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 1}}>
            <TextField
                select
                size="small"
                label={t("messaging.templates.account")}
                value={account.id}
                onChange={(event) => setSelectedId(event.target.value)}
                sx={{maxWidth: 400}}
            >
                {channelAccounts.map((candidate) => (
                    <MenuItem key={candidate.id} value={candidate.id}>
                        {candidate.name}
                    </MenuItem>
                ))}
            </TextField>
            <Table size="small" aria-label={String(t("messaging.templates.approvalTitle"))}>
                <TableHead>
                    <TableRow>
                        <TableCell>{t("messaging.templates.language")}</TableCell>
                        <TableCell>
                            {t("messaging.templates.approvalFor", {
                                purpose: t(`messaging.purpose.${purpose}`),
                            })}
                        </TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {rows.map((row) => (
                        <TableRow key={row.language}>
                            <TableCell>{row.language}</TableCell>
                            <TableCell>
                                <Chip
                                    size="small"
                                    color={row.approved ? "success" : "default"}
                                    label={t(
                                        row.approved
                                            ? "messaging.templates.approved"
                                            : "messaging.templates.notApproved"
                                    )}
                                />
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <Typography variant="caption" color="text.secondary">
                {t("messaging.templates.approvalHelp")}
            </Typography>
        </Box>
    )
}

/** WhatsApp, Viber and Messenger content of a template. */
export const InstantMessageTemplateSection: React.FC<InstantMessageTemplateSectionProps> = ({
    channel,
}) => {
    const {t} = useTranslation()
    const key = contentKey(channel)

    if (channel === EMessageChannel.MESSENGER) {
        return (
            <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
                <Typography variant="body2" color="text.secondary">
                    {t("messaging.templates.messengerIntro")}
                </Typography>
                <FormStyles.TextInput
                    minRows={3}
                    multiline={true}
                    source={`template.${key}.message`}
                    label={String(t("messaging.templates.messengerMessage"))}
                />
                <Alert severity="info">{t("messaging.templates.messengerWindow")}</Alert>
            </Box>
        )
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <Typography variant="body2" color="text.secondary">
                {t(`messaging.templates.intro.${channel}`)}
            </Typography>
            <FormStyles.TextInput
                minRows={3}
                multiline={true}
                source={`template.${key}.message`}
                label={String(t("messaging.templates.approvedWording"))}
                helperText={String(t("messaging.templates.approvedWordingHelp"))}
            />
            <ParametersInput source={`template.${key}.parameters`} />
            <ApprovalStatus channel={channel} />
        </Box>
    )
}
