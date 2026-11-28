// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Settings → Configuration package → Danger zone: returns the tenant to what
// it was before its first configuration package, so the package and its
// roster can be imported again. The server checks everything again; this
// card only shows what it said and asks for the typed confirmation.

import React, {useContext, useEffect, useId, useState} from "react"
import {useTranslation} from "react-i18next"
import {useLazyQuery, useMutation, useQuery} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Paper,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {RESET_TENANT, TENANT_RESET_PREVIEW} from "@/queries/TenantReset"
import {GET_PACKAGE_TASK} from "@/queries/ApplyConfigurationPackage"

const ROLE = {headers: {"x-hasura-role": IPermissions.TENANT_RESET}}
const POLL_INTERVAL_MS = 3000

export type Refusal =
    | {kind: "disabled"}
    | {kind: "super_admin_tenant"}
    | {kind: "no_baseline"}
    | {kind: "nothing_to_reset"}
    | {kind: "task_running"; task_id: string; name: string}
    | {kind: "voting_open"; election_event_id: string}
    | {kind: "locked_down"; election_event_id: string}
    | {kind: "ballots"; cast: number; sealed: number; overridable: boolean}

export interface TenantResetPreview {
    enabled: boolean
    has_baseline: boolean
    baseline_captured_at: string | null
    resetting: boolean
    election_events: {id: string; name: string | null}[]
    rows: Record<string, number>
    cast_ballots: number
    sealed_ballots: number
    refusals: Refusal[]
    erase_ballots_phrase: string
    slug: string
    fresh_login_required: boolean
}

/** Refusals that still stop a reset the admin confirmed. */
export const blockingRefusals = (refusals: Refusal[], eraseBallots: boolean): Refusal[] =>
    refusals.filter((r) => !(r.kind === "ballots" && r.overridable && eraseBallots))

const totalRows = (rows: Record<string, number>): number =>
    Object.values(rows).reduce((sum, n) => sum + n, 0)

type Phase = "idle" | "submitting" | "running" | "succeeded" | "failed" | "unknown"

interface TenantResetCardProps {
    tenantId: string | null
    /** Called once a reset has run, so the page drops what it deleted. */
    onReset?: () => void
}

export const TenantResetCard: React.FC<TenantResetCardProps> = ({tenantId, onReset}) => {
    const auth = useContext(AuthContext)
    const allowed = !!tenantId && auth.isAuthorized(true, tenantId, IPermissions.TENANT_RESET)
    return allowed && tenantId ? <TenantResetCardFor tenantId={tenantId} onReset={onReset} /> : null
}

