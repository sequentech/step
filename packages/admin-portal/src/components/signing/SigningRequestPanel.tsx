// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useId, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Divider,
    Drawer,
    IconButton,
    LinearProgress,
    List,
    ListItem,
    ListItemIcon,
    ListItemText,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import CloseIcon from "@mui/icons-material/Close"
import DrawIcon from "@mui/icons-material/Draw"
import HourglassEmptyIcon from "@mui/icons-material/HourglassEmpty"
import {AuthContext} from "@/providers/AuthContextProvider"
import {validatePanel, type ISigningApi, type ISigningPanelData} from "@/lib/signing/api"
import {SignBlock, isViewer, signBlock, signerCertificate, signerName} from "@/lib/signing/request"
import {requestStatusKey, shownRequestStatus} from "@/lib/signing/status"
import {SigningRequestStatus} from "@/lib/signing/types"
import {SigningDialog} from "./SigningDialog"
import {SigningHandoverLauncher} from "./SigningHandoverLauncher"
import {SigningSubject} from "./SigningSubject"
import {actionDescription, requestTitle, useSigningFormat} from "./format"
import {useSettlePolling} from "./useSettlePolling"
import {useSigningPermissions} from "./useSigningPermissions"

type ChipColor = "warning" | "success" | "default" | "error"

const STATUS_COLOR: Record<SigningRequestStatus, ChipColor> = {
    [SigningRequestStatus.Waiting]: "warning",
    [SigningRequestStatus.Completed]: "success",
    [SigningRequestStatus.Executed]: "success",
    [SigningRequestStatus.Cancelled]: "default",
    [SigningRequestStatus.Expired]: "default",
    [SigningRequestStatus.Failed]: "error",
}

export interface ISigningRequestPanelProps {
    requestId: string
    api: ISigningApi
    open: boolean
    onClose: () => void
    /** Opens the signing dialog once the request has loaded, if the viewer can sign it. */
    autoSign?: boolean
    /** What a completed request offers (download, print, transmit…). */
    completionActions?: (data: ISigningPanelData) => React.ReactNode
    /** After each change the viewer made (a signature, a cancellation). */
    onChange?: (data: ISigningPanelData) => void
    /** Where a handover leaves its note; session storage by default. */
    storage?: Storage
    /** A request of another election event than this is closed instead of shown (handover notes). */
    expectedEventId?: string
}

