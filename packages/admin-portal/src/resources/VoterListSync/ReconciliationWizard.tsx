// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useState} from "react"
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
    Drawer,
    Stack,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ATTR_RESET_VALUE} from "@/types/keycloak"
import {DropFile} from "@sequentech/ui-essentials"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {SyncDiffTable} from "./SyncDiffTable"
import {CategorySummary} from "./CategorySummary"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {ESyncChangeCategory, SyncDiffRow} from "./types"
import {HIGHLIGHTED_CATEGORIES} from "./constants"
import {formatGeneratedAt} from "./utils"
import ElectionHeader from "@/components/ElectionHeader"
import {DrawerStyles} from "@/components/styles/DrawerStyles"
import {ImportStyles} from "@/components/election-event/import-data/ImportScreen"
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

type TranslateFn = ReturnType<typeof useTranslation>["t"]

const POLL_INTERVAL_MS = 3000

/**
 * A field's own `(old, new)` pair now lives inside the field itself, wire-
 * shaped as a single-key object keyed by the Rust variant name (e.g.
 * `{"Channel": ["NONE", "INTERNET"]}`, `{"Enabled": [true, false]}`,
 * `{"KeycloakUA": [{"voted-channel": "NONE"}, {"voted-channel": "PAPER"}]}`)
 * — not a separate `old_value`/`new_value` on the item, since a field is
 * never meaningful apart from its own old/new values. `null` when the change
 * (e.g. a CountyMun mismatch row failure) has no corresponding field at all.
 * See `describeFields` below for how this is turned back into flat
 * label/oldValue/newValue rows for display.
 */
type RawFieldValue = Record<string, unknown> | null

interface RawDiffItem {
    voter_username: string
    target: "datafix" | "sequent"
    category: ESyncChangeCategory
    field?: RawFieldValue
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
    /** False for an equal-Sequence convergence check after a successful apply. */
    apply_allowed: boolean
    external_patch_document_id: string | null
    items: RawDiffItem[]
}

interface ApplyTaskRowFailure {
    voter_id: string
    reason: string
}

interface ApplyTaskRowFailureSummary {
    rows: SyncDiffRow[]
    totalCount: number
    truncated: boolean
}

/**
 * Counts distinct voters per category, not rows: a single voter's change can
 * fan out into several `SyncDiffRow`s (e.g. a new voter is 4 rows - area,
 * DoB, channel, enabled - all tagged `VOTER_ADDED`), so counting rows would
 * multiply-count that one voter under every category with more than one
 * field per voter.
 */
const countsByCategory = (rows: SyncDiffRow[]): Partial<Record<ESyncChangeCategory, number>> => {
    const votersByCategory = new Map<ESyncChangeCategory, Set<string>>()
    rows.forEach((row) => {
        const voters = votersByCategory.get(row.category) ?? new Set<string>()
        voters.add(row.voterId)
        votersByCategory.set(row.category, voters)
    })
    const counts: Partial<Record<ESyncChangeCategory, number>> = {}
    votersByCategory.forEach((voters, category) => {
        counts[category] = voters.size
    })
    return counts
}

const summarizeForConfirmation = (rows: SyncDiffRow[], t: TranslateFn): string => {
    const counts = countsByCategory(rows)

    const phrases = [
        [
            ESyncChangeCategory.VOTED_OTHER_CHANNEL,
            "reconciliation.wizard.summary.votedOtherChannel",
        ],
        [ESyncChangeCategory.DISABLED_DELETE_CALL, "reconciliation.wizard.summary.disabled"],
        [ESyncChangeCategory.REENABLED, "reconciliation.wizard.summary.reenabled"],
        [ESyncChangeCategory.VOTED_UNMARKED, "reconciliation.wizard.summary.votedUnmarked"],
        [ESyncChangeCategory.PROFILE_UPDATE, "reconciliation.wizard.summary.profileUpdated"],
        [ESyncChangeCategory.VOTER_ADDED, "reconciliation.wizard.summary.voterAdded"],
    ] as const

    const parts = phrases
        .map(([category, key]) => {
            const count = counts[category] ?? 0
            return count > 0 ? t(key, {count}) : null
        })
        .filter((part): part is string => part !== null)

    return parts.length > 0
        ? t("reconciliation.wizard.summary.prefix", {parts: parts.join(", ")})
        : t("reconciliation.wizard.summary.empty")
}

