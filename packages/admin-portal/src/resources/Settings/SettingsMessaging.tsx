// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import {useMutation} from "@apollo/client"
import {useGetOne, useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {
    Box,
    Button,
    Chip,
    CircularProgress,
    Drawer,
    IconButton,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Tooltip,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import DeleteIcon from "@mui/icons-material/Delete"
import EditIcon from "@mui/icons-material/Edit"
import SendIcon from "@mui/icons-material/Send"
import SyncIcon from "@mui/icons-material/Sync"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {ITenantSettings} from "@sequentech/ui-core"
import {ChannelLabel, Dialog} from "@sequentech/ui-essentials"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useMessagingAccounts} from "@/hooks/useMessagingAccounts"
import {IPermissions} from "@/types/keycloak"
import {
    EMessagePurpose,
    EReadinessBlocker,
    EReadinessPolicy,
    IMessagingAccount,
    MESSAGE_PURPOSES,
} from "@/types/messaging"
import {accountSenderLabel} from "@/services/messaging"
import {CHECK_MESSAGING_ACCOUNT} from "@/queries/CheckMessagingAccount"
import {DELETE_MESSAGING_ACCOUNT} from "@/queries/DeleteMessagingAccount"
import {accountReadiness} from "./messagingAccountForm"
import {
    MESSAGING_ACCOUNT_EDITOR_TITLE,
    MessagingAccountEditor,
    formatDate,
} from "./MessagingAccountEditor"
import {MessagingTestDialog} from "./MessagingTestDialog"

const WRITE_ROLE = {headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE}}

interface IReadinessCellProps {
    ready: boolean
    label: string
    reasons: string[]
}

/** A readiness chip with the reasons it is not ready below it. */
export const ReadinessCell: React.FC<IReadinessCellProps> = ({ready, label, reasons}) => (
    <Box sx={{display: "flex", flexDirection: "column", gap: "2px", alignItems: "flex-start"}}>
        <Chip
            size="small"
            color={ready ? "success" : "default"}
            variant={ready ? "filled" : "outlined"}
            label={label}
        />
        {reasons.map((reason) => (
            <Typography key={reason} variant="caption" color="text.secondary">
                {reason}
            </Typography>
        ))}
    </Box>
)

type IDrawerState = {kind: "CLOSED"} | {kind: "ADD"} | {kind: "EDIT"; account: IMessagingAccount}

