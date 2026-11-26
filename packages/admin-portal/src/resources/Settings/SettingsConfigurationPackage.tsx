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
    Chip,
    Paper,
    Stack,
    Drawer,
    FormControlLabel,
    List,
    ListItem,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import {
    publishCompletedPackageApply,
    rememberApplyForElectionRefresh,
} from "@/hooks/usePendingConfigurationPackageRefresh"
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
import {PackageImportWorkflow, packageImportPhase} from "./PackageImportWorkflow"
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
    readPackageRetry,
    readPortablePreview,
    readRefusedPreview,
    changeLines,
    hasPortableReplacement,
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
    const runHeadingId = useId()
    const historyHeadingId = useId()
    const confirmTitleId = useId()
    const applyReasonId = useId()
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
    const operatorPreview = useRef<{
        scope: typeof scope.current
        taskId: string
    } | null>(null)
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
            publishCompletedPackageApply(
                {tenantId, actor: authContext.userId},
                submitted.taskId,
                task
            )
        ) {
            refreshedElectionTask.current = submitted.taskId
            rememberSubmission(tenantId, {...submitted, electionsRefreshed: true})
        }
        if (
            submitted.mode === "preview" &&
            view?.mode === "preview" &&
            task?.id === submitted.taskId &&
            task.execution_status === "SUCCESS"
        ) {
            setDocumentId(submitted.documentId)
            setExpectedSha(submitted.expectedSha ?? null)
            setPreviewed({
                tenantId,
                documentId: submitted.documentId,
                requirements: submitted.requirements,
                view,
                taskId: submitted.taskId,
                encryptedPayloadSha256: submitted.encryptedPayloadSha256,
                managedBinding: readManagedPreview(
                    task?.annotations,
                    tenantId,
                    submitted.requirements
                ),
                portablePreview: readPortablePreview(
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
        if (!rosterStateAllowsImport(current) || preview?.portablePreview === "unavailable")
            return false
        const thisOperatorsPreview =
            !!preview?.taskId &&
            !!scope.current.actor &&
            operatorPreview.current?.scope === scope.current &&
            operatorPreview.current.taskId === preview.taskId
        if (current?.mode !== "portal_managed")
            return (
                !preview?.managedBinding &&
                (preview?.portablePreview !== "matched" || thisOperatorsPreview)
            )
        return (
            thisOperatorsPreview &&
            !!preview?.managedBinding &&
            preview.managedBinding.sha256 === current.active_sha256 &&
            preview.managedBinding.generation === current.generation &&
            preview.managedBinding.requiredApprovals === current.required_approvals
        )
    }
    const confirmedInputs = useRef({
        documentId,
        requirements,
        expectedSha,
        allowed,
    })
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
        encryptedPayloadSha256?: string,
        restoredRequirements?: PackageRequirements
    ) => {
        if (packageRequest.current) return
        setError(null)
        if (!authContext.userId || !mayApplyPackages(tenantId, authContext.isAuthorized)) {
            setError(t("configurationPackage.notPermitted"))
            return
        }
        const forTenant = tenantId
        const submittedRequirements =
            mode === "preview" && restoredRequirements ? restoredRequirements : requirements
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
            } else if (mode === "apply" && forPreview?.portablePreview === "matched") {
                // Portable account choices are pinned by the receiver even
                // when verification uses certificate or site-pinned trust.
                previewTaskId = forPreview.taskId
                if (!previewTaskId) return
            }
            operatorPreview.current = null
            const {data, errors} = previewTaskId
                ? await applyManagedConfigurationPackage({
                      variables: {
                          tenantId: forTenant,
                          documentId: document,
                          requirements: submittedRequirements,
                          previewTaskId,
                          ...(encryptedPayloadSha256 ? {encryptedPayloadSha256} : {}),
                      },
                  })
                : await applyConfigurationPackage({
                      variables: {
                          tenantId: forTenant,
                          documentId: document,
                          mode,
                          requirements: submittedRequirements,
                          ...(encryptedPayloadSha256 ? {encryptedPayloadSha256} : {}),
                      },
                  })
            const taskId = data?.apply_configuration_package?.task_execution?.id
            if (mode === "apply" && taskId && !errors?.length && requestScope.actor) {
                rememberApplyForElectionRefresh(
                    {tenantId: forTenant, actor: requestScope.actor},
                    taskId
                )
            }
            if (!mounted.current || scope.current !== requestScope) {
                return
            }
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
                requirements: submittedRequirements,
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

    const retry =
        !unreadable &&
        !view &&
        submitted?.mode === "preview" &&
        submitted.actor === authContext.userId &&
        task?.id === submitted.taskId &&
        task.execution_status === "FAILED" &&
        documentId === submitted.documentId &&
        (!typedDocumentId.trim() || typedDocumentId.trim() === submitted.documentId)
            ? readPackageRetry(task.annotations, tenantId, submitted)
            : null

    const previewOriginalRequirements = () => {
        if (!retry || !submitted || running || !permitsImport || packageRequest.current) return
        changeRequirements(retry.requirements)
        setExpectedSha(retry.packageSha256)
        setConfirming(false)
        void submit(
            "preview",
            submitted.documentId,
            retry.packageSha256,
            submitted.encryptedPayloadSha256,
            retry.requirements
        )
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
    const refusal = !unreadable ? readRefusedPreview(task, submitted) : null
    const blockers = refusal?.blockers ?? previewed?.view.blockers ?? []
    const missingSections = refusal?.missingSections ?? []
    const hasRefusalDetails =
        blockers.length > 0 || missingSections.length > 0 || refusal?.censusMissing
    const knownSections: readonly string[] = PACKAGE_SECTIONS
    const canApply =
        !running &&
        !refusal &&
        mayApply(previewed, tenantId, documentId, requirements, expectedSha) &&
        matchesTrust(roster.status, previewed)
    const shaMismatch = !!previewed && !shaMatches(previewed.view, expectedSha)
    const noRecord = !!submitted && finished && !view
    const phase = packageImportPhase({
        permitsImport,
        rosterLoading: roster.loading,
        submitting,
        running,
        submittedMode: submitted?.mode,
        outcome: view?.outcome,
        canApply,
        hasBlockers: blockers.length > 0,
        previewRefused: !!refusal,
        unreadable,
        noRecord,
        hasError: !!error,
        hasScopeMismatch: !!previewed && !canApply,
    })
    const showApply = canApply || phase === "previewBlocked" || phase === "previewAgain"
    const previewTrustChanged =
        !!previewed &&
        mayApply(previewed, tenantId, documentId, requirements, expectedSha) &&
        !matchesTrust(roster.status, previewed)
    const applyDisabledReason =
        phase === "previewBlocked"
            ? `${applyReasonId}-blocked`
            : shaMismatch
              ? `${applyReasonId}-sha`
              : previewTrustChanged
                ? `${applyReasonId}-trust`
                : undefined

    return (
        <Box sx={{p: {xs: 0, sm: 1}, maxWidth: 1440, mx: "auto", minWidth: 0}}>
            <Stack spacing={1} sx={{mb: 2}}>
                <Typography variant="h4" component="h2" sx={{textAlign: "start", m: 0}}>
                    {t("configurationPackage.title")}
                </Typography>
                <Typography variant="body1" color="text.secondary" sx={{maxWidth: "75ch"}}>
                    {t("configurationPackage.description")}
                </Typography>
                <Typography
                    variant="caption"
                    color="text.secondary"
                    data-testid="package-tenant"
                    sx={{overflowWrap: "anywhere"}}
                >
                    {t("configurationPackage.tenant", {tenantId})}
                </Typography>
            </Stack>

            <Box
                sx={{
                    display: "grid",
                    gridTemplateColumns: {
                        xs: "minmax(0, 1fr)",
                        lg: "minmax(0, 1fr) minmax(280px, 32%)",
                    },
                    alignItems: "start",
                    gap: 2,
                }}
            >
                <Stack spacing={2} sx={{minWidth: 0}}>
                    <PackageImportWorkflow
                        phase={phase}
                        uploadAction={
                            <Button
                                variant={phase === "review" ? "secondary" : "contained"}
                                disabled={running || !permitsImport}
                                aria-describedby={!permitsImport ? rosterStatusId : undefined}
                                sx={{"&&.Mui-disabled": disabledButtonStyle}}
                                onClick={() => {
                                    if (!running && permitsImport) setOpenUpload(true)
                                }}
                            >
                                {t("configurationPackage.upload")}
                            </Button>
                        }
                        applyAction={
                            <Button
                                variant="contained"
                                color="warning"
                                disabled={!canApply}
                                aria-describedby={
                                    showApply && !canApply ? applyDisabledReason : undefined
                                }
                                hidden={!showApply}
                                sx={{
                                    "display": showApply ? undefined : "none",
                                    "&&.Mui-disabled": disabledButtonStyle,
                                }}
                                data-testid="package-apply"
                                onClick={() => setConfirming(true)}
                            >
                                {t("configurationPackage.apply")}
                            </Button>
                        }
                    >
                        {!canApply && (refusal || blockers.length > 0) ? (
                            <Alert
                                severity="warning"
                                role="status"
                                id={`${applyReasonId}-blocked`}
                                data-testid="package-blocked"
                            >
                                <Typography variant="body2" sx={{fontWeight: "medium"}}>
                                    {t(
                                        hasRefusalDetails
                                            ? "configurationPackage.blocked"
                                            : "configurationPackage.workflow.previewRefusalWithoutDetails"
                                    )}
                                </Typography>
                                {hasRefusalDetails && (
                                    <Box component="ul" sx={{m: 0, mt: 0.5, pl: 2.5}}>
                                        {blockers.map((blocker, index) => (
                                            <Typography
                                                component="li"
                                                variant="body2"
                                                key={`${index}:${blocker}`}
                                            >
                                                {safeText(blocker)}
                                            </Typography>
                                        ))}
                                        {missingSections.map((section, index) => (
                                            <Typography
                                                component="li"
                                                variant="body2"
                                                key={`missing:${index}:${section}`}
                                            >
                                                {t("configurationPackage.workflow.missingSection", {
                                                    section: knownSections.includes(section)
                                                        ? t(
                                                              `configurationPackage.sections.${section}`
                                                          )
                                                        : safeText(section),
                                                })}
                                            </Typography>
                                        ))}
                                        {refusal?.censusMissing && (
                                            <Typography component="li" variant="body2">
                                                {t("configurationPackage.workflow.missingCensus")}
                                            </Typography>
                                        )}
                                    </Box>
                                )}
                            </Alert>
                        ) : null}
                        {previewTrustChanged && (
                            <Alert severity="warning" role="status" id={`${applyReasonId}-trust`}>
                                {t("configurationPackage.roster.previewAgain")}
                            </Alert>
                        )}
                        {shaMismatch && (
                            <Alert
                                severity="error"
                                sx={{overflowWrap: "anywhere"}}
                                data-testid="package-sha-mismatch"
                                id={`${applyReasonId}-sha`}
                            >
                                {t("configurationPackage.shaMismatch", {expected: expectedSha})}
                            </Alert>
                        )}
                        {error && (
                            <Alert
                                severity="error"
                                sx={{overflowWrap: "anywhere"}}
                                data-testid="package-error"
                            >
                                {safeText(error)}
                            </Alert>
                        )}
                    </PackageImportWorkflow>
                    {submitted && (
                        <Paper
                            component="section"
                            variant="outlined"
                            aria-labelledby={runHeadingId}
                            sx={{p: 2, minWidth: 0, overflowWrap: "anywhere"}}
                            data-testid="package-run"
                        >
                            <Typography id={runHeadingId} variant="h6" component="h3">
                                {t("configurationPackage.workflow.result")}
                            </Typography>
                            <Typography variant="subtitle1" component="h4" sx={{mt: 1}}>
                                {t(`configurationPackage.mode.${submitted.mode}`)}
                            </Typography>
                            <Typography
                                variant="body2"
                                data-testid="package-outcome"
                                hidden={!!retry}
                                aria-live="polite"
                                aria-atomic="true"
                                sx={{fontWeight: "medium"}}
                            >
                                {unreadable
                                    ? t("configurationPackage.unreadable", {
                                          reason: taskError?.message
                                              ? safeText(taskError.message)
                                              : t("configurationPackage.notFound"),
                                      })
                                    : noRecord
                                      ? t("configurationPackage.stoppedWithoutRecord")
                                      : finished
                                        ? t(
                                              `configurationPackage.outcome.${view?.outcome ?? "unknown"}`
                                          )
                                        : t("configurationPackage.running")}
                            </Typography>
                            {retry && (
                                <Alert
                                    severity="warning"
                                    sx={{mt: 1}}
                                    data-testid="package-retry-requirements"
                                >
                                    <Typography variant="body2" data-testid="package-retry-outcome">
                                        {t("configurationPackage.retryRequirements.refused")}
                                    </Typography>
                                    <Typography variant="body2">
                                        {t("configurationPackage.retryRequirements.original", {
                                            sections:
                                                retry.requirements.sections
                                                    .map((section) =>
                                                        t(
                                                            `configurationPackage.sections.${section}`
                                                        )
                                                    )
                                                    .join(", ") ||
                                                t("configurationPackage.retryRequirements.none"),
                                        })}
                                    </Typography>
                                    <Typography variant="body2">
                                        {t(
                                            `configurationPackage.retryRequirements.${retry.requirements.census}`
                                        )}
                                    </Typography>
                                    <Button
                                        variant="outlined"
                                        size="small"
                                        sx={{mt: 1}}
                                        disabled={running || !permitsImport}
                                        onClick={previewOriginalRequirements}
                                    >
                                        {t("configurationPackage.retryRequirements.preview")}
                                    </Button>
                                </Alert>
                            )}
                            {noRecord && !retry && lastLogLine(task?.logs) && (
                                <Typography variant="body2" data-testid="package-last-log">
                                    {t("configurationPackage.lastLog", {
                                        message: safeText(lastLogLine(task?.logs) ?? ""),
                                    })}
                                </Typography>
                            )}
                            {view && finished && view.outcome === "previewed" && (
                                <Typography variant="body2">
                                    {t("configurationPackage.toChange", {
                                        count: sectionsToChange(view),
                                    })}
                                </Typography>
                            )}
                            {view && (
                                <Box sx={{my: 1.5}}>
                                    <ApplicationTable view={view} />
                                </Box>
                            )}
                            <Typography variant="body2" data-testid="package-task">
                                {t("configurationPackage.task", {taskId: submitted.taskId})}
                            </Typography>
                            <Button
                                size="small"
                                sx={{mt: 1}}
                                data-testid="package-stop-following"
                                onClick={stopFollowing}
                            >
                                {t("configurationPackage.stopFollowing")}
                            </Button>
                        </Paper>
                    )}
                    <Paper
                        component="section"
                        variant="outlined"
                        aria-labelledby={historyHeadingId}
                        sx={{p: 2, minWidth: 0}}
                    >
                        <Typography id={historyHeadingId} variant="h6" component="h3">
                            {t("configurationPackage.history")}
                        </Typography>
                        <ApplicationHistory rows={historyData?.sequent_backend_tasks_execution} />
                    </Paper>
                </Stack>
                <Stack spacing={2} sx={{minWidth: 0}}>
                    {roster.status?.mode !== "not_enabled" && (
                        <Paper
                            component="section"
                            variant="outlined"
                            aria-label={String(t("configurationPackage.workflow.trust"))}
                            sx={{px: 2, pb: 2, minWidth: 0}}
                        >
                            <ArchitectSignerRoster
                                tenantId={tenantId}
                                status={roster.status}
                                loading={roster.loading}
                                statusTextId={rosterStatusId}
                                refresh={roster.read}
                            />
                        </Paper>
                    )}
                    <Paper
                        component="section"
                        variant="outlined"
                        aria-labelledby={advancedHeadingId}
                        sx={{p: 2, minWidth: 0}}
                        data-testid="package-advanced"
                    >
                        {/* Uncontrolled: closed by default, never opened, closed or reset from state. */}
                        <Box component="details" data-testid="package-advanced-details">
                            <Box
                                component="summary"
                                sx={{
                                    "cursor": "pointer",
                                    "borderRadius": 1,
                                    "&:focus-visible": {
                                        outline: (theme) =>
                                            `2px solid ${theme.palette.primary.main}`,
                                        outlineOffset: 2,
                                    },
                                }}
                            >
                                <Typography id={advancedHeadingId} component="span" variant="h6">
                                    {t("configurationPackage.advanced")}
                                </Typography>
                                <Typography
                                    component="span"
                                    variant="body2"
                                    color="text.secondary"
                                    sx={{display: "block"}}
                                >
                                    {t("configurationPackage.workflow.optionsSummary", {
                                        selected: requirements.sections.length,
                                        available: PACKAGE_SECTIONS.length,
                                    })}
                                    {" · "}
                                    {t(
                                        requirements.census === "required"
                                            ? "configurationPackage.workflow.censusRequired"
                                            : "configurationPackage.workflow.censusOptional"
                                    )}
                                </Typography>
                            </Box>
                            <Box sx={{my: 2, display: "grid"}}>
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
                                        label={String(
                                            t(`configurationPackage.sections.${section}`)
                                        )}
                                    />
                                ))}
                                <FormControlLabel
                                    control={
                                        <Checkbox
                                            checked={requirements.census === "required"}
                                            onChange={(event) =>
                                                changeRequirements({
                                                    ...requirements,
                                                    census: event.target.checked
                                                        ? "required"
                                                        : "optional",
                                                })
                                            }
                                            inputProps={{"aria-label": "census"}}
                                        />
                                    }
                                    label={String(t("configurationPackage.censusRequired"))}
                                />
                            </Box>
                            {showManualEntry && (
                                <Box
                                    sx={{
                                        display: "flex",
                                        gap: 2,
                                        alignItems: "center",
                                        flexWrap: "wrap",
                                    }}
                                >
                                    <TextField
                                        size="small"
                                        fullWidth
                                        label={String(
                                            t(
                                                roster.status?.mode === "portal_managed"
                                                    ? "configurationPackage.signedDocumentId"
                                                    : "configurationPackage.documentId"
                                            )
                                        )}
                                        helperText={String(
                                            t("configurationPackage.existingDocumentHelp")
                                        )}
                                        slotProps={{
                                            formHelperText: {
                                                sx: {"&.Mui-disabled": {color: "text.secondary"}},
                                            },
                                        }}
                                        disabled={running || !permitsImport}
                                        value={typedDocumentId}
                                        onChange={(event) => {
                                            setPreviewed(null)
                                            setTypedDocumentId(event.target.value)
                                        }}
                                    />
                                    <Button
                                        variant="outlined"
                                        size="small"
                                        disabled={
                                            running || !permitsImport || !typedDocumentId.trim()
                                        }
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
                                            <ConfigurationSigningSettings
                                                key={tenantId}
                                                tenantId={tenantId}
                                            />
                                        )}
                                    </Box>
                                </Box>
                            )}
                        </Box>
                    </Paper>
                    <Paper variant="outlined" sx={{px: 2, minWidth: 0}}>
                        <ArchitectApprovalPolicy key={tenantId} tenantId={tenantId} />
                    </Paper>
                </Stack>
            </Box>

            {/* The page's own upload: no election-event dry run, no shared import state. */}
            <Drawer
                anchor="right"
                open={openUpload && permitsImport}
                onClose={() => setOpenUpload(false)}
                PaperProps={{
                    "aria-label": String(t("configurationPackage.uploadTitle")),
                }}
                sx={{
                    "& .MuiDrawer-paper": {
                        width: {xs: "100%", sm: "min(640px, 100%)"},
                    },
                }}
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
                            // The signed-in session's own token, for the direct
                            // same-origin unwrap of packages with credentials.
                            getAccessToken={authContext.getAccessToken}
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

            <Dialog
                open={confirming}
                onClose={() => setConfirming(false)}
                aria-labelledby={confirmTitleId}
                fullWidth
                maxWidth="sm"
            >
                <DialogTitle id={confirmTitleId}>
                    {t("configurationPackage.confirmTitle")}
                </DialogTitle>
                <DialogContent>
                    <Typography variant="body2" paragraph>
                        {t("configurationPackage.confirmBody", {tenantId})}
                    </Typography>
                    {previewed && <ProposedChanges view={previewed.view} />}
                </DialogContent>
                <DialogActions>
                    <Button
                        variant="cancel"
                        autoFocus
                        disableFocusRipple
                        onClick={() => setConfirming(false)}
                    >
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
                                    previewed?.encryptedPayloadSha256
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

