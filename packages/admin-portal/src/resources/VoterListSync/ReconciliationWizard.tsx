// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {useMutation, useQuery} from "@apollo/client"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Divider,
    IconButton,
    Stack,
    Typography,
} from "@mui/material"
import CloseIcon from "@mui/icons-material/Close"
import {DropFile} from "@sequentech/ui-essentials"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {SyncDiffTable} from "./SyncDiffTable"
import {CategorySummary} from "./CategorySummary"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {ESyncChangeCategory, SyncDiffRow} from "./types"
import {HIGHLIGHTED_CATEGORIES, CATEGORY_LABELS} from "./constants"
import {formatGeneratedAt} from "./utils"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {CREATE_EXTERNAL_RECONCILIATION_IMPORT} from "@/queries/CreateExternalReconciliationImport"
import {APPLY_EXTERNAL_RECONCILIATION_CHANGES} from "@/queries/ApplyExternalReconciliationChanges"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"

type WizardStep = "drop" | "processing" | "review" | "applying" | "done"

interface ReconciliationWizardProps {
    open: boolean
    electionEventId: string
    onClose: () => void
}

const POLL_INTERVAL_MS = 3000

interface RawDiffItem {
    voter_username: string
    target: "datafix" | "sequent"
    category: ESyncChangeCategory
    field?: string | null
    old_value?: string | null
    new_value?: string | null
    failure_reason?: string | null
}

/**
 * Content of the diff-envelope document `generate_reconciliation_patches`
 * uploads, referenced from the generate task_execution's own
 * `annotations.document_id` — there is no `datafix_reconciliation_import`
 * table/row anymore, so this document, fetched and parsed client-side, is
 * the sole source for the review step.
 */
interface ReconciliationDiffEnvelope {
    sequence: number
    generated_at: number
    datafix_patch_document_id: string | null
    items: RawDiffItem[]
}

const summarizeForConfirmation = (rows: SyncDiffRow[]): string => {
    const counts = new Map<ESyncChangeCategory, number>()
    rows.forEach((row) => counts.set(row.category, (counts.get(row.category) ?? 0) + 1))

    const phrases = [
        [ESyncChangeCategory.VOTED_OTHER_CHANNEL, "marks", "voter(s) as voted via other channels"],
        [ESyncChangeCategory.DISABLED, "disables", "voter(s)"],
        [ESyncChangeCategory.REENABLED, "re-enables", "voter(s)"],
        [ESyncChangeCategory.PROFILE_UPDATE, "updates", "profile(s)"],
        [ESyncChangeCategory.VOTER_ADDED, "adds", "voter(s)"],
    ] as const

    const parts = phrases
        .map(([category, verb, noun]) => {
            const count = counts.get(category) ?? 0
            return count > 0 ? `${verb} ${count} ${noun}` : null
        })
        .filter((part): part is string => part !== null)

    return parts.length > 0
        ? `This will apply changes that ${parts.join(", ")}.`
        : "There are no Sequent-side changes to apply."
}

const countsByCategory = (rows: SyncDiffRow[]): Partial<Record<ESyncChangeCategory, number>> => {
    const counts: Partial<Record<ESyncChangeCategory, number>> = {}
    rows.forEach((row) => {
        counts[row.category] = (counts[row.category] ?? 0) + 1
    })
    return counts
}

const toRows = (items: RawDiffItem[], target: "datafix" | "sequent"): SyncDiffRow[] =>
    items
        .filter(
            (item) => item.target === target && item.category !== ESyncChangeCategory.ROW_FAILURE
        )
        .map((item, index) => ({
            id: `${item.voter_username}:${item.field ?? ""}:${index}`,
            voterId: item.voter_username,
            field: item.field ?? "",
            label: item.field ?? "",
            oldValue: item.old_value ?? "NONE",
            newValue: item.new_value ?? "NONE",
            category: item.category,
            target: item.target,
        }))

