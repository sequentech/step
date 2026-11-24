// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useRef, useState} from "react"
import {Alert, Button, Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    requestProductSetup,
    type ProductSetupPhase,
    type ProductSetupScope,
} from "@/lib/signing/productSetup"

type Props = {
    /** The parent obtains this from the current, exact authorized request. */
    scope: ProductSetupScope
    eligible: boolean
    reviewed: boolean
    disabled?: boolean
    onRequested?: () => void
}

export function ProductSetupButton({scope, eligible, reviewed, disabled, onRequested}: Props) {
    const {t} = useTranslation()
    const auth = useContext(AuthContext)
    const operation = useRef<AbortController | null>(null)
    const [result, setResult] = useState<{
        key: string
        ticket?: string
        phase?: ProductSetupPhase
        error?: boolean
    }>()
    const [working, setWorking] = useState(false)
    const key = [scope.tenant, scope.event, scope.request, scope.reviewedPayload, auth.userId].join(
        "/"
    )
    const visible = auth.isAuthenticated && auth.tenantId === scope.tenant && eligible
    const current = useRef({key, visible, reviewed})
    current.current = {key, visible, reviewed}
    useEffect(() => {
        setWorking(false)
        return () => {
            operation.current?.abort()
            operation.current = null
        }
    }, [key, visible, reviewed])

    if (!visible || typeof window === "undefined" || window.location.protocol !== "https:") {
        return null
    }

    const resultHere = result?.key === key ? result : undefined
    const request = async () => {
        if (!reviewed || disabled || operation.current) return
        const controller = new AbortController()
        operation.current = controller
        setWorking(true)
        const stillSelected = () =>
            operation.current === controller &&
            current.current.key === key &&
            current.current.visible &&
            current.current.reviewed &&
            !controller.signal.aborted
        try {
            if (!auth.isGoldUser()) {
                // Only the established normal tenant realm can establish
                // freshness. Nothing is submitted on return without another click.
                await auth.reauthWithGold(window.location.href, true)
                return
            }
            const token = auth.getAccessToken()
            if (!token) throw new Error("No authenticated setup session")
            const reply = await requestProductSetup(token, scope, auth.userId, controller.signal)
            if (!stillSelected()) return
            setResult({key, ticket: reply.ticket, phase: reply.phase})
            onRequested?.()
        } catch {
            if (stillSelected()) setResult({key, error: true})
        } finally {
            if (operation.current === controller) {
                operation.current = null
                setWorking(false)
            }
        }
    }

    if (resultHere?.ticket) {
        return (
            <Stack role="status" spacing={1}>
                <Typography>
                    {t(
                        resultHere.phase === "reconciliation-required"
                            ? "signing.operational.phase.reconcile"
                            : resultHere.phase === "certificate-on-file"
                              ? "signing.operational.phase.challenge-available"
                              : "signing.operational.phase.setup-pending"
                    )}
                </Typography>
                <Typography component="code">{resultHere.ticket}</Typography>
            </Stack>
        )
    }
    return (
        <Stack spacing={1}>
            {resultHere?.error && <Alert severity="error">{t("signing.operational.error")}</Alert>}
            <Button disabled={disabled || working || !reviewed} onClick={() => void request()}>
                {t("signing.operational.setup")}
            </Button>
        </Stack>
    )
}
