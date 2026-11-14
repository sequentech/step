// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The tenant's trust in configuration packages: whether a package must be
// signed, which roots its signing key and its approvers' certificates must
// chain to, and how many distinct approvals it needs. Saved only through the
// set_configuration_signing action, which checks the setting first; the
// database refuses any other write of it.

import React, {useEffect, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {useApolloClient, useMutation} from "@apollo/client"
import {useGetOne} from "react-admin"
import {
    Alert,
    Box,
    Button,
    Chip,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import {GET_TENANT_SETTINGS, SET_CONFIGURATION_SIGNING} from "@/queries/SetConfigurationSigning"
import {
    ConfigurationSigningForm,
    EConfigurationSigningPolicy,
    EConfigurationSigningState,
    EFinalSignerSeparation,
    EPackageFormat,
    readPolicyFile,
    MAX_REQUIRED_APPROVALS,
    SUGGESTED_FORM,
    TrustCheck,
    certificateCount,
    readCheck,
    readSetting,
    toSetting,
    validApprovals,
} from "./configurationSigning"

interface SetConfigurationSigningResult {
    set_configuration_signing?: {saved: boolean; check: unknown} | null
}

type RootField = "packageRoots" | "staffRoots"

/** A file's text: `Blob.text()` where the browser has it, else a FileReader. */
const readText = (file: Blob): Promise<string> =>
    typeof file.text === "function"
        ? file.text()
        : new Promise((resolve, reject) => {
              const reader = new FileReader()
              reader.onload = () => resolve(String(reader.result ?? ""))
              reader.onerror = () => reject(reader.error)
              reader.readAsText(file)
          })

export const ConfigurationSigningSettings: React.FC<{tenantId: string}> = ({tenantId}) => {
    const {t} = useTranslation()
    const {data: tenant, dataUpdatedAt} = useGetOne("sequent_backend_tenant", {id: tenantId})
    // Only this tenant's own record counts: a record still cached from
    // another tenant must never seed this form.
    const current = tenant && String(tenant.id) === tenantId ? tenant : undefined
    // After a save, the stored setting is read back from the server (the
    // canonical one), without refetching react-admin's tenant record: that
    // refetch re-rendered the Settings tabs, which remounted this section and
    // lost the check's answer. The read-back is shown only until the tenant
    // record is fetched again: react-query's dataUpdatedAt moves on every
    // successful fetch, even when the content is unchanged (a policy changed
    // back to exactly what it was), and the fresh record then wins.
    const apollo = useApolloClient()
    const [afterSave, setAfterSave] = useState<{fetchedAt: number; settings: unknown} | null>(null)
    // The newest fetch time seen, read when the read-back completes: a refresh
    // of the old record that lands while the save is pending must not outrank
    // the read-back; only a fetch after it does.
    const latestFetch = useRef(dataUpdatedAt)
    latestFetch.current = dataUpdatedAt
    const fresh = afterSave === null || dataUpdatedAt > afterSave.fetchedAt
    const installed = readSetting(fresh ? current?.settings : afterSave.settings)
    const [form, setForm] = useState<ConfigurationSigningForm>(SUGGESTED_FORM)
    const [loadedFor, setLoadedFor] = useState<string | null>(null)
    const [result, setResult] = useState<{saved: boolean; check: TrustCheck} | null>(null)
    const [error, setError] = useState<string | null>(null)
    const [saving, setSaving] = useState(false)
    const [setConfigurationSigning] =
        useMutation<SetConfigurationSigningResult>(SET_CONFIGURATION_SIGNING)

    // Completions that arrive after this form is gone (another tenant
    // selected, the page left) change nothing.
    const mounted = useRef(true)
    // Set on every setup: StrictMode runs setup, cleanup, setup.
    useEffect(() => {
        mounted.current = true
        return () => {
            mounted.current = false
        }
    }, [])

    // The form starts from the installed setting, once its record is loaded.
    useEffect(() => {
        if (!current || loadedFor === tenantId) return
        setForm(installed.form ?? SUGGESTED_FORM)
        setLoadedFor(tenantId)
    }, [current, tenantId])
    const ready = loadedFor === tenantId

    const loadFile = (field: RootField) => async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0]
        event.target.value = ""
        if (!file) return
        const text = await file.text()
        if (!mounted.current) return
        setForm((previous) => ({...previous, [field]: text}))
    }

    /**
     * The installer's signing policy file: who may approve and sign (format
     * 2), the roots and counts, the minimum format and separation. Loaded
     * whole, never typed, and refused when it is for another tenant.
     */
    const loadPolicy = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0]
        event.target.value = ""
        if (!file) return
        let text: string
        try {
            text = await readText(file)
        } catch {
            setError(t("configurationSigning.policyErrors.not-json"))
            return
        }
        if (!mounted.current) return
        const read = readPolicyFile(text, tenantId)
        if (read.form === undefined) {
            setError(t(`configurationSigning.policyErrors.${read.error ?? "not-a-policy"}`))
            return
        }
        setError(null)
        setForm(read.form)
    }

    const save = async () => {
        if (!ready) return
        setError(null)
        setResult(null)
        if (!validApprovals(form.requiredApprovals)) {
            setError(t("configurationSigning.invalidApprovals", {max: MAX_REQUIRED_APPROVALS}))
            return
        }
        setSaving(true)
        try {
            const {data} = await setConfigurationSigning({
                variables: {tenantId, configurationSigning: toSetting(form)},
            })
            if (!mounted.current) return
            const answer = data?.set_configuration_signing
            if (!answer) {
                setError(t("configurationSigning.noAnswer"))
                return
            }
            setResult({saved: answer.saved, check: readCheck(answer.check)})
            if (answer.saved) {
                const stored = await apollo.query({
                    query: GET_TENANT_SETTINGS,
                    variables: {tenantId},
                    fetchPolicy: "network-only",
                })
                if (!mounted.current) return
                setAfterSave({
                    fetchedAt: latestFetch.current,
                    settings: stored.data?.sequent_backend_tenant_by_pk?.settings ?? null,
                })
            }
        } catch (caught) {
            if (!mounted.current) return
            setError(caught instanceof Error ? caught.message : String(caught))
        } finally {
            if (mounted.current) setSaving(false)
        }
    }

    const stateColor =
        installed.state === EConfigurationSigningState.REQUIRED
            ? "success"
            : installed.state === EConfigurationSigningState.UNREADABLE
              ? "error"
              : "warning"

    return (
        <Box data-testid="configuration-signing" sx={{mb: 2}}>
            <Typography variant="h6" component="h3">
                {t("configurationSigning.title")}
            </Typography>
            <Typography variant="body2" paragraph>
                {t("configurationSigning.description")}
            </Typography>
            <Box sx={{display: "flex", gap: 1, alignItems: "center", flexWrap: "wrap", mb: 2}}>
                <Typography variant="body2">{t("configurationSigning.current")}</Typography>
                <Chip
                    data-testid="configuration-signing-state"
                    color={stateColor}
                    label={t(`configurationSigning.states.${installed.state}`, {
                        approvals: installed.form?.requiredApprovals ?? 0,
                    })}
                />
            </Box>

            <Box sx={{display: "flex", gap: 2, flexWrap: "wrap", mb: 2}}>
                <FormControl sx={{minWidth: 240}}>
                    <InputLabel id="configuration-signing-policy">
                        {t("configurationSigning.policy")}
                    </InputLabel>
                    <Select
                        labelId="configuration-signing-policy"
                        label={t("configurationSigning.policy")}
                        value={form.policy}
                        disabled={!ready}
                        inputProps={{"data-testid": "configuration-signing-policy"}}
                        onChange={(event) =>
                            setForm({
                                ...form,
                                policy: event.target.value as EConfigurationSigningPolicy,
                            })
                        }
                    >
                        {Object.values(EConfigurationSigningPolicy).map((policy) => (
                            <MenuItem key={policy} value={policy}>
                                {t(`configurationSigning.policies.${policy}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <TextField
                    type="number"
                    label={t("configurationSigning.requiredApprovals")}
                    value={form.requiredApprovals}
                    disabled={!ready}
                    inputProps={{
                        "min": 0,
                        "max": MAX_REQUIRED_APPROVALS,
                        "data-testid": "configuration-signing-approvals",
                    }}
                    helperText={t("configurationSigning.requiredApprovalsHelp")}
                    onChange={(event) =>
                        setForm({...form, requiredApprovals: Number(event.target.value)})
                    }
                />
            </Box>

            {(
                [
                    ["packageRoots", "packageRootsHelp"],
                    ["staffRoots", "staffRootsHelp"],
                ] as [RootField, string][]
            ).map(([field, help]) => (
                <Box key={field} sx={{mb: 2}}>
                    <TextField
                        fullWidth
                        multiline
                        minRows={3}
                        maxRows={8}
                        label={t(`configurationSigning.${field}`)}
                        value={form[field]}
                        disabled={!ready}
                        inputProps={{
                            "data-testid": `configuration-signing-${field}`,
                            "spellCheck": false,
                            "style": {fontFamily: "monospace", fontSize: 12},
                        }}
                        helperText={`${t(`configurationSigning.${help}`)} ${t(
                            "configurationSigning.certificates",
                            {count: certificateCount(form[field])}
                        )}`}
                        onChange={(event) => setForm({...form, [field]: event.target.value})}
                    />
                    <Button component="label" size="small" sx={{mt: 0.5}} disabled={!ready}>
                        {t("configurationSigning.loadFile")}
                        <input
                            hidden
                            type="file"
                            accept=".pem,.crt,.cer,text/plain"
                            data-testid={`configuration-signing-${field}-file`}
                            onChange={loadFile(field)}
                        />
                    </Button>
                </Box>
            ))}

            <Box sx={{display: "flex", gap: 2, flexWrap: "wrap", mb: 2}}>
                <FormControl sx={{minWidth: 240}}>
                    <InputLabel id="configuration-signing-minimum">
                        {t("configurationSigning.minimumFormat")}
                    </InputLabel>
                    <Select
                        labelId="configuration-signing-minimum"
                        label={t("configurationSigning.minimumFormat")}
                        value={form.minimumFormat ?? ""}
                        disabled={!ready}
                        inputProps={{"data-testid": "configuration-signing-minimum"}}
                        onChange={(event) =>
                            setForm({...form, minimumFormat: event.target.value as EPackageFormat})
                        }
                    >
                        {Object.values(EPackageFormat).map((format) => (
                            <MenuItem key={format} value={format}>
                                {t(`configurationSigning.formats.${format}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <FormControl sx={{minWidth: 240}}>
                    <InputLabel id="configuration-signing-separation">
                        {t("configurationSigning.separation")}
                    </InputLabel>
                    <Select
                        labelId="configuration-signing-separation"
                        label={t("configurationSigning.separation")}
                        value={form.separation ?? ""}
                        disabled={!ready}
                        inputProps={{"data-testid": "configuration-signing-separation"}}
                        onChange={(event) =>
                            setForm({
                                ...form,
                                separation: event.target.value as EFinalSignerSeparation,
                            })
                        }
                    >
                        {Object.values(EFinalSignerSeparation).map((separation) => (
                            <MenuItem key={separation} value={separation}>
                                {t(`configurationSigning.separations.${separation}`)}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <Button component="label" variant="outlined" disabled={!ready}>
                    {t("configurationSigning.loadPolicy")}
                    <input
                        hidden
                        type="file"
                        accept=".json,application/json"
                        data-testid="configuration-signing-policy-file"
                        onChange={(event) => void loadPolicy(event)}
                    />
                </Button>
            </Box>
            {form.signers.length > 0 ? (
                <Table size="small" sx={{mb: 2}} data-testid="configuration-signing-signers">
                    <TableHead>
                        <TableRow>
                            <TableCell>{t("configurationSigning.columns.signerId")}</TableCell>
                            <TableCell>{t("configurationSigning.columns.name")}</TableCell>
                            <TableCell>{t("configurationSigning.columns.roles")}</TableCell>
                            <TableCell>{t("configurationSigning.columns.keys")}</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {form.signers.map((signer) => (
                            <TableRow key={signer.id} data-testid={`configuration-signing-signer-${signer.id}`}>
                                <TableCell sx={{fontFamily: "monospace"}}>{signer.id}</TableCell>
                                <TableCell>{signer.name}</TableCell>
                                <TableCell>{signer.roles.join(", ")}</TableCell>
                                <TableCell sx={{fontFamily: "monospace"}}>
                                    {signer.keys.map((key) => `${key.spki_sha256.slice(0, 16)}…`).join(" ")}
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            ) : null}

            <Button
                variant="contained"
                disabled={!ready || saving}
                onClick={() => void save()}
                data-testid="configuration-signing-save"
            >
                {t("configurationSigning.save")}
            </Button>

            {error ? (
                <Alert severity="error" sx={{mt: 2}} data-testid="configuration-signing-error">
                    {error}
                </Alert>
            ) : null}
            {result ? <CheckResult saved={result.saved} check={result.check} /> : null}
        </Box>
    )
}

const CheckResult: React.FC<{saved: boolean; check: TrustCheck}> = ({saved, check}) => {
    const {t} = useTranslation()
    return (
        <Box sx={{mt: 2}} data-testid="configuration-signing-result">
            <Alert severity={saved ? "success" : "error"}>
                {saved ? t("configurationSigning.saved") : t("configurationSigning.refused")}
            </Alert>
            {check.problems.map((problem, index) => (
                <Alert
                    key={index}
                    severity={problem.severity === "error" ? "error" : "warning"}
                    sx={{mt: 1}}
                    data-testid="configuration-signing-problem"
                >
                    {`${problem.path}: ${problem.message}`}
                </Alert>
            ))}
            {check.policySha256 ? (
                <Typography variant="body2" sx={{mt: 1}} data-testid="configuration-signing-policy-digest">
                    {t("configurationSigning.policyDigest")}{" "}
                    <Box component="span" sx={{fontFamily: "monospace"}}>
                        {check.policySha256}
                    </Box>
                </Typography>
            ) : null}
            {check.anchors.length > 0 ? (
                <Table size="small" sx={{mt: 1}} data-testid="configuration-signing-anchors">
                    <TableHead>
                        <TableRow>
                            <TableCell>{t("configurationSigning.columns.setting")}</TableCell>
                            <TableCell>{t("configurationSigning.columns.subject")}</TableCell>
                            <TableCell>{t("configurationSigning.columns.validUntil")}</TableCell>
                            <TableCell>{t("configurationSigning.columns.fingerprint")}</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {check.anchors.map((anchor) => (
                            <TableRow key={`${anchor.setting}-${anchor.fingerprint_sha256}`}>
                                <TableCell>
                                    {t(
                                        `configurationSigning.${
                                            anchor.setting === "staff_roots"
                                                ? "staffRoots"
                                                : "packageRoots"
                                        }`
                                    )}
                                </TableCell>
                                <TableCell>{anchor.subject}</TableCell>
                                <TableCell>{anchor.not_after}</TableCell>
                                <TableCell sx={{fontFamily: "monospace"}}>
                                    {anchor.fingerprint_sha256.slice(0, 16)}…
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            ) : null}
        </Box>
    )
}
