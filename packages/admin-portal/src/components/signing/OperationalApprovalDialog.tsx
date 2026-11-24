// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useId, useMemo, useRef, useState} from "react"
import {useApolloClient} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Stack,
    Typography,
} from "@mui/material"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {validatePanel, type ISigningApi, type ISigningPanelData} from "@/lib/signing/api"
import {
    createOperationalApprovalApi,
    SetupGoldRequired,
    trustedSigningRedirect,
    type ApprovalPhase,
} from "@/lib/signing/operationalApprovalApi"
import {approvalControl, signingExperience} from "@/lib/signing/operationalApprovalPolicy"
import {
    isId,
    markBeforeFullRedirect,
    observedPage,
    saveDisplayReturn,
    signingReturnPath,
    type IReturnPage,
} from "@/lib/signing/operationalGoldReturn"
import {signBlock, SignBlock} from "@/lib/signing/request"
import {SigningSubject, SigningSubjectVariant} from "./SigningSubject"
import {useSignedView} from "./useSignedView"

type Phase = ApprovalPhase | "loading" | "setup-required" | "setup-pending"
const pending = (phase: Phase) =>
    phase === "pending" || phase === "waiting-for-identity" || phase === "setup-pending"

interface Props {
    data: ISigningPanelData
    api: ISigningApi
    onClose: () => void
    onRefresh: () => void
    eligible: boolean
}