/** Flat label/oldValue/newValue the table actually renders, derived from a
 * raw item's `field` (see the `RawFieldValue` doc above). */
interface FieldDisplay {
    label: string
    oldValue: string
    newValue: string
}

/** Renders a single old/new element: booleans as "true"/"false" (`Enabled`),
 * otherwise the value as-is (every other field, including each individual
 * `KeycloakUA` attribute, is already a plain string). */
const formatFieldValue = (value: unknown): string => {
    if (typeof value === "boolean") {
        return value ? "true" : "false"
    }
    return value == null ? ATTR_RESET_VALUE : String(value)
}

/**
 * `KeycloakUA`'s `(old, new)` pair is a bag of Keycloak attributes, not a
 * single field - e.g. "voted via other channel" writes `voted-channel` and
 * `disable-comment` together in one atomic Keycloak edit (see
 * `reconciliation::diff` in windmill), so `new` can carry a key `old` never
 * had. Splitting into one row per attribute key (the union of both bags'
 * keys, each looked up independently) is what keeps that case readable:
 * joining the bags into a single row instead mismatches keys against values
 * whenever the two sides don't share the exact same key set.
 */
const describeFields = (field: RawFieldValue, t: TranslateFn): FieldDisplay[] => {
    const entry = field ? Object.entries(field)[0] : undefined
    if (!entry) {
        // `null` when the change (e.g. a CountyMun mismatch row failure)
        // has no corresponding field at all - the Reason column explains it.
        return [
            {
                label: t("reconciliation.table.rowLabel"),
                oldValue: ATTR_RESET_VALUE,
                newValue: ATTR_RESET_VALUE,
            },
        ]
    }
    const [variantName, tuple] = entry as [string, [unknown, unknown]]
    const [oldRaw, newRaw] = tuple

    if (variantName === "KeycloakUA") {
        const oldBag = (oldRaw ?? {}) as Record<string, string>
        const newBag = (newRaw ?? {}) as Record<string, string>
        const keys = Array.from(new Set([...Object.keys(oldBag), ...Object.keys(newBag)]))
        return keys.map((key) => ({
            label: t(`usersAndRolesScreen.users.fields.${key}`, key),
            oldValue: formatFieldValue(oldBag[key] ?? null),
            newValue: formatFieldValue(newBag[key] ?? null),
        }))
    }

    return [
        {
            label: variantName,
            oldValue: formatFieldValue(oldRaw),
            newValue: formatFieldValue(newRaw),
        },
    ]
}

const toRows = (
    items: RawDiffItem[],
    target: "datafix" | "sequent",
    t: TranslateFn
): SyncDiffRow[] =>
    items
        .filter(
            (item) => item.target === target && item.category !== ESyncChangeCategory.ROW_FAILURE
        )
        .flatMap((item, index) =>
            describeFields(item.field ?? null, t).map(
                ({label, oldValue, newValue}, fieldIndex) => ({
                    id: `${item.voter_username}:${label}:${index}:${fieldIndex}`,
                    voterId: item.voter_username,
                    field: label,
                    label,
                    oldValue,
                    newValue,
                    category: item.category,
                    target: item.target,
                })
            )
        )

const toRowFailures = (items: RawDiffItem[], t: TranslateFn): SyncDiffRow[] =>
    items
        .filter((item) => item.category === ESyncChangeCategory.ROW_FAILURE)
        .flatMap((item, index) =>
            describeFields(item.field ?? null, t).map(
                ({label, oldValue, newValue}, fieldIndex) => ({
                    id: `failure:${item.voter_username}:${index}:${fieldIndex}`,
                    voterId: item.voter_username,
                    field: label,
                    label,
                    oldValue,
                    newValue,
                    category: item.category,
                    target: item.target,
                    failureReason: item.failure_reason ?? undefined,
                })
            )
        )

/** Converts the backend's version-stable structured task result into the same
 * rows used for generate-time failures. Human task logs remain presentation
 * only and can be reworded without breaking this contract. */
