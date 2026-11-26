// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useId, useLayoutEffect, useRef, useState} from "react"
import {
    Alert,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Stack,
    Step,
    StepLabel,
    Stepper,
    Typography,
} from "@mui/material"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import DrawIcon from "@mui/icons-material/Draw"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {reviewedDirectAction, reviewedInitializePost} from "@/lib/signing/operationalApprovalPolicy"
import {isId} from "@/lib/signing/operationalGoldReturn"
import {SignBlock, signBlock, signedView, signerName, type ISignedView} from "@/lib/signing/request"
import {SigningSignerStatus} from "@/lib/signing/types"
import {SigningSubject, SigningSubjectVariant} from "./SigningSubject"
import {actionDescription, actionObject, requestTitle, useSigningFormat} from "./format"
import {useSignedView} from "./useSignedView"
import {useSigningPermissions} from "./useSigningPermissions"

type Outcome = {kind: "review" | "retry" | "blocked" | "check"} | {kind: "signed"; at: string}

const directAction = (api: ISigningApi, data: ISigningPanelData, view: ISignedView | null) => {
    if (
        data.operational_signing_mode === "hsm-demo-raw" &&
        reviewedInitializePost(data.request, view)
    )
        return api.approveDirectInitialize
    if (
        data.operational_signing_mode === "hsm-demo-direct-action" &&
        reviewedDirectAction(data.request, view)
    )
        return api.approveDirectAction
    return undefined
}

const sameReview = (original: ISigningPanelData, fresh: ISigningPanelData) =>
    original.operational_signing_mode === fresh.operational_signing_mode &&
    original.request.id === fresh.request.id &&
    original.request.action === fresh.request.action &&
    original.request.tenant_id === fresh.request.tenant_id &&
    original.request.election_event_id === fresh.request.election_event_id &&
    original.request.election_id === fresh.request.election_id &&
    original.request.area_id === fresh.request.area_id &&
    original.request.document_id === fresh.request.document_id &&
    original.request.document_sha256 === fresh.request.document_sha256 &&
    original.request.canonical_payload === fresh.request.canonical_payload &&
    original.request.payload_sha256 === fresh.request.payload_sha256 &&
    original.request.trustee_id === fresh.request.trustee_id &&
    original.request.config_revision === fresh.request.config_revision &&
    original.request.rule_revision === fresh.request.rule_revision &&
    original.request.required === fresh.request.required

/** In-memory write state for an exact member, request and payload, shared across drawer openings. */
export type DirectApprovalAttempts = Map<string, "signing" | "acknowledged" | "unknown">

interface Props {
    data: ISigningPanelData
    api: ISigningApi
    reload: () => Promise<ISigningPanelData | null>
    attempts: DirectApprovalAttempts
    onChange?: (data: ISigningPanelData) => void
    onClose: () => void
}

