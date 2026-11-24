// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useId, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {
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
import {isId, observedPage} from "@/lib/signing/operationalGoldReturn"
import {
    productSigningSelected,
    productStatusRequired,
    productGoldRedirect,
    productRequest,
    saveProductTab,
} from "@/lib/signing/productApproval"
import {signBlock, SignBlock, signerOf} from "@/lib/signing/request"
import {SigningSubject, SigningSubjectVariant} from "./SigningSubject"
import {useSignedView} from "./useSignedView"
import {ProductSetupButton} from "./ProductSetupButton"

type Props = {
    data: ISigningPanelData
    api: ISigningApi
    eligible: boolean
    returned: boolean
    onClose: () => void
    onRefresh: () => void
}

export const ProductApprovalDialog: React.FC<Props> = ({
    data,
    api,
    eligible,
    returned,
    onClose,
    onRefresh,
}) => {
    const {t} = useTranslation()
    const {tenantId, userId, isAuthenticated, getAccessToken} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const titleId = useId()
    const request = data.request
    const scope = {
        tenant: request.tenant_id,
        event: request.election_event_id,
        request: request.id,
        actor: userId,
        action: request.action,
    }
    const selection = JSON.stringify([
        tenantId,
        userId,
        request.id,
        request.election_event_id,
        request.payload_sha256,
        request.canonical_payload,
    ])
    const selected = useRef(selection)
    const inFlight = useRef<object | null>(null)
    const latest = useRef({eligible, tenantId, isAuthenticated, data})
    latest.current = {eligible, tenantId, isAuthenticated, data}
    const [activeSelection, setActiveSelection] = useState<string>()
    const busy = activeSelection === selection
    const [uncertainSelection, setUncertainSelection] = useState<string | undefined>(
        returned ? selection : undefined
    )
    const [readySelection, setReadySelection] = useState<string>()
    const uncertain = productStatusRequired(returned, selection, uncertainSelection, readySelection)
    const reviewed = useSignedView(data)
    const goldClient = globalSettings.OPERATIONAL_PRODUCT_SIGNING_GOLD_CLIENT_ID
    const usable =
        isAuthenticated &&
        tenantId === scope.tenant &&
        [scope.actor, scope.tenant, scope.event, scope.request].every(isId) &&
        productSigningSelected(
            data.operational_signing_mode,
            scope.action,
            data.operational_signing_flow
        ) &&
        Boolean(goldClient) &&
        window.location.protocol === "https:"
    const recorded = Boolean(signerOf(data, userId)?.signed_at)

    useEffect(() => {
        selected.current = selection
        return () => {
            inFlight.current = null
        }
    }, [selection])

    async function approve() {
        if (!usable || !eligible || !reviewed.verified || uncertain || recorded || inFlight.current)
            return
        const owner = {}
        inFlight.current = owner
        const path = window.location.pathname
        const current = () =>
            inFlight.current === owner &&
            selected.current === selection &&
            latest.current.eligible &&
            latest.current.isAuthenticated &&
            latest.current.tenantId === scope.tenant &&
            latest.current.data.request.canonical_payload === request.canonical_payload &&
            window.location.pathname === path
        setActiveSelection(selection)
        let leaving = false
        try {
            const fresh = validatePanel(await api.getRequest(scope.request))
            const next = fresh.request
            if (
                !current() ||
                !productSigningSelected(
                    fresh.operational_signing_mode,
                    next.action,
                    fresh.operational_signing_flow
                ) ||
                next.id !== scope.request ||
                next.tenant_id !== scope.tenant ||
                next.election_event_id !== scope.event ||
                next.election_id !== request.election_id ||
                next.action !== request.action ||
                next.payload_sha256 !== request.payload_sha256 ||
                next.canonical_payload !== request.canonical_payload ||
                signBlock(fresh, scope.actor, true) !== SignBlock.None
            )
                throw new Error("Approval request changed")
            const token = getAccessToken()
            if (!token || !goldClient) throw new Error("Approval session unavailable")
            const result = await productRequest(window.location.origin, token, scope, {
                step: "start",
                reviewedPayload: request.payload_sha256,
            })
            if (!current()) return
            if (result.phase !== "gold") {
                setUncertainSelection(selection)
                onRefresh()
                return
            }
            const redirect = productGoldRedirect(
                result.authorization_url,
                globalSettings.KEYCLOAK_URL,
                goldClient,
                scope.tenant
            )
            const page = (request.election_id
                ? observedPage(path, scope.event, "publish", request.election_id)
                : undefined) ??
                observedPage(path, scope.event, "signatures-requests") ?? {
                    path: "event-show" as const,
                    panel: "signatures-requests" as const,
                }
            if (
                !saveProductTab(window.sessionStorage, {
                    marker: {
                        tenant: scope.tenant,
                        event: scope.event,
                        request: scope.request,
                        official: scope.actor,
                        attempt: result.attempt,
                        page,
                    },
                    action: scope.action,
                    turn: "payload",
                })
            )
                throw new Error("Unable to secure the approval return")
            window.location.replace(redirect)
            leaving = true
        } catch {
            if (inFlight.current === owner) setUncertainSelection(selection)
        } finally {
            if (inFlight.current === owner && !leaving) {
                inFlight.current = null
                setActiveSelection((current) => (current === selection ? undefined : current))
            }
        }
    }

    async function reconcile() {
        if (!usable || recorded || inFlight.current) return
        const owner = {}
        inFlight.current = owner
        setActiveSelection(selection)
        try {
            const token = getAccessToken()
            if (token) {
                const result = await productRequest(window.location.origin, token, scope, {
                    step: "reconcile",
                })
                if (inFlight.current === owner && selected.current === selection) {
                    // Only the exact durable broker can report that no ceremony exists.
                    // Never use a timeout, status error or empty panel to allow a retry.
                    if (result.phase === "ready") {
                        setReadySelection(selection)
                        setUncertainSelection((current) =>
                            current === selection ? undefined : current
                        )
                    }
                }
            }
        } catch {
            // The normal panel can already show a settled approval if a prior response was lost.
        } finally {
            if (inFlight.current === owner) {
                inFlight.current = null
                setActiveSelection((current) => (current === selection ? undefined : current))
                onRefresh()
            }
        }
    }

    const phase = recorded
        ? "approval-recorded"
        : uncertain
          ? "reconcile"
          : usable && eligible
            ? "challenge-available"
            : "unavailable"
    return (
        <Dialog
            open
            fullWidth
            maxWidth="sm"
            aria-labelledby={titleId}
            onClose={busy ? undefined : onClose}
        >
            <DialogTitle id={titleId}>{t("signing.operational.title")}</DialogTitle>
            <DialogContent>
                <Stack spacing={2}>
                    <SigningSubject data={data} api={api} variant={SigningSubjectVariant.Dialog} />
                    {data.operational_signing_setup === true &&
                    usable &&
                    eligible &&
                    !recorded &&
                    !uncertain ? (
                        <ProductSetupButton
                            scope={{
                                tenant: scope.tenant,
                                event: scope.event,
                                request: scope.request,
                                reviewedPayload: request.payload_sha256,
                            }}
                            eligible={eligible}
                            reviewed={reviewed.verified}
                            disabled={busy}
                            onRequested={onRefresh}
                        />
                    ) : null}
                    <Typography role="status" aria-live="polite">
                        {t(`signing.operational.phase.${phase}`)}
                    </Typography>
                    <Typography variant="body2">
                        {t("signing.operational.quorum", {
                            count: data.count,
                            required: request.required,
                        })}
                    </Typography>
                </Stack>
            </DialogContent>
            <DialogActions>
                <Button disabled={busy} onClick={onClose}>
                    {t("signing.widget.close")}
                </Button>
                {uncertain && !recorded ? (
                    <Button disabled={busy} onClick={() => void reconcile()}>
                        {t("signing.operational.checkStatus")}
                    </Button>
                ) : usable && eligible && !recorded ? (
                    <Button
                        variant="contained"
                        disabled={busy || !reviewed.verified}
                        onClick={() => void approve()}
                    >
                        {t("signing.operational.approve")}
                    </Button>
                ) : null}
            </DialogActions>
        </Dialog>
    )
}
