// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useContext, useEffect, useRef, useState} from "react"
import {gql, useMutation, useQuery} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    Checkbox,
    CircularProgress,
    FormControlLabel,
    Paper,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {
    CertificateAuthority,
    ConfigurationSigning,
    MAX_CERTIFICATE_FILE_BYTES,
    parseCertificateAuthorities,
    signingConfiguration,
} from "./configurationSigning"

export const GET_CONFIGURATION_SIGNING = gql`
    query GetConfigurationSigning($id: uuid!) {
        sequent_backend_tenant_by_pk(id: $id) {
            id
            settings
        }
    }
`
export const SAVE_CONFIGURATION_SIGNING = gql`
    mutation SaveConfigurationSigning(
        $id: uuid!
        $settings: jsonb!
        $previous: sequent_backend_tenant_bool_exp!
    ) {
        update_sequent_backend_tenant(
            where: {_and: [{id: {_eq: $id}}, $previous]}
            _append: {settings: $settings}
        ) {
            affected_rows
            returning {
                id
                settings
            }
        }
    }
`
export const INITIALIZE_CONFIGURATION_SIGNING = gql`
    mutation InitializeConfigurationSigning($id: uuid!, $settings: jsonb!) {
        update_sequent_backend_tenant(
            where: {id: {_eq: $id}, settings: {_is_null: true}}
            _set: {settings: $settings}
        ) {
            affected_rows
            returning {
                id
                settings
            }
        }
    }
`
interface TenantResult {
    sequent_backend_tenant_by_pk: {
        id: string
        settings: {configuration_signing?: ConfigurationSigning} | null
    } | null
}
interface SavedResult {
    update_sequent_backend_tenant: {
        affected_rows: number
        returning: {id: string; settings: {configuration_signing?: ConfigurationSigning}}[]
    } | null
}

function AuthorityFiles({
    label,
    initial,
    disabled,
    onChange,
}: {
    label: string
    initial: string
    disabled: boolean
    onChange: (pem: string, confirmed: boolean) => void
}) {
    const {t} = useTranslation()
    const [certificates, setCertificates] = useState<CertificateAuthority[]>([])
    const [confirmed, setConfirmed] = useState(false)
    const [error, setError] = useState("")
    const [loading, setLoading] = useState(false)
    const generation = useRef(0)
    const changed = useRef(onChange)
    changed.current = onChange
    const load = useCallback(
        async (pem: string) => {
            const request = ++generation.current
            setLoading(true)
            setConfirmed(false)
            setError("")
            changed.current("", false)
            try {
                const parsed = await parseCertificateAuthorities(pem)
                if (request !== generation.current) return
                setCertificates(parsed)
                // Existing saved trust remains visible but must be rechecked before a new save.
                changed.current(parsed.map(({pem}) => pem).join(""), false)
            } catch {
                if (request !== generation.current) return
                setCertificates([])
                setError(
                    t(
                        "configurationSigning.invalidCertificate",
                        "Choose a valid public PEM CA certificate file. Private keys, duplicate, expired or personal certificates are not accepted."
                    )
                )
            } finally {
                if (request === generation.current) setLoading(false)
            }
        },
        [t]
    )
    useEffect(() => {
        if (initial) void load(initial)
    }, [initial, load])
    return (
        <Paper variant="outlined" sx={{p: 2, minWidth: 0}}>
            <Stack spacing={2}>
                <Typography component="h3" variant="h6">
                    {label}
                </Typography>
                <Button component="label" variant="outlined" disabled={disabled || loading}>
                    {t("configurationSigning.chooseFile", "Choose CA certificate file")}
                    <input
                        hidden
                        disabled={disabled || loading}
                        aria-label={label}
                        type="file"
                        accept=".pem,.crt,.cer"
                        onChange={async (event) => {
                            const file = event.target.files?.[0]
                            event.target.value = ""
                            if (!file) return
                            if (file.size > MAX_CERTIFICATE_FILE_BYTES) {
                                setError(
                                    t(
                                        "configurationSigning.fileSize",
                                        "Choose a file no larger than 256 KiB."
                                    )
                                )
                                setConfirmed(false)
                                changed.current("", false)
                                return
                            }
                            try {
                                await load(await file.text())
                            } catch {
                                setError(
                                    t(
                                        "configurationSigning.fileReadFailed",
                                        "The certificate file could not be read. Choose it again."
                                    )
                                )
                                changed.current("", false)
                            }
                        }}
                    />
                </Button>
                {loading ? (
                    <CircularProgress
                        size={20}
                        aria-label={t("configurationSigning.reading", "Reading certificates")}
                    />
                ) : null}
                {error ? <Alert severity="error">{error}</Alert> : null}
                {certificates.map((certificate) => (
                    <Box key={certificate.fingerprint}>
                        <Typography sx={{overflowWrap: "anywhere"}}>
                            {certificate.subject}
                        </Typography>
                        <Typography variant="body2">
                            {t("configurationSigning.expires", "Expires")}: {certificate.expires}
                        </Typography>
                        <Typography
                            variant="body2"
                            sx={{fontFamily: "monospace", overflowWrap: "anywhere", mt: 1}}
                        >
                            SHA-256: {certificate.fingerprint}
                        </Typography>
                    </Box>
                ))}
                {certificates.length ? (
                    <FormControlLabel
                        control={
                            <Checkbox
                                checked={confirmed}
                                disabled={disabled || loading}
                                onChange={(_, checked) => {
                                    setConfirmed(checked)
                                    onChange(certificates.map(({pem}) => pem).join(""), checked)
                                }}
                            />
                        }
                        label={t(
                            "configurationSigning.confirmFingerprint",
                            "I compared every SHA-256 fingerprint with the election organizer's independently supplied record."
                        )}
                    />
                ) : null}
            </Stack>
        </Paper>
    )
}

