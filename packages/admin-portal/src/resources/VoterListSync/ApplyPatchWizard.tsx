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
    Stack,
    Typography,
} from "@mui/material"
import {DropFile} from "@sequentech/ui-essentials"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {SyncDiffTable} from "./SyncDiffTable"
import {CategorySummary} from "./CategorySummary"
import {MockTaskProgress} from "./MockTaskProgress"
import {ESyncChangeCategory, SyncDiffRow, SyncFileMeta, SyncTaskResult} from "./types"
import {HIGHLIGHTED_CATEGORIES, CATEGORY_LABELS} from "./constants"
import {
    MOCK_APPLY_PATCH_STEPS,
    buildRowFailuresCsv,
    downloadTextFile,
    formatGeneratedAt,
    mockUploadToS3,
    parsePatchFile,
    runMockTask,
    validateSequence,
} from "./mockSyncEngine"

type WizardStep = "idle" | "processing" | "review" | "applying" | "done"

interface ApplyPatchWizardProps {
    lastAppliedSequence: number
    onApplied: (sequence: number) => void
}

const summarizeForConfirmation = (rows: SyncDiffRow[]): string => {
    const counts = new Map<ESyncChangeCategory, number>()
    rows.forEach((row) => counts.set(row.category, (counts.get(row.category) ?? 0) + 1))

    const phrases = [
        [ESyncChangeCategory.VOTED_OTHER_CHANNEL, "marks", "voter(s) as voted via other channels"],
        [ESyncChangeCategory.DISABLED, "disables", "voter(s)"],
        [ESyncChangeCategory.VOTED_INTERNET, "confirms", "internet vote(s)"],
        [ESyncChangeCategory.DELETION_REVERTED, "reverts", "deletion(s)"],
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
        ? `This patch ${parts.join(", ")}.`
        : "This patch has no changes to apply."
}

/**
 * "Reconciliation flow" from DatafixPossibleImplementation.md: drop the
 * Sequent patch, review the diff, confirm at the aggregate category level
 * (no per-row approval), then apply. Backend interaction is mocked in
 * mockSyncEngine.ts.
 */
export const ApplyPatchWizard: React.FC<ApplyPatchWizardProps> = ({
    lastAppliedSequence,
    onApplied,
}) => {
    const [step, setStep] = useState<WizardStep>("idle")
    const [fileName, setFileName] = useState<string | null>(null)
    const [meta, setMeta] = useState<SyncFileMeta | null>(null)
    const [diffRows, setDiffRows] = useState<SyncDiffRow[]>([])
    const [sequenceError, setSequenceError] = useState<string | null>(null)
    const [confirmOpen, setConfirmOpen] = useState(false)
    const [taskStatus, setTaskStatus] = useState<ETaskExecutionStatus>(ETaskExecutionStatus.STARTED)
    const [taskLogs, setTaskLogs] = useState<string[]>([])
    const [taskResult, setTaskResult] = useState<SyncTaskResult | null>(null)

    const confirmationSummary = useMemo(() => summarizeForConfirmation(diffRows), [diffRows])

    const reset = () => {
        setStep("idle")
        setFileName(null)
        setMeta(null)
        setDiffRows([])
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
        const {meta: fileMeta, rows} = parsePatchFile(text)
        setMeta(fileMeta)

        // The real check validates the patch's Sequence matches the current
        // reconciliation round; this mock reuses the simpler monotonic check.
        const sequenceCheck = validateSequence(fileMeta, lastAppliedSequence)
        if (!sequenceCheck.valid) {
            setSequenceError(sequenceCheck.message ?? "Stale file.")
            setStep("idle")
            return
        }

        setDiffRows(rows)
        setStep("review")
    }

    const handleApply = async () => {
        setConfirmOpen(false)
        setStep("applying")
        setTaskStatus(ETaskExecutionStatus.IN_PROGRESS)
        setTaskLogs([])
        const result = await runMockTask(MOCK_APPLY_PATCH_STEPS, diffRows, (log) =>
            setTaskLogs((previous) => [...previous, log])
        )
        setTaskResult(result)
        setTaskStatus(ETaskExecutionStatus.SUCCESS)
        setStep("done")
        if (meta) {
            onApplied(meta.sequence)
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
        <Stack spacing={3}>
            <Typography color="text.secondary">
                Drop the Sequent patch generated by the previous wizard. Review the changes, then
                approve at the category level - there is no per-row approval.
            </Typography>

            {step === "idle" && (
                <>
                    {sequenceError && <Alert severity="error">{sequenceError}</Alert>}
                    <DropFile handleFiles={handleFiles} accept=".csv" formatLabel="CSV file" />
                </>
            )}

            {step === "processing" && (
                <Stack direction="row" spacing={2} alignItems="center">
                    <CircularProgress size={20} />
                    <Typography>Uploading {fileName} and reading the patch...</Typography>
                </Stack>
            )}

            {(step === "review" || step === "applying" || step === "done") && meta && (
                <Box sx={{border: "1px solid", borderColor: "divider", borderRadius: 1, p: 1.5}}>
                    <Typography variant="body2" color="text.secondary">
                        {fileName} - Sequence {meta.sequence}, generated{" "}
                        {formatGeneratedAt(meta.generatedAt)}
                    </Typography>
                </Box>
            )}

            {step === "review" && (
                <>
                    <CategorySummary rows={diffRows} highlighted={HIGHLIGHTED_CATEGORIES} />
                    <SyncDiffTable rows={diffRows} />
                    <Stack direction="row" spacing={2}>
                        <Button
                            variant="contained"
                            disabled={diffRows.length === 0}
                            onClick={() => setConfirmOpen(true)}
                        >
                            Review and apply changes
                        </Button>
                        <Button onClick={reset}>Start over</Button>
                    </Stack>
                </>
            )}

            {(step === "applying" || step === "done") && (
                <MockTaskProgress title="Apply Sequent patch" status={taskStatus} logs={taskLogs} />
            )}

            {step === "done" && taskResult && (
                <Stack spacing={2}>
                    {taskResult.rowFailures.length > 0 ? (
                        <Alert severity="warning">
                            {taskResult.rowFailures.length} row(s) failed to apply - fix manually;
                            the next reconciliation file will point out the same diff otherwise.
                        </Alert>
                    ) : (
                        <Alert severity="success">All rows applied successfully.</Alert>
                    )}
                    <Stack direction="row" spacing={2}>
                        {taskResult.rowFailures.length > 0 && (
                            <Button onClick={handleDownloadFailures}>
                                Download row failures report
                            </Button>
                        )}
                        <Button onClick={reset}>Apply another patch</Button>
                    </Stack>
                </Stack>
            )}

            <Dialog open={confirmOpen} onClose={() => setConfirmOpen(false)}>
                <DialogTitle>Confirm reconciliation changes</DialogTitle>
                <DialogContent>
                    <Stack spacing={2} sx={{pt: 1}}>
                        <Typography>{confirmationSummary}</Typography>
                        <CategorySummary rows={diffRows} highlighted={HIGHLIGHTED_CATEGORIES} />
                        <Typography variant="caption" color="text.secondary">
                            Categories outlined in orange ({" "}
                            {[
                                ESyncChangeCategory.VOTED_INTERNET,
                                ESyncChangeCategory.VOTED_OTHER_CHANNEL,
                                ESyncChangeCategory.DISABLED,
                            ]
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
        </Stack>
    )
}

export default ApplyPatchWizard
