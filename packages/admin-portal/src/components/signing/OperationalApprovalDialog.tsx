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
import {
    approvalControl,
    reviewedInitializePost,
    signingExperience,
} from "@/lib/signing/operationalApprovalPolicy"
import {matchLocalInitializeSettings} from "@/lib/signing/localInitializeAuthority"
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

type Phase =
    | ApprovalPhase
    | "loading"
    | "setup-required"
    | "setup-pending"
    | "setup-retry-available"
    | "setup-recovery-required"
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
    const request = data.request
    const location = window.location
    const registration = (globalSettings as {FELIX3_LOCAL_INITIALIZE?: unknown}).FELIX3_LOCAL_INITIALIZE
    const localOrigin =
        location.protocol === "http:" &&
        location.href.startsWith(location.origin + "/") &&
        matchLocalInitializeSettings(location.origin, request.tenant_id, globalSettings)
            ? location.origin
            : undefined
    const client = useMemo(
        () => createOperationalApprovalApi(apollo, localOrigin, registration),
        [apollo, localOrigin, registration]
    )
    const scope = useMemo(
        () => ({
            tenant: request.tenant_id,
            event: request.election_event_id,
            request: request.id,
            official: userId,
        }),
        [request.tenant_id, request.election_event_id, request.id, userId]
    )
    const usable =
        (location.protocol === "https:" || localOrigin !== undefined) &&
        isAuthenticated &&
        tenantId === scope.tenant &&
        Object.values(scope).every(isId)
    const view = useMemo(
        () => ({}),
        [scope, usable, localOrigin, registration, request.election_id, request.payload_sha256]
    )
    const reviewed = useSignedView(data)
    const canApproveReviewed =
        reviewed.verified && (!localOrigin || reviewedInitializePost(request, reviewed.view))
    const titleId = useId()
    const [phase, setPhase] = useState<Phase>("loading")
    const [cancellableAttempt, setCancellableAttempt] = useState<string>()
    const [confirmCancel, setConfirmCancel] = useState<string>()
    const [device, setDevice] = useState<{view: object; softwareDemo: boolean} | null>(null)
    const [activeView, setActiveView] = useState<object | null>(null)
    const busy = activeView === view
    const softwareDemo = usable && device?.view === view && device.softwareDemo
    const [failed, setFailed] = useState(false)
    const epoch = useRef(0)
    const operation = useRef<{view: object} | null>(null)
    const latest = useRef({view, usable, eligible, data, onRefresh})
    latest.current = {view, usable, eligible, data, onRefresh}
    const reported = useRef(false)

    const read = useCallback(async () => {
        const ticket = ++epoch.current
        if (!usable) {
            setDevice(null)
            setPhase("unavailable")
            return
        }
        try {
            const status = await client.status(scope)
            let current: Phase = status.phase
            if (
                (current === "unavailable" || current === "challenge-available") &&
                !localOrigin &&
                latest.current.eligible
            ) {
                const setupStatus = await client.setupStatus(scope)
                if (setupStatus === "setup-available") current = "setup-required"
                if (
                    setupStatus === "setup-pending" ||
                    setupStatus === "setup-retry-available" ||
                    setupStatus === "setup-recovery-required"
                )
                    current = setupStatus
            }
            if (ticket !== epoch.current || latest.current.view !== view) return
            setPhase(current)
            setCancellableAttempt(status.cancellableAttempt)
            setConfirmCancel((confirmed) =>
                confirmed === status.cancellableAttempt ? confirmed : undefined
            )
            setDevice(current === "unavailable" ? null : {view, softwareDemo: status.softwareDemo})
            if (current === "approval-recorded" && !reported.current) {
                reported.current = true
                latest.current.onRefresh()
            }
            return current
        } catch {
            if (ticket === epoch.current && latest.current.view === view) {
                setDevice(null)
                setPhase("unavailable")
                setCancellableAttempt(undefined)
                setConfirmCancel(undefined)
            }
        }
    }, [client, scope, usable, view, localOrigin])

    useEffect(() => {
        reported.current = false
        setDevice(null)
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
                if (!active || operation.current?.view === view) return
                const next = await read()
                if (active && next && pending(next) && ++checks < 12) schedule()
            }, 2500)
        }
        schedule()
        return () => {
            active = false
            window.clearTimeout(timer)
        }
    }, [phase, busy, read, view])

    const run = async (action: "set-up" | "approve" | "check" | "cancel-unused") => {
        if (operation.current?.view === view || !usable) return
        if (action === "approve" && !canApproveReviewed) return
        const owner = {view}
        operation.current = owner
        const ticket = ++epoch.current
        const path = window.location.pathname
        const search = window.location.search
        const current = () =>
            operation.current === owner &&
            epoch.current === ticket &&
            latest.current.view === view &&
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

        setActiveView(view)
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
                    (localOrigin && !reviewedInitializePost(next, reviewed.view)) ||
                    next.payload_sha256 !== request.payload_sha256 ||
                    next.canonical_payload !== request.canonical_payload ||
                    signBlock(fresh, scope.official, true) !== SignBlock.None
                ) {
                    throw new Error("Request no longer open or does not match")
                }
                if (action === "set-up") {
                    if (localOrigin) return
                    const setup = await client.setupStatus(scope)
                    if (
                        !stillReviewed() ||
                        (phase === "setup-retry-available"
                            ? setup !== "setup-retry-available"
                            : setup !== "setup-available")
                    ) {
                        throw new Error("Signing setup state changed")
                    }
                    if (!isGoldUser()) throw new SetupGoldRequired()
                    await client.setup(scope)
                } else {
                    const ready = localOrigin
                        ? (await client.status(scope)).phase === "challenge-available"
                        : (await client.setupStatus(scope)) === "certificate-on-file"
                    if (!stillReviewed() || !ready) throw new Error("Signing unavailable")
                    const page = originPage()
                    const begin = await client.start(scope, page, request.payload_sha256)
                    if (!stillReviewed()) return
                    const redirect = trustedSigningRedirect(
                        begin.redirect,
                        globalSettings.KEYCLOAK_URL,
                        scope.tenant,
                        window.location.origin,
                        registration
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
                    if (current()) setFailed(true)
                }
            } else if (current()) {
                setFailed(true)
            }
        } finally {
            const refresh = !leaving && current()
            // An abandoned earlier context cannot clear the lock or refresh the newer one.
            if (operation.current === owner && !leaving) {
                operation.current = null
                setActiveView((active) => (active === view ? null : active))
            }
            // Never retry a mutation automatically; ask only for fresh status.
            if (refresh) await read()
        }
    }

    const scopeHasStatus = device?.view === view
    const control =
        eligible && usable && scopeHasStatus ? approvalControl(phase, canApproveReviewed) : null
    const canCancelUnused =
        usable &&
        eligible &&
        scopeHasStatus &&
        phase === "waiting-for-identity" &&
        Boolean(cancellableAttempt)
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
                    {!localOrigin && (
                        <Alert severity="info">{t("signing.operational.isolatedDemo")}</Alert>
                    )}
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
                                ? phase === "setup-retry-available"
                                    ? "signing.operational.retrySetup"
                                    : "signing.operational.setup"
                                : "signing.operational.approve"
                        )}
                    </Button>
                )}
            </DialogActions>
        </Dialog>
    )
}
