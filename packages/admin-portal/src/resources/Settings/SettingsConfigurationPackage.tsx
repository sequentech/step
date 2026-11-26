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
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Chip,
    Paper,
    Stack,
    Drawer,
    List,
    ListItem,
    ListItemButton,
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
import {packageImportPhase} from "./PackageImportWorkflow"
import {
    type HubTab,
    PackageRail,
    PackageStatusHeader,
    RequirementsPicker,
    RequirementsSummary,
    RunStages,
    SectionBadgeChip,
    TenantReceipt,
    WizardSteps,
    isSectionTab,
} from "./PackageHub"
import {
    type HeaderAction,
    type TrustBlock,
    changingSections,
    headerActions,
    isConfirmedApply,
    isUnconfirmedApplied,
    lastAppliedReceipt,
    missingPackagePermissions,
    orderedSectionBadges,
    readRecovery,
    readRunIdentity,
    runStages,
    type CurrentTrust,
    type Recovery,
} from "./packageOverview"
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
    sameRequirements,
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

/** First and last eight hex digits of a digest, for a summary line. */
const shortDigest = (digest: string | null | undefined): string | undefined =>
    typeof digest === "string" && digest.length > 20
        ? `${digest.slice(0, 8)} … ${digest.slice(-8)}`
        : undefined

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
    const [tab, setTab] = useState<HubTab>("overview")
    // The run card on Overview carries progress, errors and recovery: an error
    // raised from the header, a drawer or a dialog on another tab brings it forward.
    useEffect(() => {
        if (error) setTab("overview")
    }, [error])
    const [drawerRequirements, setDrawerRequirements] = useState(false)
    // Continuing a stopped Apply by choosing the same file again: its identity and
    // requirements, locked while the drawer is open for it.
    const [recoveryDraft, setRecoveryDraft] = useState<{
        taskId: string
        packageSha256: string
        requirements: PackageRequirements
    } | null>(null)
    // "Change requirements" and the document-ID menu item open the options in place.
    const [openAdvanced, setOpenAdvanced] = useState(false)
    const advancedRef = useRef<HTMLDetailsElement | null>(null)
    const runCardHeadingId = useId()
    const sectionsHeadingId = useId()
    const sectionPanelHeadingId = useId()
    useEffect(() => {
        if (!openAdvanced) return
        const details = advancedRef.current
        if (details) {
            details.open = true
            details.querySelector("summary")?.focus()
        }
        setOpenAdvanced(false)
    }, [openAdvanced, tab])

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
        const missing = authContext.userId
            ? missingPackagePermissions(tenantId, authContext.isAuthorized)
            : []
        return (
            <Alert severity="info" data-testid="package-not-permitted">
                <Typography variant="body2" sx={{fontWeight: 500}}>
                    {t("configurationPackage.hub.noAccess.title")}
                </Typography>
                {missing.length > 0 ? (
                    <Typography variant="body2" data-testid="package-missing-permissions">
                        {t("configurationPackage.hub.noAccess.missing", {
                            permissions: missing.join(", "),
                        })}
                    </Typography>
                ) : (
                    <Typography variant="body2">
                        {t("configurationPackage.notPermitted")}
                    </Typography>
                )}
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
        restoredRequirements?: PackageRequirements,
        recoveryOf?: string
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
        // Whatever tab it was started from, the run is followed on Overview.
        setTab("overview")
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
                ...(recoveryOf && mode === "preview" ? {recoveryOf} : {}),
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

    const setCensus = (census: PackageRequirements["census"]) => {
        if (requirementsLocked) return
        changeRequirements({...requirements, census})
    }

    const toggleSection = (section: PackageSection) =>
        !requirementsLocked &&
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
    // The followed Preview ran with other requirements than the ones now selected:
    // what it found is that run's history, not what blocks the current choice.
    const previewStale =
        !!submitted &&
        submitted.mode === "preview" &&
        finished &&
        !!view &&
        !sameRequirements(submitted.requirements, requirements)
    const phase = packageImportPhase({
        permitsImport,
        rosterLoading: roster.loading,
        submitting,
        running,
        submittedMode: submitted?.mode,
        outcome: view?.outcome,
        executionStatus: task?.execution_status,
        canApply,
        hasBlockers: blockers.length > 0,
        previewRefused: !!refusal,
        unreadable,
        noRecord,
        hasError: !!error,
        hasScopeMismatch: !!previewed && !canApply,
        previewStale,
    })
    // A blocked preview offers Import fixed package and Change requirements, never Apply.
    const showApply = canApply || phase === "previewAgain"
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
                : phase === "previewAgain"
                  ? refusal || blockers.length > 0
                      ? `${applyReasonId}-blocked`
                      : `${applyReasonId}-stale`
                  : undefined
    // An out-of-date Preview's disabled Apply says why, when nothing more specific does.
    const showStaleReason =
        phase === "previewAgain" &&
        !shaMismatch &&
        !previewTrustChanged &&
        !refusal &&
        blockers.length === 0

    const openDocumentId = () => {
        setTab("overview")
        setOpenAdvanced(true)
    }
    const openRequirements = () => {
        if (running || !permitsImport) return
        setDrawerRequirements(true)
        setOpenUpload(true)
    }
    // The same uploaded document this operator previewed, with the current
    // requirements: a new Preview the platform verifies from scratch. Not offered
    // when the platform read different bytes than the entered SHA-256.
    const canPreviewAgain =
        phase === "previewAgain" &&
        !running &&
        permitsImport &&
        !shaMismatch &&
        submitted?.mode === "preview" &&
        submitted.actor === authContext.userId &&
        !!documentId &&
        submitted.documentId === documentId &&
        (!submitted.recoveryOf || !!submitted.expectedSha)
    const previewAgain = () => {
        if (!canPreviewAgain || !submitted || !documentId) return
        const pinnedSha = submitted.recoveryOf ? (submitted.expectedSha ?? null) : expectedSha
        // Old or incomplete local follow state must never make recovery unpinned.
        if (submitted.recoveryOf && !pinnedSha) return
        setConfirming(false)
        void submit(
            "preview",
            documentId,
            pinnedSha,
            submitted.encryptedPayloadSha256,
            submitted.recoveryOf ? submitted.requirements : undefined,
            submitted.recoveryOf
        )
    }
    const actionFor = (wanted: HeaderAction, primary: boolean) => {
        const variant = primary ? "contained" : "outlined"
        switch (wanted) {
            case "reviewAndApply":
                return null // The Apply button below is the one Review and apply control.
            case "previewRemaining":
                return (
                    <Button
                        key="previewRemaining"
                        variant={variant}
                        data-testid="package-preview-remaining"
                        onClick={startRecovery}
                    >
                        {t("configurationPackage.hub.recovery.previewRemaining")}
                    </Button>
                )
            case "openTrust":
                return (
                    <Button
                        key="openTrust"
                        variant={variant}
                        data-testid="package-open-trust"
                        onClick={() => setTab("trust")}
                    >
                        {t("configurationPackage.hub.actions.openTrust")}
                    </Button>
                )
            case "checkAgain":
                return (
                    <Button
                        key="checkAgain"
                        variant={variant}
                        disabled={roster.loading}
                        data-testid="package-check-trust-again"
                        onClick={() => void roster.read()}
                    >
                        {t("configurationPackage.hub.actions.checkAgain")}
                    </Button>
                )
            case "previewAgain":
                return (
                    <Button
                        key="previewAgain"
                        variant={variant}
                        data-testid="package-preview-again"
                        onClick={previewAgain}
                    >
                        {t("configurationPackage.hub.actions.previewAgain")}
                    </Button>
                )
            case "changeRequirements":
                return (
                    <Button
                        key="requirements"
                        variant={variant}
                        data-testid="package-change-requirements"
                        onClick={openRequirements}
                    >
                        {t("configurationPackage.hub.actions.changeRequirements")}
                    </Button>
                )
            default:
                return (
                    <Button
                        key="upload"
                        variant={variant}
                        disabled={running || !permitsImport}
                        aria-describedby={!permitsImport ? rosterStatusId : undefined}
                        data-testid="package-upload"
                        sx={{"&&.Mui-disabled": disabledButtonStyle}}
                        onClick={() => {
                            if (running || !permitsImport) return
                            setDrawerRequirements(false)
                            setRecoveryDraft(null)
                            setOpenUpload(true)
                        }}
                    >
                        {t(
                            wanted === "importFixed"
                                ? "configurationPackage.hub.actions.importFixed"
                                : phase === "review"
                                  ? "configurationPackage.hub.actions.importAnother"
                                  : "configurationPackage.hub.actions.importPackage"
                        )}
                    </Button>
                )
        }
    }
    // Without the same followed document to preview again, the way on is a new import.
    // Only a managed roster can be missing; anything else that blocks is a failed check.
    const trustBlock: TrustBlock =
        roster.status?.mode === "portal_managed" ? "missingRoster" : "checkFailed"
    const suggested = headerActions(phase, trustBlock)
    // The roster the receiver enforces now; only a portal-managed one binds a run.
    const currentTrust: CurrentTrust =
        roster.status?.mode === "portal_managed" && roster.status.active_sha256
            ? {
                  managed: true,
                  binding: {
                      sha256: roster.status.active_sha256,
                      generation: roster.status.generation,
                      requiredApprovals: roster.status.required_approvals,
                  },
              }
            : {managed: roster.status?.mode === "portal_managed", binding: null}
    const recovery = readRecovery(
        historyData?.sequent_backend_tasks_execution,
        tenantId,
        currentTrust,
        submitted
    )
    // Offered only while nothing else is in flight or under review.
    const canStartRecovery =
        recovery.kind === "recoverable" &&
        !running &&
        permitsImport &&
        (phase === "attention" || phase === "choose")
    const actions: {primary: HeaderAction; secondary?: HeaderAction} = canStartRecovery
        ? {primary: "previewRemaining", secondary: "upload"}
        : suggested.primary === "previewAgain" && !canPreviewAgain
          ? {primary: "upload"}
          : suggested
    // A fresh Preview of the same signed package with its original requirements:
    // never Apply, never a stale preview, one request at a time.
    const startRecovery = () => {
        if (!canStartRecovery || recovery.kind !== "recoverable" || packageRequest.current) return
        setConfirming(false)
        setPreviewed(null)
        if (recovery.documentId) {
            setRequirements(recovery.requirements)
            setDocumentId(recovery.documentId)
            setExpectedSha(recovery.packageSha256)
            void submit(
                "preview",
                recovery.documentId,
                recovery.packageSha256,
                undefined,
                recovery.requirements,
                recovery.taskId
            )
            return
        }
        setRecoveryDraft({
            taskId: recovery.taskId,
            packageSha256: recovery.packageSha256,
            requirements: recovery.requirements,
        })
        setDrawerRequirements(false)
        setOpenUpload(true)
    }
    const requirementsLocked = !!recoveryDraft || !!submitted?.recoveryOf
    const trustSummary =
        phase === "waitingForTrust"
            ? {label: String(t("configurationPackage.hub.trustChecking"))}
            : !permitsImport
              ? {label: String(t("configurationPackage.hub.trustBlocked"))}
              : roster.status?.mode === "portal_managed"
                ? {
                      label: String(
                          t("configurationPackage.hub.trustManaged", {
                              generation: roster.status.generation,
                          })
                      ),
                      detail: shortDigest(roster.status.active_sha256),
                  }
                : roster.status?.mode === "site_pinned"
                  ? {
                        label: String(t("configurationPackage.hub.trustPinned")),
                        detail: shortDigest(roster.status.active_sha256),
                    }
                  : {label: String(t("configurationPackage.hub.trustLegacy"))}
    const shownView = submitted && !unreadable ? view : null
    const sectionRows = orderedSectionBadges(shownView, running, task?.execution_status)
    const stages = runStages(phase, submitted?.mode)
    const receipt = lastAppliedReceipt(historyData?.sequent_backend_tasks_execution)
    const runCard = (
        <Paper
            component="section"
            variant="outlined"
            aria-labelledby={runCardHeadingId}
            data-testid="package-run-card"
            sx={{p: 2, minWidth: 0}}
        >
            <Stack spacing={1.5}>
                <Stack
                    direction="row"
                    spacing={1}
                    sx={{alignItems: "center", justifyContent: "space-between"}}
                >
                    <Typography id={runCardHeadingId} variant="subtitle2" component="h3">
                        {t(
                            submitted
                                ? "configurationPackage.hub.currentRun"
                                : "configurationPackage.hub.lastRun"
                        )}
                    </Typography>
                    {submitted && (
                        <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{fontFamily: "monospace", overflowWrap: "anywhere"}}
                        >
                            {submitted.taskId}
                        </Typography>
                    )}
                </Stack>
                <RunStages stages={stages} />
                {!submitted && (
                    <Typography variant="body2" color="text.secondary">
                        {t("configurationPackage.hub.noRun")}
                    </Typography>
                )}
                {canStartRecovery && recovery.kind === "recoverable" && (
                    <Alert severity="info" data-testid="package-recovery">
                        <Typography variant="body2" sx={{fontWeight: 500}}>
                            {t("configurationPackage.hub.recovery.title")}
                        </Typography>
                        <Typography variant="body2">
                            {t("configurationPackage.hub.recovery.remaining", {
                                sections: recovery.remaining
                                    .map((section) => t(`configurationPackage.sections.${section}`))
                                    .join(", "),
                            })}
                        </Typography>
                        <Typography variant="body2">
                            {t("configurationPackage.hub.recovery.explain")}
                        </Typography>
                    </Alert>
                )}
                {recovery.kind === "blocked" &&
                    !running &&
                    (phase === "attention" || phase === "choose") && (
                        <Alert severity="warning" data-testid="package-recovery-blocked">
                            {t(`configurationPackage.hub.recovery.blocked.${recovery.reason}`)}
                        </Alert>
                    )}
                {showStaleReason && (
                    <Alert
                        severity="info"
                        role="status"
                        id={`${applyReasonId}-stale`}
                        data-testid="package-stale-reason"
                    >
                        {t("configurationPackage.hub.staleReason")}
                    </Alert>
                )}
                {!canApply && (refusal || blockers.length > 0) ? (
                    <Alert
                        severity={previewStale ? "info" : "warning"}
                        role="status"
                        id={`${applyReasonId}-blocked`}
                        data-testid="package-blocked"
                        data-historical={previewStale ? "true" : undefined}
                    >
                        {previewStale && (
                            <Typography
                                variant="body2"
                                sx={{fontWeight: "medium"}}
                                data-testid="package-blocked-historical"
                            >
                                {t("configurationPackage.hub.historicalBlockers")}
                            </Typography>
                        )}
                        <Typography
                            variant="body2"
                            sx={{fontWeight: previewStale ? undefined : "medium"}}
                        >
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
                                                ? t(`configurationPackage.sections.${section}`)
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
            </Stack>
        </Paper>
    )

    const overview = (
        <Box
            sx={{
                display: "grid",
                gridTemplateColumns: {
                    xs: "minmax(0, 1fr)",
                    lg: "minmax(0, 1.3fr) minmax(300px, 1fr)",
                },
                alignItems: "start",
                gap: 1.5,
            }}
        >
            {/* First in reading order, so a phone shows why before the section list;
                on wide screens it is placed in the right column. */}
            <Stack
                spacing={1.5}
                sx={{minWidth: 0, gridColumn: {lg: 2}, gridRow: {lg: 1}}}
                data-testid="package-overview-run"
            >
                {runCard}
                <TenantReceipt
                    receipt={receipt}
                    partialTaskId={
                        recovery.kind !== "none" && recovery.applied.length
                            ? recovery.taskId
                            : undefined
                    }
                    onHistory={() => setTab("history")}
                />
            </Stack>
            <Stack
                spacing={1.5}
                sx={{minWidth: 0, gridColumn: {lg: 1}, gridRow: {lg: 1}}}
                data-testid="package-overview-sections"
            >
                <Paper
                    component="section"
                    variant="outlined"
                    aria-labelledby={sectionsHeadingId}
                    sx={{p: 2, pb: 1, minWidth: 0}}
                    data-testid="package-section-rows"
                >
                    <Typography id={sectionsHeadingId} variant="subtitle2" component="h3">
                        {t("configurationPackage.hub.sectionsHeading")}
                    </Typography>
                    {previewStale && (
                        <Typography
                            variant="caption"
                            color="text.secondary"
                            data-testid="package-sections-historical"
                        >
                            {t("configurationPackage.hub.historicalSections")}
                        </Typography>
                    )}
                    <List component="div" dense disablePadding>
                        {sectionRows.map(({section, badge, view: row}, index) => {
                            const summary = row
                                ? [
                                      ...(row.preview?.blockers ?? []),
                                      ...(row.error ? [row.error] : []),
                                  ]
                                      .map(safeText)
                                      .join(" · ")
                                : ""
                            return (
                                <ListItemButton
                                    key={section}
                                    divider={index < sectionRows.length - 1}
                                    data-testid={`package-section-row-${section}`}
                                    onClick={() => setTab(section)}
                                    sx={{px: 0.5, gap: 1.5, alignItems: "center"}}
                                >
                                    <Typography
                                        component="span"
                                        variant="caption"
                                        color="text.secondary"
                                        aria-hidden="true"
                                        sx={{
                                            width: 22,
                                            height: 22,
                                            flex: "0 0 22px",
                                            border: 1,
                                            borderColor: "divider",
                                            borderRadius: "50%",
                                            display: "inline-flex",
                                            alignItems: "center",
                                            justifyContent: "center",
                                        }}
                                    >
                                        {index + 1}
                                    </Typography>
                                    <Box sx={{flex: "1 1 auto", minWidth: 0}}>
                                        <Typography variant="body2" sx={{fontWeight: 500}} noWrap>
                                            {t(`configurationPackage.sections.${section}`)}
                                        </Typography>
                                        {summary && (
                                            <Typography
                                                variant="caption"
                                                color="text.secondary"
                                                component="div"
                                                noWrap
                                                title={summary}
                                            >
                                                {summary}
                                            </Typography>
                                        )}
                                    </Box>
                                    <SectionBadgeChip badge={badge} />
                                </ListItemButton>
                            )
                        })}
                    </List>
                </Paper>
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
                                    ? isUnconfirmedApplied(view, task?.execution_status)
                                        ? t("configurationPackage.hub.appliedNotConfirmed")
                                        : t(
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
                                                    t(`configurationPackage.sections.${section}`)
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
                                <ApplicationTable
                                    view={view}
                                    executionStatus={task?.execution_status}
                                />
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
                    aria-labelledby={advancedHeadingId}
                    sx={{p: 2, minWidth: 0}}
                    data-testid="package-advanced"
                >
                    {/* Uncontrolled: closed by default; only Change requirements opens it, never state. */}
                    <Box
                        component="details"
                        ref={advancedRef}
                        data-testid="package-advanced-details"
                    >
                        <Box
                            component="summary"
                            sx={{
                                "cursor": "pointer",
                                "borderRadius": 1,
                                "&:focus-visible": {
                                    outline: (theme) => `2px solid ${theme.palette.primary.main}`,
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
                                <RequirementsSummary requirements={requirements} />
                            </Typography>
                        </Box>
                        <Box sx={{my: 2}}>
                            <RequirementsPicker
                                requirements={requirements}
                                onToggleSection={toggleSection}
                                onCensus={setCensus}
                                disabled={requirementsLocked}
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
            </Stack>
        </Box>
    )

    const sectionPanel = isSectionTab(tab) ? (
        <Paper
            component="section"
            variant="outlined"
            aria-labelledby={sectionPanelHeadingId}
            data-testid={`package-section-panel-${tab}`}
            sx={{p: 2, minWidth: 0}}
        >
            <Stack
                direction="row"
                spacing={2}
                sx={{alignItems: "flex-start", justifyContent: "space-between", mb: 1.5}}
            >
                <Box>
                    <Typography variant="caption" color="text.secondary">
                        {t("configurationPackage.hub.sectionOf", {
                            n: PACKAGE_SECTIONS.indexOf(tab) + 1,
                            total: PACKAGE_SECTIONS.length,
                        })}
                    </Typography>
                    <Typography id={sectionPanelHeadingId} variant="h6" component="h3">
                        {t(`configurationPackage.sections.${tab}`)}
                    </Typography>
                </Box>
                <SectionBadgeChip
                    badge={sectionRows.find((row) => row.section === tab)?.badge ?? "notRun"}
                />
            </Stack>
            {shownView && shownView.sections.some((s) => s.section === tab) ? (
                <ApplicationTable
                    view={{
                        ...shownView,
                        sections: shownView.sections.filter((s) => s.section === tab),
                        members: shownView.members.filter((m) => m.section === tab),
                    }}
                    executionStatus={task?.execution_status}
                />
            ) : (
                <Typography variant="body2" color="text.secondary">
                    {t("configurationPackage.hub.sectionNotInRun")}
                </Typography>
            )}
        </Paper>
    ) : null

    const trustPanel = (
        <Stack spacing={1.5} sx={{minWidth: 0}} data-testid="package-trust-panel">
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
            <Paper variant="outlined" sx={{px: 2, minWidth: 0}}>
                <ArchitectApprovalPolicy key={tenantId} tenantId={tenantId} />
            </Paper>
        </Stack>
    )

    const historyPanel = (
        <Paper
            component="section"
            variant="outlined"
            aria-labelledby={historyHeadingId}
            sx={{p: 2, minWidth: 0}}
        >
            <Typography id={historyHeadingId} variant="h6" component="h3">
                {t("configurationPackage.history")}
            </Typography>
            <ApplicationHistory
                rows={historyData?.sequent_backend_tasks_execution}
                recovery={recovery}
                canContinue={canStartRecovery}
                onContinue={startRecovery}
            />
        </Paper>
    )

    return (
        <Box sx={{p: {xs: 0, sm: 1}, maxWidth: 1440, mx: "auto", minWidth: 0}}>
            <Stack spacing={0.5} sx={{mb: 1.5}}>
                <Typography variant="h5" component="h2" sx={{textAlign: "start", m: 0}}>
                    {t("configurationPackage.title")}
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

            <PackageStatusHeader
                phase={phase}
                trust={trustSummary}
                // The roster panel carries this id while Signing trust is open.
                trustTextId={
                    tab === "trust" && roster.status?.mode !== "not_enabled"
                        ? undefined
                        : rosterStatusId
                }
                onTrust={() => setTab("trust")}
                actions={
                    <>
                        {actions.secondary && actionFor(actions.secondary, false)}
                        {actionFor(actions.primary, true)}
                        <Button
                            variant="contained"
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
                            {t("configurationPackage.hub.actions.reviewAndApply")}
                        </Button>
                    </>
                }
                menu={[
                    {
                        label: String(t("configurationPackage.hub.actions.documentId")),
                        onClick: openDocumentId,
                        disabled: running || !permitsImport || !showManualEntry,
                    },
                    {
                        label: String(t("configurationPackage.hub.actions.stopFollowing")),
                        onClick: stopFollowing,
                        disabled: !submitted,
                    },
                    {
                        label: String(t("configurationPackage.hub.actions.history")),
                        onClick: () => setTab("history"),
                    },
                ]}
            />

            <Box
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", md: "row"},
                    alignItems: {md: "flex-start"},
                    gap: 1.5,
                    mt: 1.5,
                    minWidth: 0,
                }}
            >
                <PackageRail
                    tab={tab}
                    onTab={setTab}
                    sections={sectionRows}
                    trustBlocked={!permitsImport && !roster.loading}
                />
                <Box sx={{flex: "1 1 auto", minWidth: 0}}>
                    {tab === "overview" && overview}
                    {sectionPanel}
                    {tab === "trust" && trustPanel}
                    {tab === "history" && historyPanel}
                </Box>
            </Box>

            {/* The page's own upload: no election-event dry run, no shared import state. */}
            <Drawer
                anchor="right"
                open={openUpload && permitsImport}
                onClose={() => {
                    setOpenUpload(false)
                    setDrawerRequirements(false)
                    setRecoveryDraft(null)
                }}
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
                    <Box sx={{mb: 2}}>
                        <WizardSteps current={0} />
                    </Box>
                    <Typography variant="body2" paragraph>
                        {t("configurationPackage.uploadParagraph")}
                    </Typography>
                    {recoveryDraft && (
                        <Alert
                            severity="info"
                            sx={{mb: 2, overflowWrap: "anywhere"}}
                            data-testid="package-recovery-identity"
                        >
                            <Typography variant="body2" sx={{fontWeight: 500}}>
                                {t("configurationPackage.hub.recovery.chooseSame")}
                            </Typography>
                            <Typography variant="body2" sx={{fontFamily: "monospace"}}>
                                {recoveryDraft.packageSha256}
                            </Typography>
                            <Typography variant="body2">
                                <RequirementsSummary requirements={recoveryDraft.requirements} />
                                {" · "}
                                {t("configurationPackage.hub.recovery.locked")}
                            </Typography>
                        </Alert>
                    )}
                    <Paper
                        variant="outlined"
                        sx={{p: 1.5, mb: 2, display: recoveryDraft ? "none" : undefined}}
                        data-testid="package-drawer-requirements"
                    >
                        <Stack
                            direction="row"
                            spacing={1}
                            sx={{alignItems: "center", justifyContent: "space-between"}}
                        >
                            <Typography variant="body2">
                                <RequirementsSummary requirements={requirements} />
                            </Typography>
                            <Button
                                size="small"
                                aria-expanded={drawerRequirements}
                                onClick={() => setDrawerRequirements(!drawerRequirements)}
                                data-testid="package-drawer-requirements-toggle"
                            >
                                {t(
                                    drawerRequirements
                                        ? "configurationPackage.hub.actions.doneRequirements"
                                        : "configurationPackage.hub.actions.changeRequirements"
                                )}
                            </Button>
                        </Stack>
                        {drawerRequirements && (
                            <Box sx={{mt: 1}}>
                                <RequirementsPicker
                                    requirements={requirements}
                                    onToggleSection={toggleSection}
                                    onCensus={setCensus}
                                    disabled={running || requirementsLocked}
                                    testId="package-requirements-picker"
                                />
                                {canPreviewAgain && (
                                    <Button
                                        variant="contained"
                                        sx={{mt: 1}}
                                        data-testid="package-drawer-preview-again"
                                        onClick={() => {
                                            setOpenUpload(false)
                                            setDrawerRequirements(false)
                                            previewAgain()
                                        }}
                                    >
                                        {t("configurationPackage.hub.actions.previewAgainSame")}
                                    </Button>
                                )}
                            </Box>
                        )}
                    </Paper>
                    {openUpload && permitsImport && (
                        <ImportScreen
                            key={JSON.stringify([tenantId, authContext.userId])}
                            refresh="configurationPackage.upload"
                            signedDeliveryUpload
                            // The signed-in session's own token, for the direct
                            // same-origin unwrap of packages with credentials.
                            getAccessToken={authContext.getAccessToken}
                            formatLabel="ZIP / EZIP"
                            doCancel={() => {
                                setOpenUpload(false)
                                setDrawerRequirements(false)
                                setRecoveryDraft(null)
                            }}
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
                                const draft = recoveryDraft
                                setRecoveryDraft(null)
                                if (draft) {
                                    // The same signed package or nothing: an entered or an
                                    // authenticated digest that differs stops here.
                                    const other = [entered, encryptedPayloadSha256].some(
                                        (digest) =>
                                            !!digest && digest.toLowerCase() !== draft.packageSha256
                                    )
                                    if (other) {
                                        setPreviewed(null)
                                        setError(t("configurationPackage.hub.recovery.wrongFile"))
                                        return
                                    }
                                    setRequirements(draft.requirements)
                                    setDocumentId(document)
                                    setTypedDocumentId(document)
                                    // The Preview must read exactly the run's package.
                                    setExpectedSha(draft.packageSha256)
                                    await submit(
                                        "preview",
                                        document,
                                        draft.packageSha256,
                                        encryptedPayloadSha256,
                                        draft.requirements,
                                        draft.taskId
                                    )
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
                    {previewed && (
                        <Box data-testid="package-confirm-identity" sx={{mb: 2}}>
                            <Box
                                component="dl"
                                sx={{
                                    display: "grid",
                                    gridTemplateColumns: "max-content minmax(0, 1fr)",
                                    columnGap: 2,
                                    rowGap: 0.5,
                                    m: 0,
                                    mb: 1.5,
                                    typography: "body2",
                                }}
                            >
                                <Box component="dt" sx={{color: "text.secondary"}}>
                                    {t("configurationPackage.hub.receipt.sha")}
                                </Box>
                                <Box
                                    component="dd"
                                    sx={{m: 0, fontFamily: "monospace", overflowWrap: "anywhere"}}
                                >
                                    {previewed.view.packageSha256 ?? "?"}
                                </Box>
                                <Box component="dt" sx={{color: "text.secondary"}}>
                                    {t("configurationPackage.requirements")}
                                </Box>
                                <Box component="dd" sx={{m: 0}}>
                                    <RequirementsSummary requirements={previewed.requirements} />
                                </Box>
                                {previewed.managedBinding && (
                                    <>
                                        <Box component="dt" sx={{color: "text.secondary"}}>
                                            {t("configurationPackage.hub.signerRoster")}
                                        </Box>
                                        <Box component="dd" sx={{m: 0}}>
                                            {t("configurationPackage.hub.rosterBinding", {
                                                generation: previewed.managedBinding.generation,
                                                approvals:
                                                    previewed.managedBinding.requiredApprovals,
                                            })}
                                        </Box>
                                    </>
                                )}
                            </Box>
                            {changingSections(previewed.view).length > 0 && (
                                <>
                                    <Typography variant="body2">
                                        {t("configurationPackage.hub.confirmOrder")}
                                    </Typography>
                                    <Box
                                        component="ol"
                                        data-testid="package-confirm-order"
                                        sx={{mt: 0.5, mb: 1, pl: 3, typography: "body2"}}
                                    >
                                        {changingSections(previewed.view).map((section) => (
                                            <li key={section}>
                                                {t(`configurationPackage.sections.${section}`)}
                                            </li>
                                        ))}
                                    </Box>
                                </>
                            )}
                            <Typography variant="body2" color="text.secondary">
                                {t("configurationPackage.hub.confirmPartial")}
                            </Typography>
                            {submitted?.recoveryOf && (
                                <Typography
                                    variant="body2"
                                    sx={{mt: 1}}
                                    data-testid="package-confirm-recovery"
                                >
                                    {t("configurationPackage.hub.recovery.confirmKeep")}
                                </Typography>
                            )}
                        </Box>
                    )}
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
    const changing = changingSections(view).flatMap((name) =>
        view.sections.filter((section) => section.section === name)
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

const ApplicationTable: React.FC<{
    view: ApplicationView
    /** The run's task execution_status, for the confirmed-apply rule. */
    executionStatus: string | null | undefined
}> = ({view, executionStatus}) => {
    const {t} = useTranslation()
    // A record that claims an applied run its task row does not confirm shows no
    // section as applied; a partially failed run keeps its genuinely applied ones.
    const unconfirmed = isUnconfirmedApplied(view, executionStatus)
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
                aria-label={String(t("configurationPackage.sectionResults"))}
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
                                {/* One line per state: the table scrolls in its own region on phones. */}
                                <TableCell
                                    sx={{whiteSpace: "nowrap", minInlineSize: "max-content"}}
                                >
                                    {/* Only a confirmed applied state is green; anything else stays neutral. */}
                                    <Chip
                                        size="small"
                                        variant={
                                            section.state === "applied" && !unconfirmed
                                                ? "filled"
                                                : "outlined"
                                        }
                                        color={
                                            section.state === "applied" && !unconfirmed
                                                ? "success"
                                                : "default"
                                        }
                                        label={`${section.state}${
                                            section.reconciled
                                                ? ` (${t("configurationPackage.reconciled")})`
                                                : ""
                                        }${
                                            section.state === "applied" && unconfirmed
                                                ? ` · ${t("configurationPackage.hub.badge.notConfirmed")}`
                                                : ""
                                        }`}
                                        data-testid={`section-state-${section.section}`}
                                        sx={{"& .MuiChip-label": {whiteSpace: "nowrap"}}}
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
              executed_by_user?: string | null
          }[]
        | undefined
    recovery: Recovery
    canContinue: boolean
    onContinue: () => void
}> = ({rows, recovery, canContinue, onContinue}) => {
    const {t} = useTranslation()
    const [selectedId, setSelectedId] = useState<string | null>(null)
    const list = rows ?? []
    const selected = list.find((row) => row.id === selectedId) ?? list[0]
    const record = selected ? readApplication(selected.annotations) : null
    const identity = selected ? readRunIdentity(selected.annotations) : null
    const modeLabel = (mode: unknown) =>
        mode === "preview" || mode === "apply"
            ? String(t(`configurationPackage.mode.${mode}`))
            : typeof mode === "string" && mode
              ? mode
              : "?"
    const detail: [string, string, boolean?][] = selected
        ? [
              [
                  String(t("configurationPackage.hub.historyOutcome")),
                  isUnconfirmedApplied(record, selected.execution_status)
                      ? String(t("configurationPackage.hub.appliedNotConfirmed"))
                      : String(record?.outcome ?? selected.execution_status ?? "?"),
              ],
              [String(t("configurationPackage.hub.historyStarted")), selected.created_at],
              ...(selected.executed_by_user
                  ? ([
                        [
                            String(t("configurationPackage.hub.receipt.by")),
                            selected.executed_by_user,
                        ],
                    ] as [string, string][])
                  : []),
              ...(identity?.packageSha256
                  ? ([
                        [
                            String(t("configurationPackage.hub.receipt.sha")),
                            identity.packageSha256,
                            true,
                        ],
                    ] as [string, string, boolean][])
                  : []),
              ...(identity?.roster
                  ? ([
                        [
                            String(t("configurationPackage.hub.signerRoster")),
                            String(
                                t("configurationPackage.hub.rosterBinding", {
                                    generation: identity.roster.generation,
                                    approvals: identity.roster.requiredApprovals,
                                })
                            ),
                        ],
                    ] as [string, string][])
                  : []),
              [String(t("configurationPackage.hub.receipt.task")), selected.id, true],
          ]
        : []
    return (
        <>
            {/* Empty only once the query has answered; undefined rows say nothing. */}
            {rows && rows.length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                    {t("configurationPackage.workflow.noHistory")}
                </Typography>
            ) : null}
            <Box
                sx={{
                    display: "grid",
                    gridTemplateColumns: {xs: "minmax(0, 1fr)", md: "280px minmax(0, 1fr)"},
                    gap: 1.5,
                    alignItems: "start",
                }}
            >
                <List
                    component="div"
                    dense
                    data-testid="package-history"
                    aria-label={String(t("configurationPackage.history"))}
                >
                    {list.map((row, index) => {
                        const rowRecord = readApplication(row.annotations)
                        // Green and "applied" only for a confirmed application.
                        const confirmed = isConfirmedApply(rowRecord, row.execution_status)
                        const outcome = isUnconfirmedApplied(rowRecord, row.execution_status)
                            ? String(t("configurationPackage.hub.appliedNotConfirmed"))
                            : (rowRecord?.outcome ?? row.execution_status)
                        const isSelected = row.id === selected?.id
                        return (
                            <ListItemButton
                                key={`${row.id}:${index}`}
                                selected={isSelected}
                                aria-current={isSelected ? "true" : undefined}
                                onClick={() => setSelectedId(row.id)}
                                data-testid={`package-history-row-${row.id}`}
                                sx={{
                                    borderRadius: 1,
                                    display: "flex",
                                    flexWrap: "wrap",
                                    alignItems: "center",
                                    columnGap: 1.5,
                                    rowGap: 0.5,
                                }}
                            >
                                <Box sx={{flex: "1 1 auto", minWidth: 0}}>
                                    <Typography
                                        variant="body2"
                                        component="span"
                                        sx={{display: "block", fontWeight: 500}}
                                    >
                                        {modeLabel(rowRecord?.mode)}
                                    </Typography>
                                    {/* The stored timestamp as-is: no locale is guessed. */}
                                    <Typography
                                        variant="caption"
                                        component="time"
                                        dateTime={row.created_at}
                                        color="text.secondary"
                                    >
                                        {row.created_at}
                                    </Typography>
                                    <Typography
                                        variant="caption"
                                        component="span"
                                        color="text.secondary"
                                        sx={{
                                            display: "block",
                                            fontFamily: "monospace",
                                            overflowWrap: "anywhere",
                                        }}
                                    >
                                        {t("configurationPackage.task", {taskId: row.id})}
                                    </Typography>
                                </Box>
                                {/* Only a confirmed application is green; anything else stays neutral. */}
                                <Chip
                                    size="small"
                                    variant={confirmed ? "filled" : "outlined"}
                                    color={confirmed ? "success" : "default"}
                                    label={String(outcome ?? "?")}
                                    sx={{
                                        "blockSize": "auto",
                                        "& .MuiChip-label": {
                                            whiteSpace: "normal",
                                            overflowWrap: "anywhere",
                                        },
                                    }}
                                />
                            </ListItemButton>
                        )
                    })}
                </List>
                {selected && (
                    <Paper
                        variant="outlined"
                        sx={{p: 2, minWidth: 0}}
                        data-testid="package-history-detail"
                    >
                        <Stack
                            direction="row"
                            spacing={1}
                            sx={{alignItems: "center", justifyContent: "space-between", mb: 1}}
                        >
                            <Typography variant="subtitle2" component="h4">
                                {modeLabel(record?.mode)}
                            </Typography>
                            {/* Only the latest run, when it is a recoverable stopped Apply. */}
                            {recovery.kind === "recoverable" && recovery.taskId === selected.id && (
                                <Button
                                    variant="outlined"
                                    size="small"
                                    disabled={!canContinue}
                                    data-testid="package-history-continue"
                                    onClick={onContinue}
                                >
                                    {t("configurationPackage.hub.recovery.continue")}
                                </Button>
                            )}
                        </Stack>
                        {recovery.kind === "blocked" && recovery.taskId === selected.id && (
                            <Alert
                                severity="warning"
                                sx={{mb: 1}}
                                data-testid="package-history-recovery-blocked"
                            >
                                {t(`configurationPackage.hub.recovery.blocked.${recovery.reason}`)}
                            </Alert>
                        )}
                        <Box
                            component="dl"
                            sx={{
                                display: "grid",
                                gridTemplateColumns: "max-content minmax(0, 1fr)",
                                columnGap: 2,
                                rowGap: 0.5,
                                m: 0,
                                typography: "body2",
                            }}
                        >
                            {detail.map(([label, value, mono]) => (
                                <React.Fragment key={label}>
                                    <Box component="dt" sx={{color: "text.secondary"}}>
                                        {label}
                                    </Box>
                                    <Box
                                        component="dd"
                                        sx={{
                                            m: 0,
                                            minWidth: 0,
                                            overflowWrap: "anywhere",
                                            fontFamily: mono ? "monospace" : undefined,
                                        }}
                                    >
                                        {value}
                                    </Box>
                                </React.Fragment>
                            ))}
                            {identity?.requirements && (
                                <>
                                    <Box component="dt" sx={{color: "text.secondary"}}>
                                        {t("configurationPackage.requirements")}
                                    </Box>
                                    <Box component="dd" sx={{m: 0}}>
                                        <RequirementsSummary requirements={identity.requirements} />
                                    </Box>
                                </>
                            )}
                        </Box>
                        {record && (
                            <List dense disablePadding sx={{mt: 1}}>
                                {orderedSectionBadges(record, false, selected.execution_status).map(
                                    ({section, badge}, index) => (
                                        <ListItem
                                            key={section}
                                            disableGutters
                                            sx={{gap: 1, justifyContent: "space-between"}}
                                        >
                                            <Typography variant="body2">
                                                {index + 1}.{" "}
                                                {t(`configurationPackage.sections.${section}`)}
                                            </Typography>
                                            <SectionBadgeChip badge={badge} />
                                        </ListItem>
                                    )
                                )}
                            </List>
                        )}
                    </Paper>
                )}
            </Box>
        </>
    )
}
