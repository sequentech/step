// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Import a complete configuration package (the delivery zip Election
// Architect exports, carried on removable media) into this tenant: always a
// preview first, then the application of exactly what was previewed, each
// through the platform's apply_configuration_package task. Applying runs the
// server's own preview again before anything changes; this page never skips it
// and never resubmits on its own.

import React, {useContext, useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useMutation, useQuery} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    Checkbox,
    Divider,
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
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {
    APPLY_CONFIGURATION_PACKAGE,
    GET_PACKAGE_TASK,
    LIST_PACKAGE_APPLICATIONS,
} from "@/queries/ApplyConfigurationPackage"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {
    ActiveSubmission,
    ApplicationView,
    ApplyMode,
    COMPLETE_DELIVERY,
    PACKAGE_SECTIONS,
    PackageRequirements,
    PackageSection,
    PreviewedPackage,
    isFinished,
    mayApply,
    mayApplyPackages,
    readApplication,
    recalledSubmission,
    rememberSubmission,
    sectionsToChange,
} from "./configurationPackage"

const POLL_INTERVAL_MS = 3000

export const SettingsConfigurationPackage: React.FC = () => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    // Checked on every render, and again just before every submission.
    const allowed = mayApplyPackages(tenantId, authContext.isAuthorized)

    const [requirements, setRequirements] = useState<PackageRequirements>(COMPLETE_DELIVERY)
    const [documentId, setDocumentId] = useState<string | null>(null)
    const [typedDocumentId, setTypedDocumentId] = useState("")
    const [openUpload, setOpenUpload] = useState(false)
    // A reload in the same tenant resumes the task it was following.
    const [submitted, setSubmitted] = useState<ActiveSubmission | null>(() =>
        recalledSubmission(tenantId)
    )
    const [previewed, setPreviewed] = useState<PreviewedPackage | null>(null)
    const [error, setError] = useState<string | null>(null)
    const [submitting, setSubmitting] = useState(false)

    // Another tenant is not what was previewed, nor the task it follows.
    useEffect(() => {
        setPreviewed(null)
        setSubmitted(recalledSubmission(tenantId))
    }, [tenantId])

    const [applyConfigurationPackage] = useMutation(APPLY_CONFIGURATION_PACKAGE, {
        context: {headers: {"x-hasura-role": IPermissions.TENANT_WRITE}},
    })

    const {
        data: taskData,
        startPolling,
        stopPolling,
    } = useQuery(GET_PACKAGE_TASK, {
        variables: {taskId: submitted?.taskId, tenantId},
        skip: !submitted || !tenantId || !allowed,
        pollInterval: POLL_INTERVAL_MS,
        fetchPolicy: "network-only",
    })
    const task = taskData?.sequent_backend_tasks_execution?.[0]
    const finished = isFinished(task?.execution_status)
    const view = useMemo(() => readApplication(task?.annotations), [task?.annotations])

    // Each submission is polled until its task finishes.
    useEffect(() => {
        if (submitted) {
            startPolling(POLL_INTERVAL_MS)
        }
    }, [submitted?.taskId, startPolling])

    // A finished task stops polling; a finished preview is what Apply applies.
    useEffect(() => {
        if (!submitted || !finished || !tenantId) {
            return
        }
        stopPolling()
        if (submitted.mode === "preview" && view) {
            setDocumentId(submitted.documentId)
            setPreviewed({
                tenantId,
                documentId: submitted.documentId,
                requirements: submitted.requirements,
                view,
            })
        }
    }, [submitted, finished, view, stopPolling, tenantId])

    const {data: historyData} = useQuery(LIST_PACKAGE_APPLICATIONS, {
        variables: {tenantId},
        skip: !allowed || !tenantId,
    })

    if (!allowed || !tenantId) {
        return (
            <Alert severity="info" data-testid="package-not-permitted">
                {t("configurationPackage.notPermitted")}
            </Alert>
        )
    }

    // One submission per click: a failure or an unknown result is reported and
    // left for the operator, never resubmitted here.
    const submit = async (mode: ApplyMode, document: string) => {
        setError(null)
        if (!mayApplyPackages(tenantId, authContext.isAuthorized)) {
            setError(t("configurationPackage.notPermitted"))
            return
        }
        setSubmitting(true)
        if (mode === "preview") {
            setPreviewed(null)
        }
        try {
            const {data, errors} = await applyConfigurationPackage({
                variables: {tenantId, documentId: document, mode, requirements},
            })
            const taskId = data?.apply_configuration_package?.task_execution?.id
            if (errors?.length || !taskId) {
                setError(
                    errors?.map((e) => e.message).join(", ") || t("configurationPackage.noTask")
                )
                return
            }
            const active = {taskId, mode, documentId: document, requirements}
            rememberSubmission(tenantId, active)
            setSubmitted(active)
        } catch (e) {
            setError(String(e instanceof Error ? e.message : e))
        } finally {
            setSubmitting(false)
        }
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

    const running = submitting || (!!submitted && !finished)
    const canApply = !running && mayApply(previewed, tenantId, documentId, requirements)

    return (
        <Box sx={{p: 1}}>
            <Typography className="title" variant="h4">
                {t("configurationPackage.title")}
            </Typography>
            <Typography variant="body1" paragraph>
                {t("configurationPackage.description")}
            </Typography>
            <Typography variant="body2" data-testid="package-tenant">
                {t("configurationPackage.tenant", {tenantId})}
            </Typography>

            <Divider sx={{my: 2}} />
            <Typography variant="h6">{t("configurationPackage.requirements")}</Typography>
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

            <Divider sx={{my: 2}} />
            <Box sx={{display: "flex", gap: 2, alignItems: "center", flexWrap: "wrap"}}>
                <Button variant="contained" disabled={running} onClick={() => setOpenUpload(true)}>
                    {t("configurationPackage.upload")}
                </Button>
                <TextField
                    size="small"
                    label={String(t("configurationPackage.documentId"))}
                    value={typedDocumentId}
                    onChange={(event) => {
                        setPreviewed(null)
                        setTypedDocumentId(event.target.value)
                    }}
                />
                <Button
                    variant="outlined"
                    disabled={running || !typedDocumentId.trim()}
                    onClick={() => {
                        const document = typedDocumentId.trim()
                        setDocumentId(document)
                        submit("preview", document)
                    }}
                >
                    {t("configurationPackage.preview")}
                </Button>
                <Button
                    variant="contained"
                    color="warning"
                    disabled={!canApply}
                    data-testid="package-apply"
                    onClick={() => documentId && submit("apply", documentId)}
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
            {error && (
                <Alert severity="error" sx={{mt: 1}} data-testid="package-error">
                    {error}
                </Alert>
            )}

            {submitted && (
                <Box sx={{mt: 2}} data-testid="package-run">
                    <Typography variant="h6">
                        {t(`configurationPackage.mode.${submitted.mode}`)}
                    </Typography>
                    <Typography variant="body2" data-testid="package-task">
                        {t("configurationPackage.task", {taskId: submitted.taskId})}
                    </Typography>
                    <Typography variant="body2" data-testid="package-outcome">
                        {finished
                            ? t(`configurationPackage.outcome.${view?.outcome ?? "unknown"}`)
                            : t("configurationPackage.running")}
                    </Typography>
                    {view && finished && view.outcome === "previewed" && (
                        <Typography variant="body2">
                            {t("configurationPackage.toChange", {count: sectionsToChange(view)})}
                        </Typography>
                    )}
                    {view && <ApplicationTable view={view} />}
                </Box>
            )}

            <Divider sx={{my: 2}} />
            <Typography variant="h6">{t("configurationPackage.history")}</Typography>
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

            <ImportDataDrawer
                open={openUpload}
                closeDrawer={() => setOpenUpload(false)}
                title="configurationPackage.uploadTitle"
                subtitle="configurationPackage.uploadSubtitle"
                paragraph="configurationPackage.uploadParagraph"
                doImport={async (document: string) => {
                    setOpenUpload(false)
                    setDocumentId(document)
                    setTypedDocumentId(document)
                    // Always a preview first: nothing is applied from an upload.
                    await submit("preview", document)
                }}
                errors={null}
            />
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
                                {section.error && <div>{section.error}</div>}
                                {section.earlierApplied && section.state !== "applied" ? (
                                    <div>
                                        {t("configurationPackage.earlierApplied", {
                                            tasks: section.submissions.join(", ") || "—",
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
            <Typography variant="subtitle2" sx={{mt: 1}}>
                {t("configurationPackage.members")}
            </Typography>
            <List dense data-testid="package-members">
                {view.members.map((member) => (
                    <ListItem key={`${member.disposition}:${member.path}`}>
                        {`${member.path} — ${member.disposition}${member.section ? ` (${member.section})` : ""}`}
                    </ListItem>
                ))}
            </List>
        </>
    )
}