export const SettingsMessaging: React.FC = () => {
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canWrite = authContext.isAuthorized(true, tenantId, IPermissions.MESSAGING_ACCOUNT_WRITE)
    const {accounts, loading, error, refetch} = useMessagingAccounts()
    const {data: tenant} = useGetOne(
        "sequent_backend_tenant",
        {id: tenantId},
        {enabled: !!tenantId}
    )
    const languages =
        (tenant?.settings as ITenantSettings | undefined)?.language_conf?.enabled_language_codes ??
        []
    const [drawer, setDrawer] = useState<IDrawerState>({kind: "CLOSED"})
    const [testing, setTesting] = useState<IMessagingAccount | null>(null)
    const [deleting, setDeleting] = useState<IMessagingAccount | null>(null)
    const [checking, setChecking] = useState<string | null>(null)
    const [checkAccount] = useMutation(CHECK_MESSAGING_ACCOUNT, {context: WRITE_ROLE})
    const [deleteAccount] = useMutation(DELETE_MESSAGING_ACCOUNT, {context: WRITE_ROLE})

    const reload = async () => {
        try {
            await refetch()
        } catch {
            notify(t("messagingAccounts.list.loadError"), {type: "error"})
        }
    }

    const check = async (account: IMessagingAccount) => {
        setChecking(account.id)
        try {
            await checkAccount({variables: {id: account.id}})
            notify(t("messagingAccounts.check.done", {name: account.name}), {type: "info"})
            await reload()
        } catch {
            notify(t("messagingAccounts.check.error", {name: account.name}), {type: "error"})
        } finally {
            setChecking(null)
        }
    }

    const remove = async (account: IMessagingAccount) => {
        try {
            await deleteAccount({variables: {id: account.id}})
            notify(t("messagingAccounts.delete.success"), {type: "success"})
            await reload()
        } catch {
            notify(t("messagingAccounts.delete.error"), {type: "error"})
        }
    }

    const blockerLabels = (blockers: EReadinessBlocker[]) =>
        blockers.map((blocker) => t(`messaging.blocker.${blocker}`))

    const purposeLabel = (purpose: EMessagePurpose) =>
        purpose === EMessagePurpose.OTP
            ? t("messaging.readiness.readyOtp")
            : t("messaging.readiness.readyNotice")

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2, padding: 2}}>
            <Box
                sx={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                    gap: 2,
                }}
            >
                <Typography variant="body2" color="text.secondary" sx={{maxWidth: 900}}>
                    {t("messagingAccounts.description")}
                </Typography>
                {canWrite && (
                    <Button
                        variant="contained"
                        startIcon={<AddIcon />}
                        onClick={() => setDrawer({kind: "ADD"})}
                        sx={{flexShrink: 0}}
                    >
                        {t("messagingAccounts.list.add")}
                    </Button>
                )}
            </Box>

            {loading && accounts.length === 0 ? (
                <CircularProgress aria-label={t("messagingAccounts.list.loading")} />
            ) : error ? (
                <Typography color="error">{t("messagingAccounts.list.loadError")}</Typography>
            ) : accounts.length === 0 ? (
                <Typography variant="body1">{t("messagingAccounts.list.empty")}</Typography>
            ) : (
                <TableContainer>
                    <Table size="small" aria-label={t("messagingAccounts.list.title")}>
                        <TableHead>
                            <TableRow>
                                <TableCell>{t("messagingAccounts.column.channel")}</TableCell>
                                <TableCell>{t("messagingAccounts.column.name")}</TableCell>
                                <TableCell>{t("messagingAccounts.column.sender")}</TableCell>
                                <TableCell>{t("messagingAccounts.column.provider")}</TableCell>
                                <TableCell>{t("messagingAccounts.column.default")}</TableCell>
                                <TableCell>{t("messaging.readiness.connected")}</TableCell>
                                {MESSAGE_PURPOSES.map((purpose) => (
                                    <TableCell key={purpose}>{purposeLabel(purpose)}</TableCell>
                                ))}
                                <TableCell>{t("messagingAccounts.column.lastCheck")}</TableCell>
                                <TableCell>{t("messagingAccounts.column.actions")}</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {accounts.map((account) => {
                                const readiness = accountReadiness(account)
                                const adminConfirmed =
                                    readiness.policy === EReadinessPolicy.ADMIN_CONFIRMED
                                return (
                                    <TableRow key={account.id}>
                                        <TableCell>
                                            <ChannelLabel
                                                channel={account.channel}
                                                label={t(`messaging.channel.${account.channel}`)}
                                            />
                                        </TableCell>
                                        <TableCell>{account.name}</TableCell>
                                        <TableCell>
                                            {accountSenderLabel(account.sender) ?? "–"}
                                        </TableCell>
                                        <TableCell>
                                            {t(`messaging.provider.${account.provider}`)}
                                        </TableCell>
                                        <TableCell>
                                            {account.is_default ? (
                                                <CheckCircleOutlineIcon
                                                    color="success"
                                                    titleAccess={t(
                                                        "messagingAccounts.column.isDefault"
                                                    )}
                                                />
                                            ) : null}
                                        </TableCell>
                                        <TableCell>
                                            {adminConfirmed ? (
                                                <ReadinessCell
                                                    ready
                                                    label={t("messaging.readiness.adminConfirmed")}
                                                    reasons={[]}
                                                />
                                            ) : (
                                                <ReadinessCell
                                                    ready={readiness.connected}
                                                    label={
                                                        readiness.connected
                                                            ? t("messaging.readiness.connected")
                                                            : t("messaging.readiness.notConnected")
                                                    }
                                                    reasons={
                                                        !readiness.connected &&
                                                        account.status?.reason
                                                            ? [account.status.reason]
                                                            : []
                                                    }
                                                />
                                            )}
                                        </TableCell>
                                        {MESSAGE_PURPOSES.map((purpose) => {
                                            const {blockers} = readiness.purposes[purpose]
                                            return (
                                                <TableCell key={purpose}>
                                                    <ReadinessCell
                                                        ready={blockers.length === 0}
                                                        label={
                                                            blockers.length === 0
                                                                ? purposeLabel(purpose)
                                                                : t("messaging.readiness.notReady")
                                                        }
                                                        reasons={blockerLabels(blockers)}
                                                    />
                                                </TableCell>
                                            )
                                        })}
                                        <TableCell>
                                            <Typography variant="caption" color="text.secondary">
                                                {adminConfirmed
                                                    ? t("messaging.readiness.checkNotUsed")
                                                    : account.status?.checked_at
                                                      ? t("messaging.readiness.lastCheck", {
                                                            date: formatDate(
                                                                account.status.checked_at,
                                                                i18n.language
                                                            ),
                                                        })
                                                      : t("messaging.readiness.neverChecked")}
                                            </Typography>
                                        </TableCell>
                                        <TableCell sx={{whiteSpace: "nowrap"}}>
                                            <Tooltip
                                                title={
                                                    canWrite
                                                        ? t("messagingAccounts.action.edit")
                                                        : t("messagingAccounts.action.view")
                                                }
                                            >
                                                <IconButton
                                                    aria-label={
                                                        canWrite
                                                            ? t(
                                                                  "messagingAccounts.action.editNamed",
                                                                  {name: account.name}
                                                              )
                                                            : t(
                                                                  "messagingAccounts.action.viewNamed",
                                                                  {name: account.name}
                                                              )
                                                    }
                                                    onClick={() =>
                                                        setDrawer({kind: "EDIT", account})
                                                    }
                                                >
                                                    {canWrite ? <EditIcon /> : <VisibilityIcon />}
                                                </IconButton>
                                            </Tooltip>
                                            {canWrite && (
                                                <>
                                                    <Tooltip
                                                        title={t("messagingAccounts.action.check")}
                                                    >
                                                        <span>
                                                            <IconButton
                                                                aria-label={t(
                                                                    "messagingAccounts.action.checkNamed",
                                                                    {
                                                                        name: account.name,
                                                                    }
                                                                )}
                                                                disabled={checking === account.id}
                                                                onClick={() => check(account)}
                                                            >
                                                                <SyncIcon />
                                                            </IconButton>
                                                        </span>
                                                    </Tooltip>
                                                    <Tooltip
                                                        title={t("messagingAccounts.action.test")}
                                                    >
                                                        <IconButton
                                                            aria-label={t(
                                                                "messagingAccounts.action.testNamed",
                                                                {
                                                                    name: account.name,
                                                                }
                                                            )}
                                                            onClick={() => setTesting(account)}
                                                        >
                                                            <SendIcon />
                                                        </IconButton>
                                                    </Tooltip>
                                                    <Tooltip
                                                        title={t("messagingAccounts.action.delete")}
                                                    >
                                                        <IconButton
                                                            aria-label={t(
                                                                "messagingAccounts.action.deleteNamed",
                                                                {
                                                                    name: account.name,
                                                                }
                                                            )}
                                                            onClick={() => setDeleting(account)}
                                                        >
                                                            <DeleteIcon />
                                                        </IconButton>
                                                    </Tooltip>
                                                </>
                                            )}
                                        </TableCell>
                                    </TableRow>
                                )
                            })}
                        </TableBody>
                    </Table>
                </TableContainer>
            )}

            <Drawer
                anchor="right"
                open={drawer.kind !== "CLOSED"}
                onClose={() => setDrawer({kind: "CLOSED"})}
                slotProps={{
                    paper: {
                        "aria-labelledby": MESSAGING_ACCOUNT_EDITOR_TITLE,
                        "sx": {width: {xs: "100%", md: "44%"}, minWidth: {md: 560}},
                    },
                }}
            >
                {drawer.kind !== "CLOSED" && (
                    <MessagingAccountEditor
                        key={drawer.kind === "EDIT" ? drawer.account.id : "new"}
                        account={drawer.kind === "EDIT" ? drawer.account : undefined}
                        canWrite={canWrite}
                        onChanged={reload}
                        onClose={() => setDrawer({kind: "CLOSED"})}
                    />
                )}
            </Drawer>

            {testing && (
                <MessagingTestDialog
                    account={testing}
                    languages={languages}
                    onClose={() => setTesting(null)}
                />
            )}

            <Dialog
                variant="warning"
                open={!!deleting}
                ok={String(t("common.label.delete"))}
                cancel={String(t("common.label.cancel"))}
                title={String(t("messagingAccounts.delete.title"))}
                handleClose={(confirmed: boolean) => {
                    if (confirmed && deleting) {
                        remove(deleting)
                    }
                    setDeleting(null)
                }}
            >
                {t("messagingAccounts.delete.body", {name: deleting?.name ?? ""})}
            </Dialog>
        </Box>
    )
}