const toRowFailures = (items: RawDiffItem[]): SyncDiffRow[] =>
    items
        .filter((item) => item.category === ESyncChangeCategory.ROW_FAILURE)
        .map((item, index) => ({
            id: `failure:${item.voter_username}:${index}`,
            voterId: item.voter_username,
            // `field` is null when the failure (e.g. a CountyMun mismatch)
            // doesn't correspond to any Sequent field - falls back to a
            // generic label instead of a blank Field column.
            field: item.field ?? "Row",
            label: item.field ?? "Row",
            oldValue: item.old_value ?? "NONE",
            newValue: item.new_value ?? "NONE",
            category: item.category,
            target: item.target,
            failureReason: item.failure_reason ?? undefined,
        }))

/**
 * Reconciliation wizard: Drop the reconciliation file, both diffs (Datafix-side, Sequent-side) are
 * calculated at once and shown in separate tables. The Datafix patch is
 * downloadable as soon as its diff is non-empty; Apply is disabled until that
 * diff is empty (clean import), then applies the Sequent-side diff directly -
 * there is no Sequent patch file to download (the Sequent-side items exist
 * only as an internal document apply_reconciliation_patch reads from). There
 * is no separate "convergence check" mode (D8): re-checking an
 * already-applied Sequence just recomputes an empty Sequent-side diff, so
 * "Next" naturally has nothing to apply.
 *
 * There is no `datafix_reconciliation_import` table: the whole diff is a
 * document referenced from the generate task_execution's own
 * `annotations.document_id`, fetched (via the existing `fetchDocument`
 * action, the same one downloads already use) and parsed client-side once.
 */