const taskAnnotationsToRowFailures = (
    annotations: unknown,
    t: TranslateFn
): ApplyTaskRowFailureSummary => {
    if (!annotations || typeof annotations !== "object") {
        return {rows: [], totalCount: 0, truncated: false}
    }
    const taskResult = annotations as Record<string, unknown>
    const failures = taskResult.reconciliation_row_failures
    if (!Array.isArray(failures)) {
        return {rows: [], totalCount: 0, truncated: false}
    }

    const rows = failures.flatMap((failure, index) => {
        if (
            !failure ||
            typeof failure !== "object" ||
            typeof (failure as Partial<ApplyTaskRowFailure>).voter_id !== "string" ||
            typeof (failure as Partial<ApplyTaskRowFailure>).reason !== "string"
        ) {
            return []
        }
        const {voter_id: voterId, reason: failureReason} = failure as ApplyTaskRowFailure
        return [
            {
                id: `apply-failure:${voterId}:${index}`,
                voterId,
                field: t("reconciliation.table.rowLabel"),
                label: t("reconciliation.table.rowLabel"),
                oldValue: ATTR_RESET_VALUE,
                newValue: ATTR_RESET_VALUE,
                category: ESyncChangeCategory.ROW_FAILURE,
                target: "sequent" as const,
                failureReason,
            },
        ]
    })
    const annotatedTotal = taskResult.reconciliation_row_failure_count
    const totalCount =
        typeof annotatedTotal === "number" &&
        Number.isSafeInteger(annotatedTotal) &&
        annotatedTotal >= rows.length
            ? annotatedTotal
            : rows.length
    const truncated =
        taskResult.reconciliation_row_failures_truncated === true || totalCount > rows.length
    return {rows, totalCount, truncated}
}

