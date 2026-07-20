// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo, useState} from "react"
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
import {MockTaskProgress} from "./MockTaskProgress"
import {
    ESyncChangeCategory,
    SyncDiffRow,
    SyncFileMeta,
    SyncTaskResult,
    SyncVoterRecord,
} from "./types"
import {HIGHLIGHTED_CATEGORIES, CATEGORY_LABELS} from "./constants"
import {
    MOCK_APPLY_STEPS,
    buildDatafixPatchCsv,
    buildRowFailuresCsv,
    downloadTextFile,
    formatGeneratedAt,
    mockCalculateReconciliationDiff,
    mockUploadToS3,
    parseReconciliationFile,
    runMockTask,
    validateSequence,
} from "./mockSyncEngine"

type WizardStep = "drop" | "processing" | "review" | "applying" | "done"

interface ReconciliationWizardProps {
    open: boolean
    onClose: () => void
}

const summarizeForConfirmation = (rows: SyncDiffRow[]): string => {
    const counts = new Map<ESyncChangeCategory, number>()
    rows.forEach((row) => counts.set(row.category, (counts.get(row.category) ?? 0) + 1))

    const phrases = [
        [ESyncChangeCategory.VOTED_OTHER_CHANNEL, "marks", "voter(s) as voted via other channels"],
        [ESyncChangeCategory.DISABLED, "disables", "voter(s)"],
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

/**
 * Single reconciliation wizard from DatafixPossibleImplementation.md: drop
 * the reconciliation file, both diffs (Datafix-side, Sequent-side) are
 * calculated at once and shown in two separate tables. The Datafix patch is
 * downloadable as soon as its diff is non-empty; Apply is disabled until that
 * diff is empty (clean import), then applies the Sequent-side diff directly -
 * there is no Sequent patch file. Opened from a button on the Voters tab
 * (see ListUsers.tsx), gated by IPermissions.ELECTION_EVENT_VOTER_LIST_SYNC.
 * Backend interaction is mocked in mockSyncEngine.ts.
 */
export const ReconciliationWizard: React.FC<ReconciliationWizardProps> = ({open, onClose}) => {
    const [lastAppliedSequence, setLastAppliedSequence] = useState(-1)
    const [step, setStep] = useState<WizardStep>("drop")
    const [fileName, setFileName] = useState<string | null>(null)
    const [meta, setMeta] = useState<SyncFileMeta | null>(null)
    const [diffRows, setDiffRows] = useState<SyncDiffRow[]>([])
    const [records, setRecords] = useState<SyncVoterRecord[]>([])
    const [sequenceError, setSequenceError] = useState<string | null>(null)
    const [confirmOpen, setConfirmOpen] = useState(false)
    const [taskStatus, setTaskStatus] = useState<ETaskExecutionStatus>(ETaskExecutionStatus.STARTED)
    const [taskLogs, setTaskLogs] = useState<string[]>([])
    const [taskResult, setTaskResult] = useState<SyncTaskResult | null>(null)
    // MOCK: simulates "repeat from step 1" in the operator flow - each new
    // import within this session is treated as the next reconciliation file,
    // so round 0 needs a Datafix patch, round 1 comes back clean on the
    // Datafix side (Sequent-side diff only), round 2+ is fully converged. Not
    // reset by reset() - it tracks session progress across imports.
    const [round, setRound] = useState(0)

    const failureRows = useMemo(
        () => diffRows.filter((row) => row.category === ESyncChangeCategory.ROW_FAILURE),
        [diffRows]
    )
    const datafixRows = useMemo(
        () =>
            diffRows.filter(
                (row) =>
                    row.target === "datafix" && row.category !== ESyncChangeCategory.ROW_FAILURE
            ),
        [diffRows]
    )
    const sequentRows = useMemo(
        () =>
            diffRows.filter(
                (row) =>
                    row.target === "sequent" && row.category !== ESyncChangeCategory.ROW_FAILURE
            ),
        [diffRows]
    )
    const actionableRows = useMemo(
        () => [...datafixRows, ...sequentRows],
        [datafixRows, sequentRows]
    )
    const confirmationSummary = useMemo(() => summarizeForConfirmation(sequentRows), [sequentRows])
    const isClean = datafixRows.length === 0

    const reset = () => {
        setStep("drop")
        setFileName(null)
        setMeta(null)
        setDiffRows([])
        setRecords([])
        setSequenceError(null)
        setTaskStatus(ETaskExecutionStatus.STARTED)
        setTaskLogs([])
        setTaskResult(null)
    }

    const handleFiles = async (files: FileList) => {
        const file = files[0]
        if (!file) {
            return
        }
        setFileName(file.name)
        setSequenceError(null)
        setStep("processing")

        // MOCK: upload to S3 (see mockUploadToS3).
        await mockUploadToS3(file)

        const text = await file.text()
        const {meta: fileMeta, rows} = parseReconciliationFile(text)
        setMeta(fileMeta)

        const sequenceCheck = validateSequence(fileMeta, lastAppliedSequence)
        if (!sequenceCheck.valid) {
            setSequenceError(sequenceCheck.message ?? "Stale file.")
            setStep("drop")
            return
        }

        // MOCK: backend diff calculation, both sides at once (see
        // mockCalculateReconciliationDiff).
        const diff = await mockCalculateReconciliationDiff(rows, round)
        setRound((previous) => previous + 1)
        setDiffRows(diff.diffRows)
        setRecords(diff.records)
        setStep("review")
    }

    const handleDownloadDatafixPatch = () => {
        if (!meta) {
            return
        }
        // MOCK: the real patch document is generated and uploaded to S3 by the
        // backend as soon as the diff is calculated; here we just re-serialize
        // the diff we already have.
        downloadTextFile(
            `datafix_patch_seq${meta.sequence}.csv`,
            buildDatafixPatchCsv(meta, records)
        )
    }

    const handleApply = async () => {
        setConfirmOpen(false)
        setStep("applying")
        setTaskStatus(ETaskExecutionStatus.IN_PROGRESS)
        setTaskLogs([])
        const result = await runMockTask(
            MOCK_APPLY_STEPS,
            [...sequentRows, ...failureRows],
            (log) => setTaskLogs((previous) => [...previous, log])
        )
        setTaskResult(result)
        setTaskStatus(ETaskExecutionStatus.SUCCESS)
        setStep("done")
        if (meta) {
            setLastAppliedSequence(meta.sequence)
        }
    }

    const handleDownloadFailures = () => {
        if (!taskResult) {
            return
        }
        downloadTextFile(
            `apply_row_failures_seq${meta?.sequence ?? 0}.csv`,
            buildRowFailuresCsv(taskResult.rowFailures)
        )
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
                            {sequenceError && <Alert severity="error">{sequenceError}</Alert>}
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

                    {step === "review" && meta && (
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
                                    {fileName} - Sequence {meta.sequence}, generated{" "}
                                    {formatGeneratedAt(meta.generatedAt)}
                                </Typography>
                                <Typography variant="caption" color="text.secondary">
                                    {/* MOCK: reconciliation round, see mockCalculateReconciliationDiff */}
                                    Simulated reconciliation round {round - 1}
                                    {round - 1 === 0 && " (fresh import - both sides have changes)"}
                                    {round - 1 === 1 &&
                                        " (clean on the Datafix side - only Sequent-side changes remain)"}
                                    {round - 1 >= 2 && " (converged - both systems in sync)"}
                                </Typography>
                            </Box>

                            {failureRows.length > 0 && (
                                <Alert severity="warning">
                                    {failureRows.length} row(s) have an unexpected CountyMun and are
                                    excluded from both diffs - reported as row failures once
                                    applied.
                                </Alert>
                            )}

                            {diffRows.length === 0 ? (
                                <Alert severity="success">
                                    No differences - the two systems are already in sync.
                                </Alert>
                            ) : (
                                <>
                                    <CategorySummary rows={actionableRows} />

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
                                            {datafixRows.length > 0 && (
                                                <Button
                                                    size="small"
                                                    variant="outlined"
                                                    onClick={handleDownloadDatafixPatch}
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
                            <MockTaskProgress
                                title="Apply Sequent-side changes"
                                status={taskStatus}
                                logs={taskLogs}
                            />
                            {step === "done" && taskResult && (
                                <>
                                    {taskResult.rowFailures.length > 0 ? (
                                        <Alert severity="warning">
                                            {taskResult.rowFailures.length} row(s) failed to apply -
                                            fix manually; the next reconciliation file will point
                                            out the same diff otherwise.
                                        </Alert>
                                    ) : (
                                        <Alert severity="success">
                                            All Sequent-side changes applied successfully.
                                        </Alert>
                                    )}
                                    {taskResult.rowFailures.length > 0 && (
                                        <Button
                                            onClick={handleDownloadFailures}
                                            sx={{alignSelf: "flex-start"}}
                                        >
                                            Download row failures report
                                        </Button>
                                    )}
                                </>
                            )}
                        </Stack>
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
                        {diffRows.length > 0 && (
                            <Button
                                variant="contained"
                                disabled={!isClean}
                                onClick={() => setConfirmOpen(true)}
                            >
                                Apply
                            </Button>
                        )}
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
                        <CategorySummary rows={sequentRows} highlighted={HIGHLIGHTED_CATEGORIES} />
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
                        Apply changes
                    </Button>
                </DialogActions>
            </Dialog>
        </Dialog>
    )
}

export default ReconciliationWizard