export const DirectApprovalDialog: React.FC<Props> = ({
    data,
    api,
    reload,
    attempts,
    onChange,
    onClose,
}) => {
    const {t} = useTranslation()
    const {userId, tenantId, isAuthenticated} = useContext(AuthContext)
    const {canSign} = useSigningPermissions()
    const [review] = useState(() => ({data, userId, url: window.location.href}))
    const {request} = review.data
    const key = JSON.stringify([
        review.userId,
        request.tenant_id,
        request.id,
        request.payload_sha256,
        review.data.operational_signing_mode,
    ])
    const {verified, view} = useSignedView(review.data)
    const format = useSigningFormat(review.data.time_zone)
    const titleId = useId()
    const [outcome, setOutcome] = useState<Outcome>(() => ({
        kind: attempts.has(key) ? "check" : "review",
    }))
    const [busy, setBusy] = useState(false)
    const inFlight = useRef(false)
    const mounted = useRef(false)
    useLayoutEffect(() => {
        mounted.current = true
        return () => {
            mounted.current = false
        }
    }, [])

    const permitted = () =>
        isAuthenticated &&
        isId(userId) &&
        userId === review.userId &&
        tenantId === request.tenant_id &&
        window.location.origin === "http://localhost:3002" &&
        window.location.href === review.url &&
        sameReview(review.data, data) &&
        canSign(request.action)
    const current = useRef(permitted)
    current.current = permitted
    const here = () => mounted.current && current.current()
    const reviewer = (panel: ISigningPanelData) =>
        panel.signers.find((signer) => signer.user_id === review.userId && signer.is_you !== false)
    const eligible = (panel: ISigningPanelData) => {
        const signer = reviewer(panel)
        return (
            signBlock(panel, review.userId, true) === SignBlock.None &&
            !!signer &&
            (signer.status === undefined || signer.status === SigningSignerStatus.NotSigned)
        )
    }
    const checkOnly = outcome.kind === "check"
    const action = directAction(api, review.data, view)
    const blocked =
        !permitted() ||
        outcome.kind === "blocked" ||
        (verified && !action) ||
        (!checkOnly && !eligible(data))
    const canSubmit = verified && !!action && !blocked && outcome.kind !== "signed"
    const signed = outcome.kind === "signed" && permitted() ? outcome : null
    const member = reviewer(review.data)
    const close = () => {
        if (!inFlight.current) onClose()
    }

    // The acknowledgement contains only a request ID. Read the same bound request
    // to distinguish a saved approval, an unsigned retry and an unknown outcome.
    const read = async () => {
        const fresh = await reload()
        if (!here()) return null
        if (!fresh) throw new Error("Signing request could not be read")
        // signedView binds the refreshed action, scope and document to the same verified bytes.
        const approve =
            sameReview(review.data, fresh) && directAction(api, fresh, signedView(fresh))
        if (!approve) {
            setOutcome({kind: "blocked"})
            return null
        }
        const signer = reviewer(fresh)
        if (
            signer?.signed_at &&
            Number.isFinite(Date.parse(signer.signed_at)) &&
            (signer.status === undefined || signer.status === SigningSignerStatus.Signed)
        ) {
            attempts.delete(key)
            setOutcome({kind: "signed", at: signer.signed_at})
            onChange?.(fresh)
            return null
        }
        if (!eligible(fresh)) {
            setOutcome({kind: "blocked"})
            onChange?.(fresh)
            return null
        }
        return approve
    }

    const submit = async () => {
        if (inFlight.current || !here() || !canSubmit) return
        inFlight.current = true
        setBusy(true)
        try {
            const approve = await read()
            if (!approve || !here()) return
            if (!checkOnly && !attempts.has(key)) {
                attempts.set(key, "signing")
                try {
                    await approve(request.id, request.payload_sha256)
                    attempts.set(key, "acknowledged")
                } catch {
                    // A lost response can follow a successful commit. Always reconcile first.
                    attempts.set(key, "unknown")
                }
                if (!here() || !(await read()) || !here()) return
            }
            // An unsigned read after an ambiguous write allows an explicit retry. An
            // in-flight or acknowledged write stays read-only across drawer openings.
            if (attempts.get(key) === "unknown") attempts.delete(key)
            setOutcome({kind: attempts.has(key) ? "check" : "retry"})
        } catch {
            if (here()) setOutcome({kind: attempts.has(key) ? "check" : "retry"})
        } finally {
            inFlight.current = false
            if (mounted.current) setBusy(false)
        }
    }

    return (
        <Dialog open onClose={close} fullWidth maxWidth="sm" aria-labelledby={titleId}>
            <DialogTitle id={titleId} sx={{display: "flex", alignItems: "center", gap: 1}}>
                <DrawIcon aria-hidden />
                {t("signing.dialog.title", {object: actionObject(t, request.action)})}
            </DialogTitle>
            <DialogContent sx={{overflowWrap: "anywhere"}}>
                <Stepper activeStep={signed ? 1 : 0} sx={{mb: 3, mt: 1}}>
                    <Step completed={!!signed}>
                        <StepLabel aria-current={signed ? undefined : "step"}>
                            {t("signing.operational.review")}
                        </StepLabel>
                    </Step>
                    <Step completed={!!signed}>
                        <StepLabel aria-current={signed ? "step" : undefined}>
                            {t("signing.dialog.steps.signed")}
                        </StepLabel>
                    </Step>
                </Stepper>
                {signed ? (
                    <Stack
                        role="status"
                        spacing={1.5}
                        sx={{alignItems: "center", textAlign: "center", py: 2}}
                    >
                        <CheckCircleIcon color="success" sx={{fontSize: 56}} aria-hidden />
                        <Typography variant="h6">
                            {t("signing.operational.phase.approval-recorded")}
                        </Typography>
                        <Typography>{requestTitle(t, review.data)}</Typography>
                        <Typography color="text.secondary">
                            {t("signing.panel.signingCode")} {request.code}
                        </Typography>
                        <Typography component="time" dateTime={signed.at} color="text.secondary">
                            {format.dateTime(signed.at)}
                        </Typography>
                    </Stack>
                ) : (
                    <Stack spacing={2}>
                        {member && (
                            <Typography color="text.secondary">
                                {t("signing.operational.signingAs", {name: signerName(member)})}
                                {member.title ? ` · ${member.title}` : ""}
                            </Typography>
                        )}
                        <div>
                            <Typography variant="h6" component="p">
                                {requestTitle(t, review.data)}
                            </Typography>
                            <Typography variant="body2" color="text.secondary">
                                {actionDescription(t, request.action, review.data.seals_ballots)}
                            </Typography>
                        </div>
                        <SigningSubject
                            data={review.data}
                            api={api}
                            variant={SigningSubjectVariant.Dialog}
                        />
                        {blocked && !busy ? (
                            <Alert severity="warning">
                                {t("signing.operational.phase.unavailable")}
                            </Alert>
                        ) : outcome.kind === "retry" || checkOnly ? (
                            <Alert severity="warning">
                                {t(
                                    checkOnly
                                        ? "signing.operational.error"
                                        : "signing.operational.retryApproval"
                                )}
                            </Alert>
                        ) : null}
                    </Stack>
                )}
            </DialogContent>
            <DialogActions sx={{px: 3, pb: 2}}>
                {signed ? (
                    <Button variant="contained" autoFocus onClick={onClose}>
                        {t("signing.widget.done")}
                    </Button>
                ) : (
                    <>
                        <Button onClick={close} disabled={busy}>
                            {t("signing.dialog.cancel")}
                        </Button>
                        <Button
                            variant="contained"
                            disabled={busy || !canSubmit}
                            aria-busy={busy}
                            startIcon={
                                busy ? (
                                    <CircularProgress aria-hidden color="inherit" size={16} />
                                ) : undefined
                            }
                            onClick={submit}
                        >
                            {t(
                                checkOnly
                                    ? "signing.operational.checkStatus"
                                    : "signing.dialog.sign"
                            )}
                        </Button>
                    </>
                )}
            </DialogActions>
        </Dialog>
    )
}