const PortableReplacementWarning: React.FC<{changes: unknown}> = ({changes}) => {
    const {t} = useTranslation()
    return hasPortableReplacement(changes) ? (
        <Alert severity="warning" sx={{my: 1}}>
            {t("configurationPackage.messagingReplaceWarning")}
        </Alert>
    ) : null
}

/** What applying would change, section by section: names, actions and keys only. */
const ProposedChanges: React.FC<{view: ApplicationView}> = ({view}) => {
    const {t} = useTranslation()
    const changing = view.sections.filter(
        (section) =>
            section.preview?.needsApply ||
            (section.section === "messaging-accounts" && !!section.preview?.changes)
    )
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
                        {section.section === "messaging-accounts" && (
                            <PortableReplacementWarning changes={section.preview?.changes} />
                        )}
                        <List dense>
                            {changeLines(
                                section.preview?.changes,
                                t("configurationPackage.messagingReplaceAction")
                            ).map((line, index) => (
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
    const namedSections: readonly string[] = PACKAGE_SECTIONS
    const rowLabelSx = {
        position: "sticky",
        insetInlineStart: 0,
        zIndex: 1,
        bgcolor: "background.paper",
    } as const
    return (
        <>
            <TableContainer
                component={Paper}
                variant="outlined"
                tabIndex={0}
                role="region"
                aria-label={String(t("configurationPackage.workflow.result"))}
                sx={{maxInlineSize: "100%"}}
            >
                <Table size="small" data-testid="package-sections">
                    <TableHead>
                        <TableRow>
                            <TableCell sx={rowLabelSx}>
                                {t("configurationPackage.columns.section")}
                            </TableCell>
                            <TableCell>{t("configurationPackage.columns.state")}</TableCell>
                            <TableCell>{t("configurationPackage.columns.task")}</TableCell>
                            <TableCell>{t("configurationPackage.columns.details")}</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {view.sections.map((section) => (
                            <TableRow
                                key={section.section}
                                data-testid={`section-${section.section}`}
                                sx={{verticalAlign: "top"}}
                            >
                                <TableCell component="th" scope="row" sx={rowLabelSx}>
                                    {namedSections.includes(section.section)
                                        ? String(
                                              t(`configurationPackage.sections.${section.section}`)
                                          )
                                        : section.section}
                                </TableCell>
                                <TableCell>
                                    {/* Only a stored applied state is green; anything else stays neutral. */}
                                    <Chip
                                        size="small"
                                        variant={
                                            section.state === "applied" ? "filled" : "outlined"
                                        }
                                        color={section.state === "applied" ? "success" : "default"}
                                        label={`${section.state}${
                                            section.reconciled
                                                ? ` (${t("configurationPackage.reconciled")})`
                                                : ""
                                        }`}
                                        sx={{
                                            "blockSize": "auto",
                                            "& .MuiChip-label": {
                                                whiteSpace: "normal",
                                                overflowWrap: "anywhere",
                                            },
                                        }}
                                    />
                                </TableCell>
                                <TableCell sx={{fontFamily: "monospace", overflowWrap: "anywhere"}}>
                                    {section.taskId ?? "—"}
                                </TableCell>
                                <TableCell sx={(theme) => ({minInlineSize: theme.spacing(30)})}>
                                    <Box
                                        sx={(theme) => ({
                                            display: "grid",
                                            rowGap: theme.spacing(0.5),
                                            overflowWrap: "anywhere",
                                        })}
                                    >
                                        {Array.from(new Set(section.preview?.blockers ?? [])).map(
                                            (blocker) => (
                                                <div key={blocker}>{`${t(
                                                    "configurationPackage.blocker"
                                                )}: ${safeText(blocker)}`}</div>
                                            )
                                        )}
                                        {section.preview && section.preview.needsApply ? (
                                            <div>{t("configurationPackage.needsApply")}</div>
                                        ) : null}
                                        {view.mode === "preview" &&
                                            section.section === "messaging-accounts" && (
                                                <PortableReplacementWarning
                                                    changes={section.preview?.changes}
                                                />
                                            )}
                                        {section.preview
                                            ? changeLines(
                                                  section.preview.changes,
                                                  t("configurationPackage.messagingReplaceAction")
                                              ).map((line, index) => <div key={index}>{line}</div>)
                                            : null}
                                        {section.error && (
                                            <Typography
                                                variant="body2"
                                                component="div"
                                                color="error"
                                            >
                                                {safeText(section.error)}
                                            </Typography>
                                        )}
                                        {/* Only a section a run confirmed applied is called applied. */}
                                        {section.earlierApplied && section.state !== "applied" ? (
                                            <div>{t("configurationPackage.earlierApplied")}</div>
                                        ) : null}
                                        {section.submissions.length > 0 &&
                                        section.state !== "applied" ? (
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
                                    </Box>
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
            {/* Native disclosure: keyboard-operable, and the list stays mounted while closed. */}
            <Box component="details" sx={(theme) => ({marginBlockStart: theme.spacing(1)})}>
                <Typography
                    component="summary"
                    variant="body2"
                    sx={(theme) => ({
                        "cursor": "pointer",
                        "inlineSize": "fit-content",
                        "maxInlineSize": "100%",
                        "paddingBlock": theme.spacing(0.5),
                        "paddingInline": theme.spacing(0.5),
                        "fontWeight": theme.typography.fontWeightMedium,
                        "&:focus-visible": {
                            outline: `2px solid ${theme.palette.primary.main}`,
                            outlineOffset: theme.spacing(0.25),
                        },
                    })}
                >
                    {t("configurationPackage.workflow.evidence", {count: view.members.length})}
                </Typography>
                <Typography
                    variant="subtitle2"
                    component="h4"
                    sx={(theme) => ({marginBlockStart: theme.spacing(1)})}
                >
                    {t("configurationPackage.members")}
                </Typography>
                <List dense data-testid="package-members">
                    {view.members.map((member, index) => (
                        <ListItem
                            key={`${member.container ?? ""}:${member.path}:${index}`}
                            sx={{overflowWrap: "anywhere"}}
                        >
                            {`${member.container ? `${member.container} › ` : ""}${
                                member.path
                            } — ${member.disposition}${
                                member.section ? ` (${member.section})` : ""
                            }`}
                        </ListItem>
                    ))}
                </List>
            </Box>
        </>
    )
}

/** Earlier runs in the order the query returned them: only what each row already stores. */
const ApplicationHistory: React.FC<{
    rows:
        | readonly {
              id: string
              created_at: string
              annotations?: unknown
              execution_status?: string | null
          }[]
        | undefined
}> = ({rows}) => {
    const {t} = useTranslation()
    return (
        <>
            <List
                dense
                data-testid="package-history"
                aria-label={String(t("configurationPackage.history"))}
            >
                {(rows ?? []).map((row, index) => {
                    const record = readApplication(row.annotations)
                    const mode: unknown = record?.mode
                    const outcome = record?.outcome ?? row.execution_status
                    return (
                        <ListItem
                            key={`${row.id}:${index}`}
                            divider
                            disableGutters
                            sx={(theme) => ({
                                display: "flex",
                                flexWrap: "wrap",
                                alignItems: "center",
                                columnGap: theme.spacing(1.5),
                                rowGap: theme.spacing(0.5),
                            })}
                        >
                            {/* The stored timestamp as-is: no locale is guessed. */}
                            <Typography
                                variant="body2"
                                component="time"
                                dateTime={row.created_at}
                                color="text.secondary"
                            >
                                {row.created_at}
                            </Typography>
                            <Typography
                                variant="body2"
                                component="span"
                                sx={{fontFamily: "monospace", overflowWrap: "anywhere"}}
                            >
                                {t("configurationPackage.task", {taskId: row.id})}
                            </Typography>
                            <Typography
                                variant="body2"
                                component="span"
                                sx={(theme) => ({fontWeight: theme.typography.fontWeightMedium})}
                            >
                                {mode === "preview" || mode === "apply"
                                    ? String(t(`configurationPackage.mode.${mode}`))
                                    : typeof mode === "string" && mode
                                      ? mode
                                      : "?"}
                            </Typography>
                            {/* Only a stored applied outcome is green; a bare execution status stays neutral. */}
                            <Chip
                                size="small"
                                variant={record?.outcome === "applied" ? "filled" : "outlined"}
                                color={record?.outcome === "applied" ? "success" : "default"}
                                label={String(outcome ?? "?")}
                                sx={{
                                    "blockSize": "auto",
                                    "& .MuiChip-label": {
                                        whiteSpace: "normal",
                                        overflowWrap: "anywhere",
                                    },
                                }}
                            />
                            {typeof record?.packageSha256 === "string" && record.packageSha256 ? (
                                <Typography
                                    variant="body2"
                                    component="span"
                                    color="text.secondary"
                                    sx={{fontFamily: "monospace"}}
                                >
                                    {record.packageSha256.slice(0, 12)}
                                </Typography>
                            ) : null}
                        </ListItem>
                    )
                })}
            </List>
            {/* Empty only once the query has answered; undefined rows say nothing. */}
            {rows && rows.length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                    {t("configurationPackage.workflow.noHistory")}
                </Typography>
            ) : null}
        </>
    )
}