export const ReconciliationWizard: React.FC<ReconciliationWizardProps> = ({
    open,
    electionEventId,
    onClose,
}) => {
    const [addWidget, setWidgetTaskId] = useWidgetStore()

    const [step, setStep] = useState<WizardStep>("drop")
    const [fileName, setFileName] = useState<string | null>(null)
    const [errorMessage, setErrorMessage] = useState<string | null>(null)
    const [diffDocumentId, setDiffDocumentId] = useState<string | null>(null)
    const [envelope, setEnvelope] = useState<ReconciliationDiffEnvelope | null>(null)
    const [generateTaskId, setGenerateTaskId] = useState<string | null>(null)
    const [applyTaskId, setApplyTaskId] = useState<string | null>(null)
    const [confirmOpen, setConfirmOpen] = useState(false)
    const [downloadingDocumentId, setDownloadingDocumentId] = useState<string | null>(null)

    const [getUploadUrl] = useMutation(GET_UPLOAD_URL)
    const [createImport] = useMutation(CREATE_EXTERNAL_RECONCILIATION_IMPORT)
    const [applyChanges] = useMutation(APPLY_EXTERNAL_RECONCILIATION_CHANGES)

    const {data: generateTaskData} = useQuery(GET_TASK_BY_ID, {
        variables: {task_id: generateTaskId},
        skip: !generateTaskId || step !== "processing",
        pollInterval: step === "processing" ? POLL_INTERVAL_MS : 0,
    })
    const {data: applyTaskData} = useQuery(GET_TASK_BY_ID, {
        variables: {task_id: applyTaskId},
        skip: !applyTaskId || step !== "applying",
        pollInterval: step === "applying" ? POLL_INTERVAL_MS : 0,
    })

    const {data: diffDocumentData} = useQuery(FETCH_DOCUMENT, {
        variables: {electionEventId, documentId: diffDocumentId},
        skip: !diffDocumentId || !!envelope,
    })

    // Reads the diff-envelope document's content once its presigned URL is
    // available - a plain client-side fetch+parse, the read-back symmetric
    // counterpart of the raw PUT upload in handleFiles below, since a
    // Document is meant to be fetched by URL, not queried inline.
    useEffect(() => {
        const url = diffDocumentData?.fetchDocument?.url
        if (!url || envelope) {
            return
        }
        fetch(url)
            .then((response) => response.json())
            .then((data: ReconciliationDiffEnvelope) => {
                setEnvelope(data)
                setStep("review")
            })
            .catch(() =>
                setErrorMessage("Failed to load the reconciliation diff - please try again.")
            )
    }, [diffDocumentData?.fetchDocument?.url, envelope])

    const items: RawDiffItem[] = envelope?.items ?? []

    const datafixRows = useMemo(() => toRows(items, "datafix"), [items])
    const sequentRows = useMemo(() => toRows(items, "sequent"), [items])
    const rowFailures = useMemo(() => toRowFailures(items), [items])
    const isClean = datafixRows.length === 0
    const summary = useMemo(
        () => countsByCategory([...datafixRows, ...sequentRows]),
        [datafixRows, sequentRows]
    )
    const confirmationSummary = useMemo(() => summarizeForConfirmation(sequentRows), [sequentRows])

    // Drives the wizard's own step transitions - separate from the ambient
    // addWidget/setWidgetTaskId call below (which only feeds the app-wide
    // floating task widget, shown everywhere else in the app; it doesn't
    // know how to advance this dialog's own steps).
    const generateStatus = generateTaskData?.sequent_backend_tasks_execution?.[0]?.execution_status
    const generateDocumentId =
        generateTaskData?.sequent_backend_tasks_execution?.[0]?.annotations?.document_id
    if (
        step === "processing" &&
        generateStatus === ETaskExecutionStatus.SUCCESS &&
        generateDocumentId
    ) {
        setDiffDocumentId(generateDocumentId)
        // Stays on "processing" until the effect above finishes fetching and
        // parsing the envelope document, then moves to "review" itself.
    } else if (step === "processing" && generateStatus === ETaskExecutionStatus.FAILED) {
        setErrorMessage(
            "Failed to calculate the reconciliation diff - see the task widget for details."
        )
        setStep("drop")
    }

    const applyStatus = applyTaskData?.sequent_backend_tasks_execution?.[0]?.execution_status
    const rowFailuresDocumentId =
        applyTaskData?.sequent_backend_tasks_execution?.[0]?.annotations?.document_id
    if (step === "applying" && applyStatus === ETaskExecutionStatus.SUCCESS) {
        setStep("done")
    } else if (step === "applying" && applyStatus === ETaskExecutionStatus.FAILED) {
        setErrorMessage(
            "Failed to apply the Sequent-side changes - see the task widget for details."
        )
        setStep("done")
    }

    const reset = () => {
        setStep("drop")
        setFileName(null)
        setErrorMessage(null)
        setDiffDocumentId(null)
        setEnvelope(null)
        setGenerateTaskId(null)
        setApplyTaskId(null)
    }

    const handleFiles = async (files: FileList) => {
        const file = files[0]
        if (!file) {
            return
        }
        setFileName(file.name)
        setErrorMessage(null)
        setStep("processing")

        try {
            const {data: uploadData} = await getUploadUrl({
                variables: {
                    name: file.name,
                    media_type: "text/csv",
                    size: file.size,
                    is_public: false,
                    election_event_id: electionEventId,
                },
            })
            const upload = uploadData?.get_upload_url
            if (!upload?.url || !upload?.document_id) {
                throw new Error("Failed to get an upload URL")
            }
            await fetch(upload.url, {
                method: "PUT",
                headers: {"Content-Type": "text/csv"},
                body: file,
            })

            const {data: createData} = await createImport({
                variables: {
                    election_event_id: electionEventId,
                    document_id: upload.document_id,
                },
            })
            const created = createData?.create_external_reconciliation_import
            if (!created?.task_execution?.id) {
                throw new Error("Failed to start the reconciliation diff task")
            }

            setGenerateTaskId(created.task_execution.id)
            const widget = addWidget(ETasksExecution.GENERATE_RECONCILIATION_PATCHES, false)
            setWidgetTaskId(widget.identifier, created.task_execution.id)
        } catch (error) {
            setErrorMessage(
                error instanceof Error ? error.message : "Failed to upload the reconciliation file"
            )
            setStep("drop")
        }
    }

    const handleApply = async () => {
        if (!diffDocumentId) {
            return
        }
        setConfirmOpen(false)

        // Nothing to apply (a plain convergence re-check, D8/D9 - see the
        // module doc above): go straight to "done" without running the apply
        // task at all, rather than kicking off a no-op Celery task.
        if (sequentRows.length === 0) {
            setStep("done")
            return
        }

        setStep("applying")
        try {
            const {data} = await applyChanges({
                variables: {election_event_id: electionEventId, diff_document_id: diffDocumentId},
            })
            const taskId = data?.apply_external_reconciliation_changes?.task_execution?.id
            if (!taskId) {
                throw new Error("Failed to start the apply task")
            }
            setApplyTaskId(taskId)
            const widget = addWidget(ETasksExecution.APPLY_RECONCILIATION_PATCH, false)
            setWidgetTaskId(widget.identifier, taskId)
        } catch (error) {
            setErrorMessage(
                error instanceof Error
                    ? error.message
                    : "Failed to apply the reconciliation changes"
            )
            setStep("done")
        }
    }

    return (
        <Dialog open={open} onClose={onClose} fullWidth maxWidth="lg" scroll="paper">
            <DialogTitle>
                <Stack direction="row" justifyContent="space-between" alignItems="center">
                    <Typography variant="h6">Datafix reconciliation sync</Typography>
                    <IconButton onClick={onClose} size="small" aria-label="Close">
                        <CloseIcon fontSize="small" />
                    </IconButton>
                </Stack>
            </DialogTitle>
            <DialogContent dividers>
                <Stack spacing={3}>
                    {(step === "drop" || step === "processing") && (
                        <>
                            <Typography color="text.secondary">
                                Drop the reconciliation file Datafix produced - both diffs
                                (Datafix-side and Sequent-side) are calculated automatically and
                                shown in separate tables.
                            </Typography>
                            {errorMessage && <Alert severity="error">{errorMessage}</Alert>}
                            {step === "drop" ? (
                                <DropFile
                                    handleFiles={handleFiles}
                                    accept=".csv"
                                    formatLabel="CSV file"
                                />
                            ) : (
                                <Stack direction="row" spacing={2} alignItems="center">
                                    <CircularProgress size={20} />
                                    <Typography>
                                        Uploading {fileName} and calculating both diffs...
                                    </Typography>
                                </Stack>
                            )}
                        </>
                    )}

                    {step === "review" && envelope && (
                        <>
                            <Box
                                sx={{
                                    border: "1px solid",
                                    borderColor: "divider",
                                    borderRadius: 1,
                                    p: 1.5,
                                }}
                            >
                                <Typography variant="body2" color="text.secondary">
                                    {fileName} - Sequence {envelope.sequence}, generated{" "}
                                    {formatGeneratedAt(envelope.generated_at)}
                                </Typography>
                            </Box>

                            {rowFailures.length > 0 && (
                                <Alert severity="warning">
                                    {rowFailures.length} row(s) have an unexpected CountyMun (or a
                                    voted-via-other-channel guard) and are excluded from both diffs
                                    - reported as row failures once applied.
                                </Alert>
                            )}

                            {datafixRows.length === 0 && sequentRows.length === 0 ? (
                                <Alert severity="success">
                                    No differences - the two systems are already in sync.
                                </Alert>
                            ) : (
                                <>
                                    <CategorySummary counts={summary} />

                                    <Stack spacing={1}>
                                        <Stack
                                            direction="row"
                                            justifyContent="space-between"
                                            alignItems="center"
                                        >
                                            <Typography variant="subtitle1">
                                                Datafix diff
                                                {datafixRows.length > 0 &&
                                                    ` (${datafixRows.length})`}
                                            </Typography>
                                            {datafixRows.length > 0 &&
                                                envelope.datafix_patch_document_id && (
                                                    <Button
                                                        size="small"
                                                        variant="outlined"
                                                        onClick={() =>
                                                            setDownloadingDocumentId(
                                                                envelope.datafix_patch_document_id
                                                            )
                                                        }
                                                    >
                                                        Download Datafix patch
                                                    </Button>
                                                )}
                                        </Stack>
                                        <SyncDiffTable
                                            rows={datafixRows}
                                            emptyMessage="No Datafix-side differences."
                                        />
                                    </Stack>

                                    <Divider />

                                    <Stack spacing={1}>
                                        <Typography variant="subtitle1">
                                            Sequent diff
                                            {sequentRows.length > 0 && ` (${sequentRows.length})`}
                                        </Typography>
                                        <Typography variant="caption" color="text.secondary">
                                            Applied directly to Sequent - no patch file is generated
                                            for these.
                                        </Typography>
                                        <SyncDiffTable
                                            rows={sequentRows}
                                            emptyMessage="No Sequent-side differences."
                                        />
                                    </Stack>
                                </>
                            )}
                        </>
                    )}

                    {(step === "applying" || step === "done") && (
                        <Stack spacing={2}>
                            {errorMessage && <Alert severity="error">{errorMessage}</Alert>}
                            {step === "applying" && (
                                <Stack direction="row" spacing={2} alignItems="center">
                                    <CircularProgress size={20} />
                                    <Typography>Applying Sequent-side changes...</Typography>
                                </Stack>
                            )}
                            {step === "done" && !errorMessage && (
                                <>
                                    {rowFailures.length > 0 ? (
                                        <Alert severity="warning">
                                            {rowFailures.length} row(s) failed to apply - fix
                                            manually; the next reconciliation file will point out
                                            the same diff otherwise.
                                        </Alert>
                                    ) : (
                                        <Alert severity="success">
                                            All Sequent-side changes applied successfully.
                                        </Alert>
                                    )}
                                    {rowFailuresDocumentId && (
                                        <Button
                                            sx={{alignSelf: "flex-start"}}
                                            onClick={() =>
                                                setDownloadingDocumentId(rowFailuresDocumentId)
                                            }
                                        >
                                            Download row failures report
                                        </Button>
                                    )}
                                </>
                            )}
                        </Stack>
                    )}

                    {downloadingDocumentId && (
                        <DownloadDocument
                            documentId={downloadingDocumentId}
                            electionEventId={electionEventId}
                            fileName={`datafix-reconciliation-${downloadingDocumentId}.csv`}
                            onDownload={() => setDownloadingDocumentId(null)}
                        />
                    )}
                </Stack>
            </DialogContent>

            <DialogActions sx={{justifyContent: "space-between", px: 3, py: 2}}>
                {(step === "drop" || step === "processing") && (
                    <Button onClick={onClose} disabled={step === "processing"}>
                        Cancel
                    </Button>
                )}
                {step === "review" && (
                    <>
                        <Button onClick={reset}>Back</Button>
                        <Button
                            variant="contained"
                            disabled={!isClean}
                            onClick={() =>
                                sequentRows.length > 0 ? setConfirmOpen(true) : handleApply()
                            }
                        >
                            {sequentRows.length > 0 ? "Apply" : "Next"}
                        </Button>
                    </>
                )}
                {step === "applying" && <Button disabled>Back</Button>}
                {step === "done" && (
                    <>
                        <Button onClick={reset}>Start over</Button>
                        <Button variant="contained" onClick={onClose}>
                            Close
                        </Button>
                    </>
                )}
            </DialogActions>

            <Dialog open={confirmOpen} onClose={() => setConfirmOpen(false)}>
                <DialogTitle>Confirm reconciliation changes</DialogTitle>
                <DialogContent>
                    <Stack spacing={2} sx={{pt: 1}}>
                        <Typography>{confirmationSummary}</Typography>
                        <CategorySummary
                            counts={countsByCategory(sequentRows)}
                            highlighted={HIGHLIGHTED_CATEGORIES}
                        />
                        <Typography variant="caption" color="text.secondary">
                            Categories outlined in orange (
                            {[ESyncChangeCategory.VOTED_OTHER_CHANNEL, ESyncChangeCategory.DISABLED]
                                .map((category) => CATEGORY_LABELS[category])
                                .join(", ")}
                            ) touch voted status or disable voters.
                        </Typography>
                    </Stack>
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setConfirmOpen(false)}>Cancel</Button>
                    <Button variant="contained" onClick={handleApply}>
                        {sequentRows.length > 0 ? "Apply changes" : "Continue"}
                    </Button>
                </DialogActions>
            </Dialog>
        </Dialog>
    )
}

export default ReconciliationWizard