export function ConfigurationSigningForm({
    current,
    onSave,
    readOnly = false,
}: {
    current?: ConfigurationSigning
    onSave: (value: ConfigurationSigning) => Promise<void>
    readOnly?: boolean
}) {
    const {t} = useTranslation()
    const [packageRoots, setPackageRoots] = useState("")
    const [staffRoots, setStaffRoots] = useState("")
    const [packageConfirmed, setPackageConfirmed] = useState(false)
    const [staffConfirmed, setStaffConfirmed] = useState(false)
    const [approvals, setApprovals] = useState(current?.required_approvals ?? 2)
    const [saving, setSaving] = useState(false)
    const [message, setMessage] = useState<"saved" | "failed" | null>(null)
    const validApprovals = Number.isInteger(approvals) && approvals >= 2 && approvals <= 65535
    return (
        <Stack spacing={3} sx={{p: {xs: 1, md: 3}, maxWidth: 1000}}>
            <Typography component="h2" variant="h5">
                {t("configurationSigning.title", "Signed configuration packages")}
            </Typography>
            <Typography>
                {t(
                    "configurationSigning.description",
                    "Choose the public CA certificates that issue the organizer's package-signing certificates and the election board's approval certificates. Signed packages and at least two distinct approvals are required. This does not configure HTTPS or voters' certificates."
                )}
            </Typography>
            <Alert severity="info">
                {t(
                    "configurationSigning.lockNotice",
                    "After the first save, the package-signing CAs and approval count cannot change. Staff CAs can change only when all the tenant's election events are archived. Include planned successor CAs before importing the first election."
                )}
            </Alert>
            <AuthorityFiles
                label={t("configurationSigning.packageRoots", "Package-signing CAs")}
                initial={current?.package_roots ?? ""}
                disabled={readOnly || saving || !!current}
                onChange={(pem, checked) => {
                    setPackageRoots(pem)
                    setPackageConfirmed(checked)
                }}
            />
            <AuthorityFiles
                label={t("configurationSigning.staffRoots", "Election board approval CAs")}
                initial={current?.staff_roots ?? ""}
                disabled={readOnly || saving}
                onChange={(pem, checked) => {
                    setStaffRoots(pem)
                    setStaffConfirmed(checked)
                }}
            />
            <TextField
                label={t("configurationSigning.approvals", "Required distinct approvals")}
                type="number"
                value={approvals}
                disabled={readOnly || saving || !!current}
                slotProps={{htmlInput: {min: 2, max: 65535, step: 1}}}
                onChange={(event) => setApprovals(Number(event.target.value))}
                error={!validApprovals}
                helperText={t(
                    "configurationSigning.approvalsHelp",
                    "At least 2 different board members must approve every package."
                )}
            />
            <Button
                variant="contained"
                sx={{alignSelf: "flex-start"}}
                disabled={
                    readOnly ||
                    saving ||
                    !validApprovals ||
                    !staffConfirmed ||
                    (!current && !packageConfirmed)
                }
                onClick={async () => {
                    setSaving(true)
                    setMessage(null)
                    try {
                        await onSave(
                            signingConfiguration(
                                current?.package_roots ?? packageRoots,
                                staffRoots,
                                approvals
                            )
                        )
                        setMessage("saved")
                    } catch {
                        setMessage("failed")
                    } finally {
                        setSaving(false)
                    }
                }}
            >
                {saving
                    ? t("configurationSigning.saving", "Saving…")
                    : t("configurationSigning.save", "Save certificate settings")}
            </Button>
            {message === "saved" ? (
                <Alert severity="success">
                    {t(
                        "configurationSigning.saved",
                        "Certificate settings saved and read back successfully."
                    )}
                </Alert>
            ) : null}
            {message === "failed" ? (
                <Alert severity="error">
                    {t(
                        "configurationSigning.failed",
                        "Settings were not confirmed. Reload this page before retrying. Check your permissions and whether election events are still active; saved package-signing CAs and approval counts cannot change."
                    )}
                </Alert>
            ) : null}
        </Stack>
    )
}

