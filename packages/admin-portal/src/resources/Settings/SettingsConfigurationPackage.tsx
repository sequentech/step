// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Import a complete configuration package (the delivery zip Election
// Architect exports, carried on removable media) into this tenant: always a
// preview first, then the application of exactly what was previewed, each
// through the platform's apply_configuration_package task. Applying runs the
// server's own preview again before anything changes; this page never skips it
// and never resubmits on its own.

import React, {useContext, useEffect, useId, useMemo, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {useMutation, useQuery} from "@apollo/client"
import {disabledButtonStyle} from "@sequentech/ui-essentials"
import {
    Alert,
    Box,
    Button,
    Checkbox,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Divider,
    Drawer,
    FormControlLabel,
    List,
    ListItem,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import {notifyConfigurationPackageElectionImported} from "@/hooks/useConfigurationPackageElectionRefresh"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import type {AuthContextValues} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {
    APPLY_CONFIGURATION_PACKAGE,
    APPLY_MANAGED_CONFIGURATION_PACKAGE,
    GET_PACKAGE_TASK,
    LIST_PACKAGE_APPLICATIONS,
} from "@/queries/ApplyConfigurationPackage"
import ElectionHeader from "@/components/ElectionHeader"
import {ImportScreen} from "@/components/election-event/import-data/ImportScreen"
import {ArchitectApprovalPolicy} from "./ArchitectApprovalPolicy"
import {
    ArchitectSignerRoster,
    RosterStatus,
    rosterStateAllowsImport,
    useArchitectSignerRoster,
} from "./ArchitectSignerRoster"
import {ConfigurationSigningSettings} from "./ConfigurationSigningSettings"
import {
    ActiveSubmission,
    ApplicationView,
    ApplyMode,
    COMPLETE_DELIVERY,
    PACKAGE_SECTIONS,
    PackageRequirements,
    PackageSection,
    PreviewedPackage,
    forgetSubmission,
    isFinished,
    lastLogLine,
    mayApply,
    mayApplyPackages,
    readApplication,
    recalledSubmission,
    readManagedPreview,
    changeLines,
    rememberSubmission,
    safeText,
    sectionsToChange,
    shaMatches,
} from "./configurationPackage"

const POLL_INTERVAL_MS = 3000

const SHA256 = /^[a-fA-F0-9]{64}$/

export const SettingsConfigurationPackage: React.FC = () => {
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    // Discard status, can_manage, file candidates, and in-flight results from
    // the previous operator even when the tenant itself did not change.
    return (
        <SettingsConfigurationPackageForScope
            key={JSON.stringify([tenantId, authContext.userId])}
            tenantId={tenantId ?? null}
            authContext={authContext}
        />
    )
}

const SettingsConfigurationPackageForScope: React.FC<{
    tenantId: string | null
    authContext: AuthContextValues
}> = ({tenantId, authContext}) => {
    const {t} = useTranslation()
    // Checked on every render, and again just before every submission.
    const allowed = !!authContext.userId && mayApplyPackages(tenantId, authContext.isAuthorized)
    const roster = useArchitectSignerRoster(tenantId, authContext.userId, allowed)
    const permitsImport = rosterStateAllowsImport(roster.status)
    const showManualEntry = roster.status !== null && roster.status.mode !== "unavailable"
    const rosterStatusId = useId()
    const advancedHeadingId = useId()
    const packageRequest = useRef(false)
    const refreshedElectionTask = useRef<string | null>(null)

    // A reload in the same tenant resumes the task it was following, with the
    // requirements and the digest that task was submitted with.
    const [submitted, setSubmitted] = useState<ActiveSubmission | null>(() =>
        recalledSubmission(tenantId)
    )
    const [requirements, setRequirements] = useState<PackageRequirements>(
        () => submitted?.requirements ?? COMPLETE_DELIVERY
    )
    const [documentId, setDocumentId] = useState<string | null>(() => submitted?.documentId ?? null)
    const [expectedSha, setExpectedSha] = useState<string | null>(
        () => submitted?.expectedSha ?? null
    )
    const [typedDocumentId, setTypedDocumentId] = useState("")
    const [openUpload, setOpenUpload] = useState(false)
    useEffect(() => {
        if (!permitsImport) setOpenUpload(false)
    }, [permitsImport])
    const [legacySetupTenant, setLegacySetupTenant] = useState<string | null>(null)
    const legacySetupId = useId()
    const [confirming, setConfirming] = useState(false)
    const [previewed, setPreviewed] = useState<PreviewedPackage | null>(null)
    const [error, setError] = useState<string | null>(null)
    const [submitting, setSubmitting] = useState(false)

    // Check referential scope after awaits, including a switch away and back.
    const scope = useRef({tenantId, actor: authContext.userId})
    if (scope.current.tenantId !== tenantId || scope.current.actor !== authContext.userId) {
        scope.current = {tenantId, actor: authContext.userId}
    }
    const mounted = useRef(true)
    const operatorPreview = useRef<{scope: typeof scope.current; taskId: string} | null>(null)
    useEffect(() => {
        mounted.current = true
        return () => {
            mounted.current = false
        }
    }, [])

    // Another tenant is not what was previewed, nor the task it follows.
    useEffect(() => {
        setPreviewed(null)
        setLegacySetupTenant(null)
        setTypedDocumentId("")
        setOpenUpload(false)
        setConfirming(false)
        setError(null)
        const recalled = recalledSubmission(tenantId)
        setSubmitted(recalled)
        setRequirements(recalled?.requirements ?? COMPLETE_DELIVERY)
        setDocumentId(recalled?.documentId ?? null)
        setExpectedSha(recalled?.expectedSha ?? null)
    }, [tenantId, authContext.userId])

    const [applyConfigurationPackage] = useMutation(APPLY_CONFIGURATION_PACKAGE, {
        context: {headers: {"x-hasura-role": IPermissions.TENANT_WRITE}},
    })

    const [applyManagedConfigurationPackage] = useMutation(APPLY_MANAGED_CONFIGURATION_PACKAGE, {
        context: {headers: {"x-hasura-role": IPermissions.TENANT_WRITE}},
    })

    const {
        data: taskData,
        error: taskError,
        loading: taskLoading,
        startPolling,
        stopPolling,
    } = useQuery(GET_PACKAGE_TASK, {
        variables: {taskId: submitted?.taskId, tenantId},
        skip: !submitted || !tenantId || !allowed,
        pollInterval: POLL_INTERVAL_MS,
        fetchPolicy: "network-only",
    })
    const rows = taskData?.sequent_backend_tasks_execution
    const task = rows?.[0]
    const finished = isFinished(task?.execution_status)
    const view = useMemo(() => readApplication(task?.annotations), [task?.annotations])
    // A task that cannot be read is reported, never waited on forever.
    const unreadable =
        !!submitted && !taskLoading && (!!taskError || (Array.isArray(rows) && rows.length === 0))

    const {data: historyData, refetch: refetchHistory} = useQuery(LIST_PACKAGE_APPLICATIONS, {
        variables: {tenantId},
        skip: !allowed || !tenantId,
        fetchPolicy: "cache-and-network",
    })
    const refreshHistory = () => {
        refetchHistory?.()?.catch?.(() => undefined)
    }

    // Each submission is polled until its task finishes.
    useEffect(() => {
        if (submitted) {
            startPolling(POLL_INTERVAL_MS)
        }
    }, [submitted?.taskId, startPolling])

    // A finished task stops polling and refreshes the history; a finished
    // preview is what Apply applies.
    useEffect(() => {
        if (!submitted || !finished || !tenantId) {
            return
        }
        stopPolling()
        refreshHistory()
        if (
            allowed &&
            submitted.mode === "apply" &&
            submitted.actor === authContext.userId &&
            !submitted.electionsRefreshed &&
            refreshedElectionTask.current !== submitted.taskId &&
            task?.id === submitted.taskId &&
            task.execution_status === "SUCCESS" &&
            view?.mode === "apply" &&
            view.outcome === "applied" &&
            view.sections.some(
                (section) => section.section === "election-event" && section.state === "applied"
            )
        ) {
            refreshedElectionTask.current = submitted.taskId
            rememberSubmission(tenantId, {...submitted, electionsRefreshed: true})
            notifyConfigurationPackageElectionImported({tenantId, actor: authContext.userId})
        }
        if (submitted.mode === "preview" && view) {
            setDocumentId(submitted.documentId)
            setPreviewed({
                tenantId,
                documentId: submitted.documentId,
                requirements: submitted.requirements,
                view,
                taskId:
                    task?.id === submitted.taskId && task?.execution_status === "SUCCESS"
                        ? submitted.taskId
                        : undefined,
                managedBinding: readManagedPreview(
                    task?.annotations,
                    tenantId,
                    submitted.requirements
                ),
            })
        }
        // refreshHistory is a fresh closure each render; the task finishing is the trigger.
    }, [
        submitted,
        finished,
        view,
        stopPolling,
        tenantId,
        task?.id,
        task?.execution_status,
        allowed,
        authContext.userId,
    ])

    const matchesTrust = (current: RosterStatus | null, preview: PreviewedPackage | null) => {
        if (!rosterStateAllowsImport(current)) return false
        if (current?.mode !== "portal_managed") return !preview?.managedBinding
        return (
            !!preview?.taskId &&
            !!scope.current.actor &&
            operatorPreview.current?.scope === scope.current &&
            operatorPreview.current.taskId === preview.taskId &&
            !!preview.managedBinding &&
            preview.managedBinding.sha256 === current.active_sha256 &&
            preview.managedBinding.generation === current.generation &&
            preview.managedBinding.requiredApprovals === current.required_approvals
        )
    }
    const confirmedInputs = useRef({documentId, requirements, expectedSha, allowed})
    confirmedInputs.current = {documentId, requirements, expectedSha, allowed}

    if (!allowed || !tenantId) {
        return (
            <Alert severity="info" data-testid="package-not-permitted">
                {t("configurationPackage.notPermitted")}
            </Alert>
        )
    }

    // One submission per click: a failure or an unknown result is reported and
    // left for the operator, never resubmitted here. Every submission, preview
    // or apply, needs a fresh preview before the next application.
    const submit = async (
        mode: ApplyMode,
        document: string,
        sha: string | null,
        encryptedPayloadSha256?: string
    ) => {
        if (packageRequest.current) return
        setError(null)
        if (!authContext.userId || !mayApplyPackages(tenantId, authContext.isAuthorized)) {
            setError(t("configurationPackage.notPermitted"))
            return
        }
        const forTenant = tenantId
        if (!permitsImport) {
            setError(t("configurationPackage.roster.importUnavailable"))
            return
        }
        const forPreview = previewed
        const requestScope = scope.current
        if (
            mode === "apply" &&
            (!mayApply(forPreview, forTenant, document, requirements, sha) ||
                !matchesTrust(roster.status, forPreview))
        )
            return
        packageRequest.current = true
        // Synchronously consume the selected preview before any await. A reload,
        // tenant change or missing server reply may not make this Apply reusable.
        forgetSubmission(forTenant)
        stopPolling()
        setPreviewed(null)
        setSubmitted(null)
        setSubmitting(true)
        try {
            let previewTaskId: string | undefined
            if (mode === "apply" && roster.status?.mode === "portal_managed") {
                // Recheck immediately on Confirm; the backend repeats the check
                // atomically when admitting the selected task.
                const read = await roster.read()
                const latest = confirmedInputs.current
                if (!mounted.current || scope.current !== requestScope) return
                if (
                    !latest.allowed ||
                    !mayApply(
                        forPreview,
                        forTenant,
                        latest.documentId,
                        latest.requirements,
                        latest.expectedSha
                    ) ||
                    read?.mode !== "portal_managed" ||
                    !matchesTrust(read, forPreview)
                ) {
                    setError(t("configurationPackage.roster.previewAgain"))
                    return
                }
                previewTaskId = forPreview?.taskId
                if (!previewTaskId) return
            }
            operatorPreview.current = null
            const {data, errors} = previewTaskId
                ? await applyManagedConfigurationPackage({
                      variables: {
                          tenantId: forTenant,
                          documentId: document,
                          requirements,
                          previewTaskId,
                          ...(encryptedPayloadSha256 ? {encryptedPayloadSha256} : {}),
                      },
                  })
                : await applyConfigurationPackage({
                      variables: {
                          tenantId: forTenant,
                          documentId: document,
                          mode,
                          requirements,
                          ...(encryptedPayloadSha256 ? {encryptedPayloadSha256} : {}),
                      },
                  })
            if (!mounted.current || scope.current !== requestScope) {
                return
            }
            const taskId = data?.apply_configuration_package?.task_execution?.id
            if (errors?.length) {
                setError(errors.map((e) => e.message).join(", "))
                return
            }
            if (!taskId) {
                setError(
                    `${t("configurationPackage.noTask")} ${t("configurationPackage.mayHaveTask")}`
                )
                return
            }
            const active: ActiveSubmission = {
                taskId,
                actor: requestScope.actor,
                mode,
                documentId: document,
                requirements,
                expectedSha: sha,
                ...(encryptedPayloadSha256 ? {encryptedPayloadSha256} : {}),
            }
            if (mode === "preview") operatorPreview.current = {scope: requestScope, taskId}
            rememberSubmission(forTenant, active)
            setSubmitted(active)
        } catch (e) {
            if (!mounted.current || scope.current !== requestScope) {
                return
            }
            // The request may have reached the platform before the reply was lost.
            setError(
                `${String(e instanceof Error ? e.message : e)} ${t("configurationPackage.mayHaveTask")}`
            )
        } finally {
            packageRequest.current = false
            if (mounted.current) {
                setSubmitting(false)
                if (scope.current === requestScope) refreshHistory()
            }
        }
    }

    const stopFollowing = () => {
        operatorPreview.current = null
        forgetSubmission(tenantId)
        stopPolling()
        setSubmitted(null)
        setPreviewed(null)
    }

    // Other requirements are not what was previewed.
    const changeRequirements = (next: PackageRequirements) => {
        setPreviewed(null)
        setRequirements(next)
    }

    const toggleSection = (section: PackageSection) =>
        changeRequirements({
            ...requirements,
            sections: requirements.sections.includes(section)
                ? requirements.sections.filter((s) => s !== section)
                : PACKAGE_SECTIONS.filter(
                      (s) => s === section || requirements.sections.includes(s)
                  ),
        })

    const running = submitting || (!!submitted && !finished && !unreadable)
    const canApply =
        !running &&
        mayApply(previewed, tenantId, documentId, requirements, expectedSha) &&
        matchesTrust(roster.status, previewed)
    const shaMismatch = !!previewed && !shaMatches(previewed.view, expectedSha)
    const noRecord = !!submitted && finished && !view

    return (
        <Box sx={{p: 1}}>
            <Typography className="title" variant="h4" component="h2">
                {t("configurationPackage.title")}
            </Typography>
            <Typography variant="body1" paragraph>
                {t("configurationPackage.description")}
            </Typography>
            <Typography variant="body2" data-testid="package-tenant">
                {t("configurationPackage.tenant", {tenantId})}
            </Typography>

            <Divider sx={{my: 2}} />
            <Button
                variant="contained"
                disabled={running || !permitsImport}
                aria-describedby={!permitsImport ? rosterStatusId : undefined}
                sx={{"&&.Mui-disabled": disabledButtonStyle}}
                onClick={() => {
                    if (!running && permitsImport) setOpenUpload(true)
                }}
            >
                {t("configurationPackage.upload")}
            </Button>
            <Typography variant="body2" sx={{mt: 1}}>
                {t("configurationPackage.verificationNote")}
            </Typography>

            <ArchitectSignerRoster
                tenantId={tenantId}
                status={roster.status}
                loading={roster.loading}
                statusTextId={rosterStatusId}
                refresh={roster.read}
            />

            <Box
                component="section"
                aria-labelledby={advancedHeadingId}
                sx={{my: 2}}
                data-testid="package-advanced"
            >
                <Typography id={advancedHeadingId} component="h3" variant="h6">
                    {t("configurationPackage.advanced")}
                </Typography>
                <Box sx={{mt: 2}}>
                    <Typography variant="subtitle1" component="h4">
                        {t("configurationPackage.requirements")}
                    </Typography>
                    {PACKAGE_SECTIONS.map((section) => (
                        <FormControlLabel
                            key={section}
                            control={
                                <Checkbox
                                    checked={requirements.sections.includes(section)}
                                    onChange={() => toggleSection(section)}
                                    inputProps={{"aria-label": section}}
                                />
                            }
                            label={String(t(`configurationPackage.sections.${section}`))}
                        />
                    ))}
                    <FormControlLabel
                        control={
                            <Checkbox
                                checked={requirements.census === "required"}
                                onChange={(event) =>
                                    changeRequirements({
                                        ...requirements,
                                        census: event.target.checked ? "required" : "optional",
                                    })
                                }
                                inputProps={{"aria-label": "census"}}
                            />
                        }
                        label={String(t("configurationPackage.censusRequired"))}
                    />
                </Box>
                {showManualEntry && (
                    <Box sx={{display: "flex", gap: 2, alignItems: "center", flexWrap: "wrap"}}>
                        <TextField
                            size="small"
                            label={String(
                                t(
                                    roster.status?.mode === "portal_managed"
                                        ? "configurationPackage.signedDocumentId"
                                        : "configurationPackage.documentId"
                                )
                            )}
                            disabled={running || !permitsImport}
                            value={typedDocumentId}
                            onChange={(event) => {
                                setPreviewed(null)
                                setTypedDocumentId(event.target.value)
                            }}
                        />
                        <Button
                            variant="outlined"
                            disabled={running || !permitsImport || !typedDocumentId.trim()}
                            onClick={() => {
                                const document = typedDocumentId.trim()
                                setDocumentId(document)
                                // A typed document id carries no entered digest.
                                setExpectedSha(null)
                                submit("preview", document, null)
                            }}
                        >
                            {t("configurationPackage.preview")}
                        </Button>
                    </Box>
                )}
                {roster.status?.mode === "not_enabled" && (
                    <Box sx={{my: 2}}>
                        <Button
                            variant="text"
                            aria-expanded={legacySetupTenant === tenantId}
                            aria-controls={legacySetupId}
                            onClick={() =>
                                setLegacySetupTenant(
                                    legacySetupTenant === tenantId ? null : tenantId
                                )
                            }
                        >
                            {t("configurationPackage.legacySetup")}
                        </Button>
                        <Box id={legacySetupId}>
                            {legacySetupTenant === tenantId && (
                                <ConfigurationSigningSettings key={tenantId} tenantId={tenantId} />
                            )}
                        </Box>
                    </Box>
                )}
            </Box>
            <Box>
                <Button
                    variant="contained"
                    color="warning"
                    disabled={!canApply}
                    data-testid="package-apply"
                    onClick={() => setConfirming(true)}
                >
                    {t("configurationPackage.apply")}
                </Button>
            </Box>
            <Typography variant="caption" display="block" sx={{mt: 1}}>
                {t("configurationPackage.applyRechecks")}
            </Typography>
            {!canApply && previewed?.view.blockers.length ? (
                <Alert severity="warning" sx={{mt: 1}}>
                    {t("configurationPackage.blocked")}
                </Alert>
            ) : null}
            {previewed &&
                mayApply(previewed, tenantId, documentId, requirements, expectedSha) &&
                !matchesTrust(roster.status, previewed) && (
                    <Alert severity="warning" sx={{mt: 1}}>
                        {t("configurationPackage.roster.previewAgain")}
                    </Alert>
                )}
            {shaMismatch && (
                <Alert severity="error" sx={{mt: 1}} data-testid="package-sha-mismatch">
                    {t("configurationPackage.shaMismatch", {expected: expectedSha})}
                </Alert>
            )}
            {error && (
                <Alert severity="error" sx={{mt: 1}} data-testid="package-error">
                    {safeText(error)}
                </Alert>
            )}

            {submitted && (
                <Box sx={{mt: 2}} data-testid="package-run">
                    <Typography variant="h6" component="h3">
                        {t(`configurationPackage.mode.${submitted.mode}`)}
                    </Typography>
                    <Typography variant="body2" data-testid="package-task">
                        {t("configurationPackage.task", {taskId: submitted.taskId})}
                    </Typography>
                    <Typography variant="body2" data-testid="package-outcome">
                        {unreadable
                            ? t("configurationPackage.unreadable", {
                                  reason: taskError?.message
                                      ? safeText(taskError.message)
                                      : t("configurationPackage.notFound"),
                              })
                            : noRecord
                              ? t("configurationPackage.stoppedWithoutRecord")
                              : finished
                                ? t(`configurationPackage.outcome.${view?.outcome ?? "unknown"}`)
                                : t("configurationPackage.running")}
                    </Typography>
                    {noRecord && lastLogLine(task?.logs) && (
                        <Typography variant="body2" data-testid="package-last-log">
                            {t("configurationPackage.lastLog", {
                                message: safeText(lastLogLine(task?.logs) ?? ""),
                            })}
                        </Typography>
                    )}
                    {view && finished && view.outcome === "previewed" && (
                        <Typography variant="body2">
                            {t("configurationPackage.toChange", {count: sectionsToChange(view)})}
                        </Typography>
                    )}
                    {view && <ApplicationTable view={view} />}
                    <Button
                        size="small"
                        sx={{mt: 1}}
                        data-testid="package-stop-following"
                        onClick={stopFollowing}
                    >
                        {t("configurationPackage.stopFollowing")}
                    </Button>
                </Box>
            )}

            <Divider sx={{my: 2}} />
            <Typography variant="h6" component="h3">
                {t("configurationPackage.history")}
            </Typography>
            <List dense data-testid="package-history">
                {(historyData?.sequent_backend_tasks_execution ?? []).map((row: any) => {
                    const record = readApplication(row.annotations)
                    return (
                        <ListItem key={row.id}>
                            {`${row.created_at} · ${row.id} · ${record?.mode ?? "?"} · ${
                                record?.outcome ?? row.execution_status
                            }${record?.packageSha256 ? ` · ${record.packageSha256.slice(0, 12)}` : ""}`}
                        </ListItem>
                    )
                })}
            </List>

            <ArchitectApprovalPolicy key={tenantId} tenantId={tenantId} />

            {/* The page's own upload: no election-event dry run, no shared import state. */}
            <Drawer
                anchor="right"
                open={openUpload && permitsImport}
                onClose={() => setOpenUpload(false)}
                PaperProps={{"aria-label": String(t("configurationPackage.uploadTitle"))}}
                sx={{"& .MuiDrawer-paper": {width: {xs: "100%", sm: "min(640px, 100%)"}}}}
            >
                <Box sx={{padding: "16px"}} data-testid="package-upload-drawer">
                    <ElectionHeader
                        title="configurationPackage.uploadTitle"
                        subtitle="configurationPackage.uploadSubtitle"
                    />
                    <Typography variant="body2" paragraph>
                        {t("configurationPackage.uploadParagraph")}
                    </Typography>
                    {openUpload && permitsImport && (
                        <ImportScreen
                            key={JSON.stringify([tenantId, authContext.userId])}
                            refresh="configurationPackage.upload"
                            signedDeliveryUpload
                            formatLabel="ZIP / EZIP"
                            doCancel={() => setOpenUpload(false)}
                            onSourceChange={() => {
                                stopFollowing()
                                setConfirming(false)
                            }}
                            doImport={async (
                                document,
                                sha256,
                                _password,
                                encryptedPayloadSha256
                            ) => {
                                setOpenUpload(false)
                                // Blank is no check; anything else must be a digest,
                                // never silently dropped.
                                const entered = sha256.trim()
                                if (entered && !SHA256.test(entered)) {
                                    setPreviewed(null)
                                    setError(t("configurationPackage.shaMalformed"))
                                    return
                                }
                                const sha = entered ? entered.toLowerCase() : null
                                setDocumentId(document)
                                setTypedDocumentId(document)
                                setExpectedSha(sha)
                                // Always a preview first: nothing is applied from an upload.
                                await submit("preview", document, sha, encryptedPayloadSha256)
                            }}
                            disableImport={running || !permitsImport}
                            errors={null}
                        />
                    )}
                </Box>
            </Drawer>

            <Dialog open={confirming} onClose={() => setConfirming(false)}>
                <DialogTitle>{t("configurationPackage.confirmTitle")}</DialogTitle>
                <DialogContent>
                    <Typography variant="body2" paragraph>
                        {t("configurationPackage.confirmBody", {tenantId})}
                    </Typography>
                    {previewed && <ProposedChanges view={previewed.view} />}
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setConfirming(false)}>
                        {t("configurationPackage.cancel")}
                    </Button>
                    <Button
                        color="warning"
                        variant="contained"
                        data-testid="package-apply-confirm"
                        disabled={!canApply}
                        onClick={() => {
                            setConfirming(false)
                            if (documentId && canApply) {
                                submit(
                                    "apply",
                                    documentId,
                                    expectedSha,
                                    submitted?.encryptedPayloadSha256
                                )
                            }
                        }}
                    >
                        {t("configurationPackage.apply")}
                    </Button>
                </DialogActions>
            </Dialog>
        </Box>
    )
}

/** What applying would change, section by section: names, actions and keys only. */
const ProposedChanges: React.FC<{view: ApplicationView}> = ({view}) => {
    const {t} = useTranslation()
    const changing = view.sections.filter((section) => section.preview?.needsApply)
    return (
        <Box data-testid="package-proposed-changes">
            <Typography variant="subtitle2" component="h3">
                {t("configurationPackage.proposedChanges")}
            </Typography>
            {changing.length === 0 ? (
                <Typography variant="body2">{t("configurationPackage.nothingToChange")}</Typography>
            ) : (
                changing.map((section) => (
                    <Box key={section.section} sx={{mt: 1}}>
                        <Typography variant="body2" sx={{fontWeight: "bold"}}>
                            {String(t(`configurationPackage.sections.${section.section}`))}
                        </Typography>
                        <List dense>
                            {changeLines(section.preview?.changes).map((line, index) => (
                                <ListItem key={index}>{line}</ListItem>
                            ))}
                        </List>
                    </Box>
                ))
            )}
        </Box>
    )
}

const ApplicationTable: React.FC<{view: ApplicationView}> = ({view}) => {
    const {t} = useTranslation()
    return (
        <>
            <Table size="small" data-testid="package-sections">
                <TableHead>
                    <TableRow>
                        <TableCell>{t("configurationPackage.columns.section")}</TableCell>
                        <TableCell>{t("configurationPackage.columns.state")}</TableCell>
                        <TableCell>{t("configurationPackage.columns.task")}</TableCell>
                        <TableCell>{t("configurationPackage.columns.details")}</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {view.sections.map((section) => (
                        <TableRow key={section.section} data-testid={`section-${section.section}`}>
                            <TableCell>{section.section}</TableCell>
                            <TableCell>
                                {section.state}
                                {section.reconciled
                                    ? ` (${t("configurationPackage.reconciled")})`
                                    : ""}
                            </TableCell>
                            <TableCell>{section.taskId ?? "—"}</TableCell>
                            <TableCell>
                                {section.preview?.blockers.map((blocker) => (
                                    <div
                                        key={blocker}
                                    >{`${t("configurationPackage.blocker")}: ${blocker}`}</div>
                                ))}
                                {section.preview && section.preview.needsApply ? (
                                    <div>{t("configurationPackage.needsApply")}</div>
                                ) : null}
                                {section.preview
                                    ? changeLines(section.preview.changes).map((line, index) => (
                                          <div key={index}>{line}</div>
                                      ))
                                    : null}
                                {section.error && <div>{safeText(section.error)}</div>}
                                {/* Only a section a run confirmed applied is called applied. */}
                                {section.earlierApplied && section.state !== "applied" ? (
                                    <div>{t("configurationPackage.earlierApplied")}</div>
                                ) : null}
                                {section.submissions.length > 0 && section.state !== "applied" ? (
                                    <div>
                                        {t("configurationPackage.earlierTasks", {
                                            tasks: section.submissions.join(", "),
                                        })}
                                    </div>
                                ) : null}
                                {section.inFlight ? (
                                    <div>
                                        {t("configurationPackage.inFlight", {
                                            taskId: section.inFlight,
                                        })}
                                    </div>
                                ) : null}
                                {section.members.length > 0 && (
                                    <div>{section.members.join(", ")}</div>
                                )}
                            </TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <Typography variant="subtitle2" component="h4" sx={{mt: 1}}>
                {t("configurationPackage.members")}
            </Typography>
            <List dense data-testid="package-members">
                {view.members.map((member, index) => (
                    <ListItem key={`${member.container ?? ""}:${member.path}:${index}`}>
                        {`${member.container ? `${member.container} › ` : ""}${member.path} — ${
                            member.disposition
                        }${member.section ? ` (${member.section})` : ""}`}
                    </ListItem>
                ))}
            </List>
        </>
    )
}
