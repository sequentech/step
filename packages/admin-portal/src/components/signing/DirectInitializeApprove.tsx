// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef, useState} from "react"
import {Alert, Button, CircularProgress, Stack} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {reviewedInitializePost} from "@/lib/signing/operationalApprovalPolicy"
import {useSignedView} from "./useSignedView"

export const DirectInitializeApprove: React.FC<{
    data: ISigningPanelData
    api: ISigningApi
    onRefresh: () => Promise<void>
}> = ({data, api, onRefresh}) => {
    const {t} = useTranslation()
    const {verified, view} = useSignedView(data)
    const click = useRef(false)
    const [busy, setBusy] = useState(false)
    const [notConfirmed, setNotConfirmed] = useState(false)
    const {request} = data
    const available = Boolean(
        api.approveDirectInitialize &&
        window.location.origin === "http://localhost:3002" &&
        verified &&
        reviewedInitializePost(request, view)
    )
    return (
        <Stack spacing={1}>
            <Alert severity="info">{t("signing.operational.softwareDemo")}</Alert>
            {notConfirmed ? <Alert severity="warning">{t("signing.operational.error")}</Alert> : null}
            <Button
                variant="contained"
                disabled={!available || busy}
                startIcon={busy ? <CircularProgress size={16} color="inherit" aria-hidden /> : undefined}
                onClick={async () => {
                    if (!available || !api.approveDirectInitialize || click.current) return
                    click.current = true
                    setBusy(true)
                    setNotConfirmed(false)
                    try {
                        await api.approveDirectInitialize(request.id, request.payload_sha256)
                        await onRefresh()
                    } catch {
                        setNotConfirmed(true)
                        await onRefresh()
                    } finally {
                        setBusy(false)
                        click.current = false
                    }
                }}
            >
                {t("signing.operational.approve")}
            </Button>
        </Stack>
    )
}