const TenantResetCardFor: React.FC<{tenantId: string; onReset?: () => void}> = ({
    tenantId,
    onReset,
}) => {
    const {t} = useTranslation()
    const auth = useContext(AuthContext)
    const titleId = useId()
    const [open, setOpen] = useState(false)
    const [confirmation, setConfirmation] = useState("")
    const [phrase, setPhrase] = useState("")
    const [phase, setPhase] = useState<Phase>("idle")
    const [taskId, setTaskId] = useState<string | null>(null)
    const [error, setError] = useState<string | null>(null)

    const [loadPreview, {data, loading, error: previewError, refetch}] = useLazyQuery<{
        tenant_reset_preview: TenantResetPreview
    }>(TENANT_RESET_PREVIEW, {fetchPolicy: "network-only", context: ROLE})
    const [reset] = useMutation<{reset_tenant: {task_execution: {id: string}}}>(RESET_TENANT, {
        context: ROLE,
    })
    const {
        data: taskData,
        error: taskError,
        loading: taskLoading,
        refetch: refetchTask,
    } = useQuery<{
        sequent_backend_tasks_execution: {
            id: string
            execution_status: string
            annotations: unknown
            logs: unknown
        }[]
    }>(GET_PACKAGE_TASK, {
        variables: {taskId, tenantId},
        skip: !taskId || phase !== "running",
        pollInterval: POLL_INTERVAL_MS,
        fetchPolicy: "network-only",
    })

    useEffect(() => {
        loadPreview({variables: {tenant_id: tenantId}}).catch(() => undefined)
    }, [tenantId, loadPreview])

    // Errors show through `previewError`.
    const reloadPreview = () => {
        refetch?.({tenant_id: tenantId})?.catch(() => undefined)
    }

    const rows = taskData?.sequent_backend_tasks_execution
    const task = rows?.[0]
    useEffect(() => {
        if (phase !== "running") return
        if (task?.execution_status === "SUCCESS") {
            setPhase("succeeded")
            reloadPreview()
            onReset?.()
        } else if (task?.execution_status === "FAILED") {
            setPhase("failed")
            reloadPreview()
            // A failed reset may have deleted part of it already.
            onReset?.()
        } else if (!taskLoading && (taskError || (Array.isArray(rows) && rows.length === 0))) {
            // The task can't be read: what happened is unknown.
            setPhase("unknown")
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [phase, task, rows, taskError, taskLoading])

    const watchAgain = () => {
        setPhase("running")
        refetchTask?.()?.catch(() => undefined)
    }

    const preview = data?.tenant_reset_preview
    const ballotRefusal = preview?.refusals.find(
        (r): r is Extract<Refusal, {kind: "ballots"}> => r.kind === "ballots"
    )
    const eraseBallots = !!preview && phrase.trim() === preview.erase_ballots_phrase
    const blocking = preview ? blockingRefusals(preview.refusals, eraseBallots) : []
    const canConfirm =
        !!preview &&
        !preview.fresh_login_required &&
        blocking.length === 0 &&
        confirmation.trim() === preview.slug &&
        phase === "idle"

    const openDialog = () => {
        setConfirmation("")
        setPhrase("")
        setError(null)
        setPhase("idle")
        setTaskId(null)
        setOpen(true)
        reloadPreview()
    }

    const submit = async () => {
        if (!canConfirm) return
        setPhase("submitting")
        setError(null)
        try {
            const result = await reset({
                variables: {
                    tenant_id: tenantId,
                    confirmation: confirmation.trim(),
                    erase_ballots_confirmation: eraseBallots ? phrase.trim() : null,
                },
            })
            const id = result.data?.reset_tenant?.task_execution?.id
            if (!id) throw new Error(String(t("configurationPackage.reset.noTask")))
            setTaskId(id)
            setPhase("running")
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
            setPhase("idle")
            reloadPreview()
        }
    }

    const refusalText = (r: Refusal): string => {
        switch (r.kind) {
            case "task_running":
                return String(t("configurationPackage.reset.refusal.task_running", {name: r.name}))
            case "voting_open":
            case "locked_down": {
                const event = preview?.election_events.find((e) => e.id === r.election_event_id)
                return String(
                    t(`configurationPackage.reset.refusal.${r.kind}`, {
                        event: event?.name ?? r.election_event_id,
                    })
                )
            }
            case "ballots":
                return String(
                    t(
                        r.overridable
                            ? "configurationPackage.reset.refusal.ballots_overridable"
                            : "configurationPackage.reset.refusal.ballots",
                        {cast: r.cast, sealed: r.sealed}
                    )
                )
            default:
                return String(t(`configurationPackage.reset.refusal.${r.kind}`))
        }
    }

    const failureMessage = (): string | undefined => {
        const logs = task?.logs
        if (!Array.isArray(logs)) return undefined
        // Task logs are {created_date, log_text}; the failure is the reset's error.
        const failure = (logs as {log_text?: unknown}[])
            .map((log) => log?.log_text)
            .find((text): text is string => typeof text === "string" && text.startsWith("Error"))
        return failure
    }

    return (
        <Paper
            variant="outlined"
            sx={{p: 2, mt: 3, borderColor: "error.main"}}
            data-testid="tenant-reset-card"
        >
            <Typography variant="h6" component="h3" color="error" sx={{m: 0}}>
                {t("configurationPackage.reset.title")}
            </Typography>
            <Typography variant="body2" sx={{mt: 1}}>
                {t("configurationPackage.reset.explain")}
            </Typography>
            {preview?.resetting && (
                <Alert severity="warning" sx={{mt: 1}} data-testid="tenant-reset-unfinished">
                    {t("configurationPackage.reset.unfinished")}
                </Alert>
            )}
            {preview && !preview.has_baseline && (
                <Typography variant="body2" color="text.secondary" sx={{mt: 1}}>
                    {t("configurationPackage.reset.noBaselineHint")}
                </Typography>
            )}
            <Box sx={{mt: 2}}>
                <Button
                    variant="outlined"
                    color="error"
                    onClick={openDialog}
                    disabled={loading && !preview && !previewError}
                >
                    {preview?.resetting
                        ? t("configurationPackage.reset.finish")
                        : t("configurationPackage.reset.open")}
                </Button>
            </Box>

            <Dialog
                open={open}
                onClose={() => phase !== "submitting" && phase !== "running" && setOpen(false)}
                aria-labelledby={titleId}
                maxWidth="sm"
                fullWidth
            >
                <DialogTitle id={titleId}>
                    {t("configurationPackage.reset.dialogTitle")}
                </DialogTitle>
                <DialogContent>
                    {!preview && previewError && !loading ? (
                        <Alert
                            severity="error"
                            data-testid="tenant-reset-preview-error"
                            action={
                                <Button color="inherit" size="small" onClick={reloadPreview}>
                                    {t("configurationPackage.reset.retry")}
                                </Button>
                            }
                        >
                            {t("configurationPackage.reset.previewFailed")}
                            <Box component="span" sx={{display: "block", mt: 1}}>
                                {previewError.message}
                            </Box>
                        </Alert>
                    ) : phase === "unknown" ? (
                        <Alert
                            severity="warning"
                            data-testid="tenant-reset-unknown"
                            action={
                                <Button color="inherit" size="small" onClick={watchAgain}>
                                    {t("configurationPackage.reset.checkAgain")}
                                </Button>
                            }
                        >
                            {t("configurationPackage.reset.unknown")}
                        </Alert>
                    ) : !preview || (loading && phase === "idle") ? (
                        <Stack direction="row" spacing={1} alignItems="center" role="status">
                            <CircularProgress size={20} />
                            <Typography>{t("configurationPackage.reset.loading")}</Typography>
                        </Stack>
                    ) : phase === "running" || phase === "submitting" ? (
                        <Stack direction="row" spacing={1} alignItems="center" role="status">
                            <CircularProgress size={20} />
                            <Typography>{t("configurationPackage.reset.running")}</Typography>
                        </Stack>
                    ) : phase === "succeeded" ? (
                        <Alert severity="success" data-testid="tenant-reset-succeeded">
                            {t("configurationPackage.reset.succeeded")}
                        </Alert>
                    ) : phase === "failed" ? (
                        <Alert severity="error" data-testid="tenant-reset-failed">
                            {t("configurationPackage.reset.failed")}
                            {failureMessage() && (
                                <Box component="span" sx={{display: "block", mt: 1}}>
                                    {failureMessage()}
                                </Box>
                            )}
                        </Alert>
                    ) : (
                        <Stack spacing={1.5}>
                            <Typography variant="body2">
                                {t("configurationPackage.reset.willDelete", {
                                    events: preview.election_events.length,
                                    rows: totalRows(preview.rows),
                                })}
                            </Typography>
                            {preview.election_events.length > 0 && (
                                <Box component="ul" sx={{m: 0, pl: 2.5}}>
                                    {preview.election_events.map((e) => (
                                        <li key={e.id}>
                                            <Typography variant="body2">
                                                {e.name ?? e.id}
                                            </Typography>
                                        </li>
                                    ))}
                                </Box>
                            )}
                            <Typography variant="body2">
                                {t("configurationPackage.reset.keeps")}
                            </Typography>
                            {blocking.length > 0 && (
                                <Alert severity="error" data-testid="tenant-reset-refusals">
                                    <Box component="ul" sx={{m: 0, pl: 2}}>
                                        {blocking.map((r, i) => (
                                            <li key={i}>{refusalText(r)}</li>
                                        ))}
                                    </Box>
                                </Alert>
                            )}
                            {preview.fresh_login_required && (
                                <Alert
                                    severity="info"
                                    action={
                                        auth.reauthenticate ? (
                                            <Button
                                                color="inherit"
                                                size="small"
                                                onClick={() => void auth.reauthenticate?.()}
                                            >
                                                {t("configurationPackage.reset.signIn")}
                                            </Button>
                                        ) : undefined
                                    }
                                >
                                    {t("configurationPackage.reset.freshLogin")}
                                </Alert>
                            )}
                            {ballotRefusal?.overridable && (
                                <TextField
                                    label={t("configurationPackage.reset.erasePhrase", {
                                        phrase: preview.erase_ballots_phrase,
                                    })}
                                    value={phrase}
                                    onChange={(e) => setPhrase(e.target.value)}
                                    autoComplete="off"
                                    fullWidth
                                    size="small"
                                    inputProps={{"data-testid": "tenant-reset-phrase"}}
                                />
                            )}
                            <TextField
                                label={t("configurationPackage.reset.confirmLabel", {
                                    slug: preview.slug,
                                })}
                                value={confirmation}
                                onChange={(e) => setConfirmation(e.target.value)}
                                autoComplete="off"
                                fullWidth
                                size="small"
                                inputProps={{"data-testid": "tenant-reset-confirmation"}}
                            />
                            {error && (
                                <Alert severity="error" data-testid="tenant-reset-error">
                                    {error}
                                </Alert>
                            )}
                        </Stack>
                    )}
                </DialogContent>
                <DialogActions>
                    <Button
                        onClick={() => setOpen(false)}
                        disabled={phase === "submitting" || phase === "running"}
                    >
                        {phase === "succeeded" || phase === "failed" || phase === "unknown"
                            ? t("configurationPackage.reset.close")
                            : t("configurationPackage.reset.cancel")}
                    </Button>
                    {(phase === "idle" || phase === "submitting") && (
                        <Button
                            color="error"
                            variant="contained"
                            disabled={!canConfirm}
                            onClick={() => void submit()}
                            data-testid="tenant-reset-confirm"
                        >
                            {t("configurationPackage.reset.confirm")}
                        </Button>
                    )}
                </DialogActions>
            </Dialog>
        </Paper>
    )
}
