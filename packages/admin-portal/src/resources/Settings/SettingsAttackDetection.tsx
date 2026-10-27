// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useRef, useState} from "react"
import {gql, useMutation, useQuery} from "@apollo/client"
import {Alert, Button, CircularProgress, Stack, TextField, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"

export const GET_ATTACK_SETTINGS = gql`
    query GetAttackDetectionSettings {
        get_attack_detection_settings {
            tenant_id
            settings
        }
    }
`
export const SAVE_ATTACK_SETTINGS = gql`
    mutation UpdateAttackDetectionSettings($settings: jsonb!) {
        update_attack_detection_settings(settings: $settings) {
            tenant_id
            settings
        }
    }
`
interface Rates {
    failed_logins_per_minute: number
    requests_per_minute: number
}
interface SettingsResult {
    tenant_id: string
    settings: Rates
}
enum SaveState {
    Idle,
    Saving,
    Saved,
    Failed,
}

const positiveInteger = (value: unknown, maximum: number): value is number =>
    typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= maximum
const validRates = (value: Rates | undefined): value is Rates =>
    !!value &&
    positiveInteger(value.failed_logins_per_minute, 100000) &&
    positiveInteger(value.requests_per_minute, 10000000)
const matches = (result: SettingsResult | undefined, tenant: string, expected: Rates) =>
    result?.tenant_id === tenant &&
    validRates(result.settings) &&
    result.settings.failed_logins_per_minute === expected.failed_logins_per_minute &&
    result.settings.requests_per_minute === expected.requests_per_minute

export function SettingsAttackDetection() {
    const {t} = useTranslation()
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const allowed =
        !!tenantId &&
        auth.isAuthorized(true, tenantId, IPermissions.ADMIN_USER) &&
        auth.isAuthorized(true, tenantId, IPermissions.TENANT_WRITE)
    const activeTenant = useRef(tenantId)
    activeTenant.current = tenantId
    const {data, loading, error, refetch} = useQuery<{
        get_attack_detection_settings: SettingsResult
    }>(GET_ATTACK_SETTINGS, {skip: !allowed, fetchPolicy: "network-only"})
    const [save] = useMutation<{update_attack_detection_settings: SettingsResult}>(
        SAVE_ATTACK_SETTINGS
    )
    const [failedLogins, setFailedLogins] = useState("")
    const [requests, setRequests] = useState("")
    const [state, setState] = useState(SaveState.Idle)
    const result = data?.get_attack_detection_settings
    const verified = result?.tenant_id === tenantId && validRates(result?.settings)
    useEffect(() => {
        setState(SaveState.Idle)
    }, [tenantId])
    useEffect(() => {
        setFailedLogins(verified ? String(result!.settings.failed_logins_per_minute) : "")
        setRequests(verified ? String(result!.settings.requests_per_minute) : "")
    }, [tenantId, result, verified])
    const selected: Rates = {
        failed_logins_per_minute: Number(failedLogins),
        requests_per_minute: Number(requests),
    }
    const update = async () => {
        if (
            !allowed ||
            !tenantId ||
            !verified ||
            !validRates(selected) ||
            state === SaveState.Saving
        )
            return
        const tenant = tenantId
        setState(SaveState.Saving)
        try {
            const response = await save({variables: {settings: selected}})
            if (!matches(response.data?.update_attack_detection_settings, tenant, selected))
                throw new Error("Unverified update")
            const current = await refetch()
            if (!matches(current.data?.get_attack_detection_settings, tenant, selected))
                throw new Error("Unverified read-back")
            if (activeTenant.current === tenant) setState(SaveState.Saved)
        } catch {
            if (activeTenant.current === tenant) setState(SaveState.Failed)
        }
    }
    if (!allowed)
        return (
            <Alert severity="info">
                {t(
                    "attackDetection.permission",
                    "Administrator permission is required to configure attack detection."
                )}
            </Alert>
        )
    if (loading && !data)
        return (
            <CircularProgress
                aria-label={t("attackDetection.loading", "Loading attack detection settings")}
            />
        )
    if (error || !verified)
        return (
            <Alert severity="error">
                {t(
                    "attackDetection.readFailed",
                    "The current tenant's settings could not be verified. Reload before making changes."
                )}
            </Alert>
        )
    return (
        <Stack spacing={2} sx={{p: 3, maxWidth: 720}}>
            <Typography component="h2" variant="h6">
                {t("attackDetection.title", "Attack detection")}
            </Typography>
            <Typography>
                {t(
                    "attackDetection.description",
                    "Set these rates with the election organizer during preparation. The system compares logged activity with the thresholds and records potential attacks for the election board. These settings do not disable account lockout or rate-limit requests."
                )}
            </Typography>
            <TextField
                type="number"
                label={t("attackDetection.failedLogins", "Failed logins per minute, all accounts")}
                value={failedLogins}
                inputProps={{min: 1, max: 100000, step: 1}}
                disabled={state === SaveState.Saving || loading}
                onChange={(event) => {
                    setFailedLogins(event.target.value)
                    setState(SaveState.Idle)
                }}
                error={!positiveInteger(selected.failed_logins_per_minute, 100000)}
                helperText={t(
                    "attackDetection.failedLoginsHelp",
                    "Enter a whole number from 1 to 100,000. Per-account authentication failure protection also remains active."
                )}
            />
            <TextField
                type="number"
                label={t("attackDetection.requests", "Requests per minute, external interfaces")}
                value={requests}
                inputProps={{min: 1, max: 10000000, step: 1}}
                disabled={state === SaveState.Saving || loading}
                onChange={(event) => {
                    setRequests(event.target.value)
                    setState(SaveState.Idle)
                }}
                error={!positiveInteger(selected.requests_per_minute, 10000000)}
                helperText={t(
                    "attackDetection.requestsHelp",
                    "Enter a whole number from 1 to 10,000,000. The count covers the external interfaces, including unauthenticated requests."
                )}
            />
            <Button
                variant="contained"
                onClick={() => void update()}
                disabled={!validRates(selected) || state === SaveState.Saving || loading}
            >
                {t("attackDetection.save", "Save attack detection settings")}
            </Button>
            {state === SaveState.Saved && (
                <Alert severity="success">
                    {t(
                        "attackDetection.saved",
                        "Attack detection settings saved and verified. The board must approve the updated configuration before execution."
                    )}
                </Alert>
            )}
            {state === SaveState.Failed && (
                <Alert severity="error">
                    {t(
                        "attackDetection.saveFailed",
                        "The settings were not saved and verified. Reload the current values and check the election phase before trying again."
                    )}
                </Alert>
            )}
        </Stack>
    )
}