export const OperationalApprovalDialog: React.FC<Props> = ({
    data,
    api,
    eligible,
    onClose,
    onRefresh,
}) => {
    const {t} = useTranslation()
    const {isAuthenticated, tenantId, userId, isGoldUser, reauthWithGold} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const apollo = useApolloClient()
    const client = useMemo(() => createOperationalApprovalApi(apollo), [apollo])
    const request = data.request
    const scope = useMemo(
        () => ({
            tenant: request.tenant_id,
            event: request.election_event_id,
            request: request.id,
            official: userId,
        }),
        [request.tenant_id, request.election_event_id, request.id, userId]
    )
    const usable = isAuthenticated && tenantId === scope.tenant && Object.values(scope).every(isId)
    const reviewed = useSignedView(data)
    const titleId = useId()
    const [phase, setPhase] = useState<Phase>("loading")
    const [cancellableAttempt, setCancellableAttempt] = useState<string>()
    const [confirmCancel, setConfirmCancel] = useState<string>()
    const [softwareDemo, setSoftwareDemo] = useState(false)
    const [busy, setBusy] = useState(false)
    const [failed, setFailed] = useState(false)
    const epoch = useRef(0)
    const operation = useRef(false)
    const latest = useRef({scope, usable, eligible, data, onRefresh})
    latest.current = {scope, usable, eligible, data, onRefresh}
    const reported = useRef(false)

    const read = useCallback(async () => {
        const ticket = ++epoch.current
        if (!usable) {
            setPhase("unavailable")
            return
        }
        try {
            const status = await client.status(scope)
            let current: Phase = status.phase
            if (current === "unavailable" && latest.current.eligible) {
                const setupStatus = await client.setupStatus(scope)
                if (setupStatus === "setup-available") current = "setup-required"
                if (setupStatus === "setup-pending") {
                    current = "setup-pending"
                }
            }
            if (ticket !== epoch.current || latest.current.scope !== scope) return
            setPhase(current)
            setCancellableAttempt(status.cancellableAttempt)
            setConfirmCancel((confirmed) =>
                confirmed === status.cancellableAttempt ? confirmed : undefined
            )
            setSoftwareDemo(status.softwareDemo)
            if (current === "approval-recorded" && !reported.current) {
                reported.current = true
                latest.current.onRefresh()
            }
            return current
        } catch {
            if (ticket === epoch.current) {
                setPhase("unavailable")
                setCancellableAttempt(undefined)
                setConfirmCancel(undefined)
            }
        }
    }, [client, scope, usable])

    useEffect(() => {
        reported.current = false
        setFailed(false)
        setPhase("loading")
        setCancellableAttempt(undefined)
        setConfirmCancel(undefined)
        void read()
        return () => {
            epoch.current++
        }
    }, [read])

    useEffect(() => {
        if (!pending(phase) || busy) return
        let active = true
        let checks = 0
        let timer: number
        const schedule = () => {
            timer = window.setTimeout(async () => {
                if (!active || operation.current) return
                const next = await read()
                if (active && next && pending(next) && ++checks < 12) schedule()
            }, 2500)
        }
        schedule()
        return () => {
            active = false
            window.clearTimeout(timer)
        }
    }, [phase, busy, read])

    const run = async (action: "set-up" | "approve" | "check" | "cancel-unused") => {
        if (operation.current || !usable) return
        operation.current = true
        const ticket = ++epoch.current
        const path = window.location.pathname
        const search = window.location.search
        const current = () =>
            epoch.current === ticket &&
            latest.current.scope === scope &&
            latest.current.usable &&
            window.location.pathname === path &&
            window.location.search === search
        const stillReviewed = () =>
            current() &&
            latest.current.eligible &&
            latest.current.data.request.payload_sha256 === request.payload_sha256 &&
            latest.current.data.request.canonical_payload === request.canonical_payload &&
            latest.current.data.request.election_id === request.election_id
        const originPage = (): IReturnPage =>
            (request.election_id
                ? observedPage(path, scope.event, "publish", request.election_id)
                : undefined) ??
            observedPage(path, scope.event, "publish") ?? {
                path: "event-show",
                panel: "signatures-requests",
            }

        setBusy(true)
        setFailed(false)
        let leaving = false
        try {
            if (action === "check") {
                if (phase === "reconcile" || phase === "pending") await client.reconcile(scope)
            } else if (action === "cancel-unused") {
                const confirmed = confirmCancel
                setConfirmCancel(undefined)
                if (!confirmed || confirmed !== cancellableAttempt || !latest.current.eligible)
                    return
                // A start response may have been lost. Cancel only the same still-unused reference;
                // authoritative status and the cancellation transaction both recheck it.
                const status = await client.status(scope)
                if (
                    !current() ||
                    !latest.current.eligible ||
                    status.phase !== "waiting-for-identity" ||
                    status.cancellableAttempt !== confirmed
                ) {
                    throw new Error("The signing challenge changed")
                }
                await client.cancelUnused(scope, confirmed)
            } else if (stillReviewed()) {
                const fresh = validatePanel(await api.getRequest(scope.request))
                const next = fresh.request
                if (
                    !stillReviewed() ||
                    signingExperience(fresh.operational_signing_mode, next.action) !== "managed" ||
                    next.id !== scope.request ||
                    next.tenant_id !== scope.tenant ||
                    next.election_event_id !== scope.event ||
                    next.election_id !== request.election_id ||
                    next.payload_sha256 !== request.payload_sha256 ||
                    next.canonical_payload !== request.canonical_payload ||
                    signBlock(fresh, scope.official, true) !== SignBlock.None
                ) {
                    throw new Error("Request no longer open or does not match")
                }
                if (action === "set-up") {
                    if (!isGoldUser()) throw new SetupGoldRequired()
                    await client.setup(scope)
                } else {
                    const page = originPage()
                    if (window.location.protocol !== "https:")
                        throw new Error("Secure context required")
                    const begin = await client.start(scope, page, request.payload_sha256)
                    if (!stillReviewed()) return
                    const redirect = trustedSigningRedirect(
                        begin.redirect,
                        globalSettings.KEYCLOAK_URL,
                        scope.tenant
                    )
                    if (
                        !markBeforeFullRedirect(window.sessionStorage, {
                            ...scope,
                            page,
                            attempt: begin.attempt,
                        })
                    ) {
                        throw new Error("Signing return unavailable")
                    }
                    window.location.replace(redirect)
                    leaving = true
                }
            }
        } catch (error) {
            if (
                current() &&
                stillReviewed() &&
                action === "set-up" &&
                error instanceof SetupGoldRequired
            ) {
                try {
                    const page = originPage()
                    const returnPath = signingReturnPath(scope.event, page)
                    saveDisplayReturn(window.sessionStorage, {
                        marker: {...scope, page, attempt: scope.request},
                        result: "setup-returned",
                        path: returnPath,
                    })
                    await reauthWithGold(window.location.origin + returnPath, true)
                } catch {
                    setFailed(true)
                }
            } else if (current()) {
                setFailed(true)
            }
        } finally {
            operation.current = false
            // Never retry a mutation automatically; ask only for fresh status.
            if (!leaving && current()) {
                setBusy(false)
                await read()
            }
        }
    }

    const control = eligible && usable ? approvalControl(phase, reviewed.verified) : null
    const canCancelUnused =
        usable && eligible && phase === "waiting-for-identity" && Boolean(cancellableAttempt)
    return (
        <Dialog
            open
            onClose={busy ? undefined : onClose}
            fullWidth
            maxWidth="sm"
            aria-labelledby={titleId}
        >
            <DialogTitle id={titleId}>{t("signing.operational.title")}</DialogTitle>
            <DialogContent>
                <Stack spacing={2}>
                    <SigningSubject data={data} api={api} variant={SigningSubjectVariant.Dialog} />
                    {softwareDemo && (
                        <Alert severity="warning">{t("signing.operational.softwareDemo")}</Alert>
                    )}
                    <Typography role="status" aria-live="polite">
                        {t(`signing.operational.phase.${phase}`)}
                    </Typography>
                    <Typography variant="body2">
                        {t("signing.operational.quorum", {
                            count: data.count,
                            required: request.required,
                        })}
                    </Typography>
                    {canCancelUnused && confirmCancel && (
                        <Alert severity="warning">
                            {t("signing.operational.confirmCancelUnused")}
                        </Alert>
                    )}
                    {failed && <Alert severity="error">{t("signing.operational.error")}</Alert>}
                </Stack>
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose} disabled={busy}>
                    {t("signing.widget.close")}
                </Button>
                {canCancelUnused &&
                    (confirmCancel ? (
                        <>
                            <Button disabled={busy} onClick={() => setConfirmCancel(undefined)}>
                                {t("signing.operational.keepChallenge")}
                            </Button>
                            <Button
                                color="warning"
                                disabled={busy}
                                onClick={() => void run("cancel-unused")}
                            >
                                {t("signing.operational.confirmDiscard")}
                            </Button>
                        </>
                    ) : (
                        <Button
                            disabled={busy}
                            onClick={() => setConfirmCancel(cancellableAttempt)}
                        >
                            {t("signing.operational.cancelUnused")}
                        </Button>
                    ))}
                {!control &&
                    phase !== "loading" &&
                    phase !== "approval-recorded" &&
                    !confirmCancel && (
                        <Button disabled={busy} onClick={() => void run("check")}>
                            {t("signing.operational.checkStatus")}
                        </Button>
                    )}
                {control && (
                    <Button variant="contained" disabled={busy} onClick={() => void run(control)}>
                        {t(
                            control === "set-up"
                                ? "signing.operational.setup"
                                : "signing.operational.approve"
                        )}
                    </Button>
                )}
            </DialogActions>
        </Dialog>
    )
}