const CancelRequestDialog: React.FC<{
    open: boolean
    onClose: () => void
    onConfirm: (reason: string) => Promise<void>
}> = ({open, onClose, onConfirm}) => {
    const {t} = useTranslation()
    const titleId = useId()
    const [reason, setReason] = useState("")
    const [busy, setBusy] = useState(false)
    const [failed, setFailed] = useState(false)
    const confirm = async () => {
        setBusy(true)
        setFailed(false)
        try {
            await onConfirm(reason.trim())
            setReason("")
        } catch {
            setFailed(true)
        } finally {
            setBusy(false)
        }
    }
    return (
        <Dialog open={open} onClose={() => !busy && onClose()} aria-labelledby={titleId}>
            <DialogTitle id={titleId}>{t("signing.widget.cancelDialog.title")}</DialogTitle>
            <DialogContent>
                <Typography sx={{mb: 2}}>{t("signing.widget.cancelDialog.body")}</Typography>
                <TextField
                    label={t("signing.widget.cancelDialog.reason")}
                    value={reason}
                    onChange={(event) => setReason(event.target.value)}
                    fullWidth
                    multiline
                    minRows={2}
                />
                {failed ? (
                    <Alert severity="error" sx={{mt: 2}}>
                        {t("signing.widget.cancelDialog.error")}
                    </Alert>
                ) : null}
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose} disabled={busy}>
                    {t("signing.widget.cancelDialog.back")}
                </Button>
                <Button
                    variant="contained"
                    color="error"
                    disabled={busy}
                    onClick={() => {
                        confirm().catch(() => undefined)
                    }}
                >
                    {t("signing.widget.cancelDialog.confirm")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}

const SignerList: React.FC<{data: ISigningPanelData}> = ({data}) => {
    const {t} = useTranslation()
    const format = useSigningFormat(data.time_zone)
    const {userId} = useContext(AuthContext)
    return (
        <List disablePadding aria-label={t("signing.panel.signers")}>
            {data.signers.map((signer) => (
                <ListItem key={signer.user_id} divider disableGutters data-signer={signer.username}>
                    <ListItemIcon sx={{minWidth: 36}}>
                        {signer.signed_at ? (
                            <CheckCircleIcon color="success" aria-hidden />
                        ) : (
                            <HourglassEmptyIcon color="action" aria-hidden />
                        )}
                    </ListItemIcon>
                    <ListItemText
                        sx={{flex: "1 1 50%"}}
                        primary={
                            isViewer(signer, userId)
                                ? `${signerName(signer)} ${t("signing.panel.you")}`
                                : signerName(signer)
                        }
                        secondary={signer.title}
                        slotProps={{primary: {sx: {fontWeight: 600}}}}
                    />
                    <ListItemText
                        sx={{flex: "1 1 50%"}}
                        primary={
                            signer.signed_at
                                ? t("signing.panel.signedAt", {
                                      time: format.time(signer.signed_at),
                                  })
                                : t("signing.panel.notSigned")
                        }
                        secondary={
                            signer.signed_at && signerCertificate(signer)
                                ? t("signing.panel.certificate", {
                                      name: signerCertificate(signer),
                                  })
                                : null
                        }
                    />
                </ListItem>
            ))}
        </List>
    )
}

const StatusLine: React.FC<{data: ISigningPanelData}> = ({data}) => {
    const {t} = useTranslation()
    const format = useSigningFormat(data.time_zone)
    const {request, count} = data
    const counted = request.status !== SigningRequestStatus.Cancelled
    // Named as the Signatures tab's Requests list names it.
    const shown = shownRequestStatus(request, new Date())
    const status = t(requestStatusKey(shown))
    const label = counted
        ? `${status} · ${t("signing.panel.progress", {count, total: request.required})}`
        : status
    const note =
        request.status === SigningRequestStatus.Waiting && request.expires_at
            ? t("signing.panel.expiresAt", {time: format.time(request.expires_at)})
            : (request.status === SigningRequestStatus.Completed ||
                    request.status === SigningRequestStatus.Executed) &&
                request.completed_at
              ? t("signing.widget.panel.completedAt", {
                    time: format.time(request.completed_at),
                })
              : request.status === SigningRequestStatus.Cancelled && request.cancel_reason
                ? t(`signing.cancelReasons.${request.cancel_reason}`)
                : null
    return (
        <Stack direction="row" spacing={1.5} sx={{alignItems: "center", flexWrap: "wrap"}}>
            <Chip
                label={label}
                color={STATUS_COLOR[shown]}
                variant={shown === SigningRequestStatus.Waiting ? "outlined" : "filled"}
                size="small"
                data-testid="signing-status"
                // The outlined warning colour is too light for text; the border carries it.
                sx={shown === SigningRequestStatus.Waiting ? {color: "text.primary"} : undefined}
            />
            {note ? <Typography color="text.secondary">{note}</Typography> : null}
        </Stack>
    )
}

/**
 * A signing request in a right-hand drawer: its status and progress, the rule,
 * what is signed, the signing code and the signers, with Sign, Next member
 * signs in and Cancel request while it waits.
 */
export const SigningRequestPanel: React.FC<ISigningRequestPanelProps> = ({
    requestId,
    api,
    open,
    onClose,
    autoSign = false,
    completionActions,
    onChange,
    storage,
    expectedEventId,
}) => {
    const {t} = useTranslation()
    const {userId} = useContext(AuthContext)
    const permissions = useSigningPermissions()
    const titleId = useId()
    const [data, setData] = useState<ISigningPanelData | null>(null)
    const [loadFailed, setLoadFailed] = useState(false)
    const [dialogOpen, setDialogOpen] = useState(false)
    const [cancelOpen, setCancelOpen] = useState(false)
    const autoSignDone = useRef(false)

    const load = useCallback(async () => {
        setLoadFailed(false)
        try {
            const loaded = validatePanel(await api.getRequest(requestId))
            setData(loaded)
            return loaded
        } catch {
            setLoadFailed(true)
            return null
        }
    }, [api, requestId])

    useEffect(() => {
        setData(null)
        setDialogOpen(false)
        autoSignDone.current = false
        if (open) {
            load().catch(() => undefined)
        }
    }, [open, load])

    const block = data
        ? signBlock(data, userId, permissions.canSign(data.request.action))
        : SignBlock.Closed

    useEffect(() => {
        if (!data || autoSignDone.current) return
        autoSignDone.current = true
        if (expectedEventId && data.request.election_event_id !== expectedEventId) {
            onClose()
            return
        }
        if (autoSign && block === SignBlock.None) setDialogOpen(true)
    }, [autoSign, data, block, expectedEventId, onClose])

    const refresh = async () => {
        const loaded = await load()
        if (loaded) onChange?.(loaded)
    }

    // The last signature dispatched the action: follow it until it ran or failed.
    useSettlePolling(data?.request.status, open, async () => {
        const loaded = await load()
        if (loaded && loaded.request.status !== SigningRequestStatus.Completed) {
            onChange?.(loaded)
        }
    })

    const request = data?.request
    const waiting = request?.status === SigningRequestStatus.Waiting
    const isRequester = !!request && request.requested_by === userId
    const canHandOver = waiting && !!request && (isRequester || permissions.canSign(request.action))
    const canCancel = waiting && (isRequester || permissions.canCancel)
    const completed =
        request?.status === SigningRequestStatus.Completed ||
        request?.status === SigningRequestStatus.Executed

    return (
        <Drawer
            anchor="right"
            open={open}
            onClose={onClose}
            sx={{
                "& .MuiDrawer-paper": {
                    width: {xs: "100%", sm: 640},
                    maxWidth: "100%",
                    boxSizing: "border-box",
                },
            }}
            slotProps={{
                paper: {
                    "aria-labelledby": titleId,
                    "sx": {p: {xs: 2, sm: 3}},
                },
            }}
        >
            <Stack direction="row" sx={{alignItems: "flex-start", gap: 1}}>
                <Box sx={{flex: 1, minWidth: 0}}>
                    <Typography id={titleId} variant="h5" component="h2">
                        {data ? requestTitle(t, data) : t("signing.widget.loading")}
                    </Typography>
                    {data ? (
                        <Typography color="text.secondary">
                            {actionDescription(t, data.request.action, data.seals_ballots)}
                        </Typography>
                    ) : null}
                </Box>
                <IconButton onClick={onClose} aria-label={t("signing.widget.panel.close")}>
                    <CloseIcon />
                </IconButton>
            </Stack>

            {!data ? (
                loadFailed ? (
                    <Alert
                        severity="error"
                        sx={{mt: 3}}
                        action={
                            <Button
                                color="inherit"
                                size="small"
                                onClick={() => {
                                    load().catch(() => undefined)
                                }}
                            >
                                {t("signing.widget.retry")}
                            </Button>
                        }
                    >
                        {t("signing.widget.loadError")}
                    </Alert>
                ) : (
                    <Box sx={{display: "flex", justifyContent: "center", mt: 6}}>
                        <CircularProgress aria-label={t("signing.widget.loading")} />
                    </Box>
                )
            ) : (
                <Stack spacing={3} sx={{mt: 3}}>
                    <StatusLine data={data} />
                    {data.request.status !== SigningRequestStatus.Cancelled ? (
                        <Stack direction="row" spacing={2} sx={{alignItems: "center"}}>
                            <LinearProgress
                                variant="determinate"
                                value={Math.min(
                                    100,
                                    (100 * data.count) / Math.max(1, data.request.required)
                                )}
                                color={completed ? "success" : "primary"}
                                sx={{flex: 1, height: 6, borderRadius: 3}}
                                aria-label={t("signing.panel.progress", {
                                    count: data.count,
                                    total: data.request.required,
                                })}
                            />
                            <Typography sx={{fontWeight: 700}}>
                                {t("signing.panel.progress", {
                                    count: data.count,
                                    total: data.request.required,
                                })}
                            </Typography>
                        </Stack>
                    ) : null}
                    <Typography>
                        {data.election_name
                            ? t("signing.panel.rulePost", {
                                  n: data.request.required,
                                  post: data.election_name,
                              })
                            : t("signing.panel.ruleEvent", {n: data.request.required})}
                    </Typography>
                    <SigningSubject data={data} api={api} />
                    <Divider />
                    <Box>
                        <Typography
                            variant="subtitle1"
                            component="h3"
                            sx={{fontWeight: 700, mb: 1}}
                        >
                            {t("signing.panel.signers")}
                        </Typography>
                        <SignerList data={data} />
                    </Box>
                    {data.request.status === SigningRequestStatus.Cancelled ? (
                        <Alert severity="warning">
                            {t("signing.dialog.problems.cancelled", {
                                reason: data.request.cancel_reason
                                    ? t(`signing.cancelReasons.${data.request.cancel_reason}`)
                                    : t("signing.status.cancelled"),
                            })}
                        </Alert>
                    ) : data.request.status === SigningRequestStatus.Expired ? (
                        <Alert severity="info">{t("signing.widget.panel.expired")}</Alert>
                    ) : data.request.status === SigningRequestStatus.Failed ? (
                        <Alert severity="error">{t("signing.widget.panel.failed")}</Alert>
                    ) : null}
                    {waiting ? (
                        <Stack direction="row" spacing={1.5} sx={{flexWrap: "wrap", rowGap: 1.5}}>
                            {block === SignBlock.None ? (
                                <Button
                                    variant="contained"
                                    startIcon={<DrawIcon />}
                                    onClick={() => setDialogOpen(true)}
                                >
                                    {t("signing.panel.sign")}
                                </Button>
                            ) : null}
                            {canHandOver ? (
                                <SigningHandoverLauncher data={data} api={api} storage={storage} />
                            ) : null}
                            {canCancel ? (
                                <Button color="inherit" onClick={() => setCancelOpen(true)}>
                                    {t("signing.panel.cancel")}
                                </Button>
                            ) : null}
                        </Stack>
                    ) : null}
                    {completed && completionActions ? (
                        <Stack direction="row" spacing={1.5} sx={{flexWrap: "wrap", rowGap: 1.5}}>
                            {completionActions(data)}
                        </Stack>
                    ) : null}
                    <SigningDialog
                        open={dialogOpen}
                        data={data}
                        api={api}
                        onClose={() => setDialogOpen(false)}
                        onSigned={() => {
                            refresh().catch(() => undefined)
                        }}
                        onRequestChanged={() => {
                            refresh().catch(() => undefined)
                        }}
                    />
                    <CancelRequestDialog
                        open={cancelOpen}
                        onClose={() => setCancelOpen(false)}
                        onConfirm={async (reason) => {
                            await api.cancel(data.request.id, reason ? {reason} : {})
                            setCancelOpen(false)
                            await refresh()
                        }}
                    />
                </Stack>
            )}
        </Drawer>
    )
}