export function SettingsConfigurationSigning() {
    const [tenantId] = useTenantStore()
    const auth = useContext(AuthContext)
    const {t} = useTranslation()
    const allowed =
        auth.isAuthorized(true, tenantId, IPermissions.TENANT_WRITE) &&
        auth.isAuthorized(true, tenantId, IPermissions.SETTINGS_MENU)
    const {data, loading, error, refetch} = useQuery<TenantResult>(GET_CONFIGURATION_SIGNING, {
        variables: {id: tenantId},
        skip: !tenantId || !allowed,
        fetchPolicy: "network-only",
    })
    const [save] = useMutation<SavedResult>(SAVE_CONFIGURATION_SIGNING)
    const [initialize] = useMutation<SavedResult>(INITIALIZE_CONFIGURATION_SIGNING)
    if (!allowed) return <Alert severity="warning">{t("electionTypeScreen.noPermissions")}</Alert>
    if (loading) return <CircularProgress />
    if (error || !data?.sequent_backend_tenant_by_pk)
        return (
            <Alert severity="error">
                {t(
                    "configurationSigning.loadFailed",
                    "Unable to load the tenant's certificate settings. Retry before making changes."
                )}
            </Alert>
        )
    const current = data.sequent_backend_tenant_by_pk.settings?.configuration_signing
    return (
        <ConfigurationSigningForm
            key={tenantId}
            current={current}
            onSave={async (value) => {
                // Optimistic check detects another administrator's save; _append preserves other settings.
                const previous = current
                    ? {settings: {_contains: {configuration_signing: current}}}
                    : {
                          _or: [
                              {settings: {_is_null: true}},
                              {_not: {settings: {_has_key: "configuration_signing"}}},
                          ],
                      }
                const result = await (
                    data.sequent_backend_tenant_by_pk?.settings == null ? initialize : save
                )({
                    variables: {id: tenantId, previous, settings: {configuration_signing: value}},
                })
                if (result.data?.update_sequent_backend_tenant?.affected_rows !== 1)
                    throw new Error("Configuration changed; reload")
                const fresh = (await refetch()).data.sequent_backend_tenant_by_pk?.settings
                    ?.configuration_signing
                if (
                    !fresh ||
                    fresh.policy !== value.policy ||
                    fresh.package_roots !== value.package_roots ||
                    fresh.staff_roots !== value.staff_roots ||
                    fresh.required_approvals !== value.required_approvals
                )
                    throw new Error("Saved configuration differs")
            }}
        />
    )
}