/**
 * Reconciliation wizard: Drop the reconciliation file, both diffs (external-side, Sequent-side) are
 * calculated at once and shown in separate tables. The external patch is
 * downloadable as soon as its diff is non-empty; Apply is disabled until that
 * diff is empty (clean import), then applies the Sequent-side diff directly -
 * there is no Sequent patch file to download (the Sequent-side items exist
 * only as an internal document apply_reconciliation_patch reads from).
 * Re-importing an already successfully applied Sequence is an explicit
 * diff-only convergence check: its envelope has `apply_allowed = false` and
 * this wizard never calls the apply route for it.
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
    const {t} = useTranslation()
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
        skip: !applyTaskId,
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
                setErrorMessage(t("reconciliation.wizard.notifications.envelopeLoadError"))
            )
    }, [diffDocumentData?.fetchDocument?.url, envelope, t])

    const items: RawDiffItem[] = envelope?.items ?? []

    const datafixRows = useMemo(() => toRows(items, "datafix", t), [items, t])
    const sequentRows = useMemo(() => toRows(items, "sequent", t), [items, t])
    const rowFailures = useMemo(() => toRowFailures(items, t), [items, t])
    const applyTaskAnnotations = applyTaskData?.sequent_backend_tasks_execution?.[0]?.annotations
    const applyRowFailureSummary = useMemo(
        () => taskAnnotationsToRowFailures(applyTaskAnnotations, t),
        [applyTaskAnnotations, t]
    )
    const applyRowFailures = applyRowFailureSummary.rows
    const finalRowFailures = useMemo(
        () =>
            Array.from(
                new Map(
                    [...rowFailures, ...applyRowFailures].map((row) => [
                        `${row.voterId}:${row.failureReason ?? ""}`,
                        row,
                    ])
                ).values()
            ),
        [rowFailures, applyRowFailures]
    )
    const finalRowFailureCount = Math.max(
        finalRowFailures.length,
        applyRowFailureSummary.totalCount
    )
    const finalRowFailuresTruncated =
        applyRowFailureSummary.truncated && finalRowFailureCount > finalRowFailures.length
    const isClean = datafixRows.length === 0
    const summary = useMemo(
        () => countsByCategory([...datafixRows, ...sequentRows]),
        [datafixRows, sequentRows]
    )
    const confirmationSummary = useMemo(
        () => summarizeForConfirmation(sequentRows, t),
        [sequentRows, t]
    )

    // Drives the wizard's own step transitions - separate from the ambient
    // addWidget/setWidgetTaskId call below (which only feeds the app-wide
    // floating task widget, shown everywhere else in the app; it doesn't
    // know how to advance this dialog's own steps).
    const generateStatus = generateTaskData?.sequent_backend_tasks_execution?.[0]?.execution_status
    const generateDocumentId =
        generateTaskData?.sequent_backend_tasks_execution?.[0]?.annotations?.document_id

    // Runs as an effect (not inline during render) so it fires once per
    // status/document transition instead of on every render - calling the
    // setters unconditionally during render never stops re-triggering itself
    // since `step` only changes once the envelope effect above finishes,
    // which React reports as "Too many re-renders".
    useEffect(() => {
        if (step !== "processing") {
            return
        }
        if (generateStatus === ETaskExecutionStatus.SUCCESS && generateDocumentId) {
            setDiffDocumentId(generateDocumentId)
            // Stays on "processing" until the effect above finishes fetching
            // and parsing the envelope document, then moves to "review"
            // itself.
        } else if (generateStatus === ETaskExecutionStatus.FAILED) {
            setErrorMessage(t("reconciliation.wizard.notifications.generateFailed"))
            setStep("drop")
        }
    }, [step, generateStatus, generateDocumentId, t])

    const applyStatus = applyTaskData?.sequent_backend_tasks_execution?.[0]?.execution_status

    useEffect(() => {
        if (step !== "applying") {
            return
        }
        if (applyStatus === ETaskExecutionStatus.SUCCESS) {
            setStep("done")
        } else if (applyStatus === ETaskExecutionStatus.FAILED) {
            setErrorMessage(t("reconciliation.wizard.notifications.applyFailed"))
            setStep("done")
        }
    }, [step, applyStatus, t])

    const reset = useCallback(() => {
        setStep("drop")
        setFileName(null)
        setErrorMessage(null)
        setDiffDocumentId(null)
        setEnvelope(null)
        setGenerateTaskId(null)
        setApplyTaskId(null)
        setConfirmOpen(false)
        setDownloadingDocumentId(null)
    }, [])

    // The wizard stays mounted across open/close (the parent only flips
    // `open`), so without this a closed-then-reopened wizard would still
    // show whatever step it was left on - the "done" screen from a prior
    // round, or a stale diff from a round closed before applying.
    useEffect(() => {
        if (open) {
            reset()
        }
    }, [open, reset])

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
                throw new Error(t("reconciliation.wizard.notifications.uploadUrlError"))
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
                throw new Error(t("reconciliation.wizard.notifications.generateTaskError"))
            }

            setGenerateTaskId(created.task_execution.id)
            const widget = addWidget(ETasksExecution.GENERATE_RECONCILIATION_PATCHES, false)
            setWidgetTaskId(widget.identifier, created.task_execution.id)
        } catch (error) {
            setErrorMessage(
                error instanceof Error
                    ? error.message
                    : t("reconciliation.wizard.notifications.uploadError")
            )
            setStep("drop")
        }
    }

    const handleApply = async () => {
        if (!diffDocumentId) {
            return
        }
        setConfirmOpen(false)

        // A successful equal-Sequence retry is a diff-only convergence check
        // and must not enqueue apply. A new round with no voter changes still
        // does enqueue: that records its Sequence and run-level audit log.
        if (!envelope?.apply_allowed) {
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
                throw new Error(t("reconciliation.wizard.notifications.applyTaskError"))
            }
            setApplyTaskId(taskId)
            const widget = addWidget(ETasksExecution.APPLY_RECONCILIATION_PATCH, false)
            setWidgetTaskId(widget.identifier, taskId)
        } catch (error) {
            setErrorMessage(
                error instanceof Error
                    ? error.message
                    : t("reconciliation.wizard.notifications.applyError")
            )
            setStep("done")
        }
    }

    return (
        <>
            <Drawer
                anchor="right"
                open={open}
                onClose={onClose}
                PaperProps={{
                    sx: {
                        width: {xs: "100vw", sm: "90vw", lg: "80vw"},
                        maxWidth: "1440px",
                    },
                }}
            >
                <Box sx={{padding: "16px"}}>
                    <ElectionHeader
                        title="reconciliation.wizard.title"
                        subtitle="reconciliation.wizard.subtitle"
                    />
                    <DrawerStyles.Wrapper>
                        <Stack spacing={3}>
                            {(step === "drop" || step === "processing") && (
                                <>
                                    <Typography color="text.secondary">
                                        {t("reconciliation.wizard.drop.description")}
                                    </Typography>
                                    {errorMessage && <Alert severity="error">{errorMessage}</Alert>}
                                    {step === "drop" ? (
                                        <DropFile
                                            handleFiles={handleFiles}
                                            accept=".csv"
                                            formatLabel={t(
                                                "reconciliation.wizard.drop.fileFormatLabel"
                                            )}
                                        />
                                    ) : (
                                        <Stack direction="row" spacing={2} alignItems="center">
                                            <CircularProgress size={20} />
                                            <Typography>
                                                {t("reconciliation.wizard.drop.uploading", {
                                                    fileName,
                                                })}
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
                                            {t("reconciliation.wizard.review.fileSummary", {
                                                fileName,
                                                sequence: envelope.sequence,
                                                generatedAt: formatGeneratedAt(
                                                    envelope.generated_at
                                                ),
                                            })}
                                        </Typography>
                                    </Box>

                                    {!envelope.apply_allowed &&
                                        (datafixRows.length > 0 || sequentRows.length > 0) && (
                                            <Alert severity="warning">
                                                {t(
                                                    "reconciliation.wizard.review.diffOnlyDifferences"
                                                )}
                                            </Alert>
                                        )}

                                    {rowFailures.length > 0 && (
                                        <Stack spacing={1}>
                                            <Alert severity="warning">
                                                {t(
                                                    "reconciliation.wizard.review.rowFailuresWarning",
                                                    {count: rowFailures.length}
                                                )}
                                            </Alert>
                                            <SyncDiffTable rows={rowFailures} />
                                        </Stack>
                                    )}

                                    {datafixRows.length === 0 && sequentRows.length === 0 ? (
                                        <Alert severity="success">
                                            {t("reconciliation.wizard.review.noDifferences")}
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
                                                        {t(
                                                            "reconciliation.wizard.review.externalDiffTitle"
                                                        )}
                                                        {datafixRows.length > 0 &&
                                                            ` (${datafixRows.length})`}
                                                    </Typography>
                                                    {datafixRows.length > 0 &&
                                                        envelope.external_patch_document_id && (
                                                            <Button
                                                                size="small"
                                                                variant="outlined"
                                                                onClick={() =>
                                                                    setDownloadingDocumentId(
                                                                        envelope.external_patch_document_id
                                                                    )
                                                                }
                                                            >
                                                                {t(
                                                                    "reconciliation.wizard.review.downloadExternalPatch"
                                                                )}
                                                            </Button>
                                                        )}
                                                </Stack>
                                                {datafixRows.length > 0 && (
                                                    <Typography
                                                        variant="caption"
                                                        color="text.secondary"
                                                    >
                                                        {t(
                                                            "reconciliation.wizard.review.externalDiffCaption"
                                                        )}
                                                    </Typography>
                                                )}
                                                <SyncDiffTable
                                                    rows={datafixRows}
                                                    emptyMessage={t(
                                                        "reconciliation.wizard.review.noExternalDifferences"
                                                    )}
                                                />
                                            </Stack>

                                            <Divider />

                                            <Stack spacing={1}>
                                                <Typography variant="subtitle1">
                                                    {t(
                                                        "reconciliation.wizard.review.sequentDiffTitle"
                                                    )}
                                                    {sequentRows.length > 0 &&
                                                        ` (${sequentRows.length})`}
                                                </Typography>
                                                <Typography
                                                    variant="caption"
                                                    color="text.secondary"
                                                >
                                                    {t(
                                                        "reconciliation.wizard.review.sequentDiffCaption"
                                                    )}
                                                </Typography>
                                                <SyncDiffTable
                                                    rows={sequentRows}
                                                    emptyMessage={t(
                                                        "reconciliation.wizard.review.noSequentDifferences"
                                                    )}
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
                                            <Typography>
                                                {t("reconciliation.wizard.applying.inProgress")}
                                            </Typography>
                                        </Stack>
                                    )}
                                    {step === "done" && (
                                        <>
                                            {finalRowFailureCount > 0 ? (
                                                <Stack spacing={1}>
                                                    <Alert severity="warning">
                                                        {t(
                                                            "reconciliation.wizard.applying.rowFailures",
                                                            {count: finalRowFailureCount}
                                                        )}
                                                    </Alert>
                                                    {finalRowFailuresTruncated && (
                                                        <Typography
                                                            variant="caption"
                                                            color="text.secondary"
                                                        >
                                                            {t(
                                                                "reconciliation.wizard.applying.rowFailuresTruncated",
                                                                {
                                                                    shown: finalRowFailures.length,
                                                                    count: finalRowFailureCount,
                                                                }
                                                            )}
                                                        </Typography>
                                                    )}
                                                    <SyncDiffTable rows={finalRowFailures} />
                                                </Stack>
                                            ) : !errorMessage ? (
                                                <Alert severity="success">
                                                    {t("reconciliation.wizard.applying.success")}
                                                </Alert>
                                            ) : null}
                                        </>
                                    )}
                                </Stack>
                            )}

                            {downloadingDocumentId && (
                                <DownloadDocument
                                    documentId={downloadingDocumentId}
                                    electionEventId={electionEventId}
                                    fileName={`external-reconciliation-${downloadingDocumentId}.csv`}
                                    onDownload={() => setDownloadingDocumentId(null)}
                                />
                            )}
                        </Stack>

                        <Box
                            sx={{
                                display: "flex",
                                flexDirection: "row",
                                justifyContent: "space-between",
                                alignItems: "center",
                                marginTop: "16px",
                            }}
                        >
                            {(step === "drop" || step === "processing") && (
                                <ImportStyles.CancelButton
                                    onClick={onClose}
                                    disabled={step === "processing"}
                                >
                                    {t("reconciliation.wizard.actions.cancel")}
                                </ImportStyles.CancelButton>
                            )}
                            {step === "review" && (
                                <>
                                    <ImportStyles.CancelButton onClick={reset}>
                                        {t("reconciliation.wizard.actions.back")}
                                    </ImportStyles.CancelButton>
                                    <ImportStyles.ImportButton
                                        disabled={
                                            !isClean ||
                                            !envelope ||
                                            (!envelope.apply_allowed && sequentRows.length > 0)
                                        }
                                        onClick={() =>
                                            sequentRows.length > 0
                                                ? setConfirmOpen(true)
                                                : handleApply()
                                        }
                                    >
                                        {sequentRows.length > 0
                                            ? t("reconciliation.wizard.actions.apply")
                                            : t("reconciliation.wizard.actions.next")}
                                    </ImportStyles.ImportButton>
                                </>
                            )}
                            {step === "applying" && (
                                <ImportStyles.CancelButton disabled>
                                    {t("reconciliation.wizard.actions.back")}
                                </ImportStyles.CancelButton>
                            )}
                            {step === "done" && (
                                <>
                                    <ImportStyles.CancelButton onClick={reset}>
                                        {t("reconciliation.wizard.actions.startOver")}
                                    </ImportStyles.CancelButton>
                                    <ImportStyles.ImportButton onClick={onClose}>
                                        {t("reconciliation.wizard.actions.close")}
                                    </ImportStyles.ImportButton>
                                </>
                            )}
                        </Box>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>

            <Dialog open={confirmOpen} onClose={() => setConfirmOpen(false)}>
                <DialogTitle>{t("reconciliation.wizard.confirm.title")}</DialogTitle>
                <DialogContent>
                    <Stack spacing={2} sx={{pt: 1}}>
                        <Typography>{confirmationSummary}</Typography>
                        <CategorySummary
                            counts={countsByCategory(sequentRows)}
                            highlighted={HIGHLIGHTED_CATEGORIES}
                        />
                        <Typography variant="caption" color="text.secondary">
                            {t("reconciliation.wizard.confirm.categoriesNote", {
                                categories: [
                                    ESyncChangeCategory.VOTED_OTHER_CHANNEL,
                                    ESyncChangeCategory.DISABLED_DELETE_CALL,
                                ]
                                    .map((category) =>
                                        t(`reconciliation.categories.${category}`, category)
                                    )
                                    .join(", "),
                            })}
                        </Typography>
                    </Stack>
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setConfirmOpen(false)}>
                        {t("reconciliation.wizard.actions.cancel")}
                    </Button>
                    <Button variant="contained" onClick={handleApply}>
                        {sequentRows.length > 0
                            ? t("reconciliation.wizard.confirm.applyChanges")
                            : t("reconciliation.wizard.confirm.continue")}
                    </Button>
                </DialogActions>
            </Dialog>
        </>
    )
}

export default ReconciliationWizard
