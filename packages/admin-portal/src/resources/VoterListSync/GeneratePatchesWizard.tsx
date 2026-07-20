// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo, useState} from "react"
import {Alert, Box, Button, CircularProgress, Stack, Tooltip, Typography} from "@mui/material"
import {DropFile} from "@sequentech/ui-essentials"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {SyncDiffTable} from "./SyncDiffTable"
import {CategorySummary} from "./CategorySummary"
import {MockTaskProgress} from "./MockTaskProgress"
import {
    ESyncChangeCategory,
    ESyncPatchTarget,
    SyncDiffRow,
    SyncFileMeta,
    SyncTaskResult,
    SyncVoterRecord,
} from "./types"
import {
    MOCK_GENERATE_PATCH_STEPS,
    buildPatchCsv,
    buildRowFailuresCsv,
    downloadTextFile,
    formatGeneratedAt,
    mockCalculateReconciliationDiff,
    mockUploadToS3,
    parseReconciliationFile,
    runMockTask,
    validateSequence,
} from "./mockSyncEngine"

type WizardStep = "idle" | "processing" | "review" | "generating" | "done"

interface GeneratePatchesWizardProps {
    lastAppliedSequence: number
    pendingIndeterminateCount: number
}

/**
 * "Patches creation flow" from DatafixPossibleImplementation.md: drop the
 * reconciliation file, review the calculated diff, generate the Datafix
 * patch (or the Sequent patch once the import is clean), then download the
 * results. Backend interaction (S3 upload, diff calculation, the Celery
 * patch-generation task) is mocked in mockSyncEngine.ts.
 */
export const GeneratePatchesWizard: React.FC<GeneratePatchesWizardProps> = ({
    lastAppliedSequence,
    pendingIndeterminateCount,
}) => {
    const [step, setStep] = useState<WizardStep>("idle")
    const [fileName, setFileName] = useState<string | null>(null)
    const [meta, setMeta] = useState<SyncFileMeta | null>(null)
    const [diffRows, setDiffRows] = useState<SyncDiffRow[]>([])
    const [records, setRecords] = useState<SyncVoterRecord[]>([])
    const [sequenceError, setSequenceError] = useState<string | null>(null)
    const [taskStatus, setTaskStatus] = useState<ETaskExecutionStatus>(ETaskExecutionStatus.STARTED)
    const [taskLogs, setTaskLogs] = useState<string[]>([])
    const [taskResult, setTaskResult] = useState<SyncTaskResult | null>(null)
    // MOCK: simulates "repeat from step 1" in the operator flow - each new
    // import within this session is treated as the next reconciliation file,
    // so round 0 needs a Datafix patch, round 1 comes back clean on the
    // Datafix side (Sequent patch only), round 2+ is fully converged. Not
    // reset by reset() - it tracks session progress across imports.
    const [round, setRound] = useState(0)

    const failureRows = useMemo(
        () => diffRows.filter((row) => row.category === ESyncChangeCategory.ROW_FAILURE),
        [diffRows]
    )
    const actionableRows = useMemo(
        () => diffRows.filter((row) => row.category !== ESyncChangeCategory.ROW_FAILURE),
        [diffRows]
    )
    // The Datafix patch always goes first; the Sequent patch only once the
    // import is clean on the Datafix side (step 3-4 of the flow).
    const patchTarget: ESyncPatchTarget | null = actionableRows.some(
        (row) => row.target === "datafix"
    )
        ? "datafix"
        : actionableRows.some((row) => row.target === "sequent")
          ? "sequent"
          : null

    const reset = () => {
        setStep("idle")
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
            setStep("idle")
            return
        }

        // MOCK: backend diff calculation (see mockCalculateReconciliationDiff).
        const diff = await mockCalculateReconciliationDiff(rows, round)
        setRound((previous) => previous + 1)
        setDiffRows(diff.diffRows)
        setRecords(diff.records)
        setStep("review")
    }

    const handleGenerate = async () => {
        if (!patchTarget) {
            return
        }
        setStep("generating")
        setTaskStatus(ETaskExecutionStatus.IN_PROGRESS)
        setTaskLogs([])
        const result = await runMockTask(MOCK_GENERATE_PATCH_STEPS, diffRows, (log) =>
            setTaskLogs((previous) => [...previous, log])
        )
        setTaskResult(result)
        setTaskStatus(ETaskExecutionStatus.SUCCESS)
        setStep("done")
    }

    const handleDownloadPatch = () => {
        if (!meta || !patchTarget) {
            return
        }
        // MOCK: the real patch document is generated and uploaded to S3 by the
        // backend task; here we just re-serialize the diff we already have.
        downloadTextFile(
            `${patchTarget}_patch_seq${meta.sequence}.csv`,
            buildPatchCsv(meta, records, patchTarget)
        )
    }

    const handleDownloadFailures = () => {
        if (!taskResult) {
            return
        }
        downloadTextFile(
            `row_failures_seq${meta?.sequence ?? 0}.csv`,
            buildRowFailuresCsv(taskResult.rowFailures)
        )
    }

    return (
        <Stack spacing={3}>
            <Box component="ol" sx={{color: "text.secondary", margin: 0, paddingLeft: "1.25rem"}}>
                <Typography component="li" color="text.secondary">
                    Step 1: Drop the reconciliation file Datafix produced - the diff is calculated
                    automatically.
                </Typography>
                <Typography component="li" color="text.secondary">
                    Step 2: Generate the Datafix patch (or jump to 4 when importing a clean
                    reconciliation file).
                </Typography>
                <Typography component="li" color="text.secondary">
                    Step 3: Import the next reconciliation file.
                </Typography>
                <Typography component="li" color="text.secondary">
                    Step 4: Generate the Sequent patch once a re-import comes back clean.
                </Typography>
            </Box>

            {pendingIndeterminateCount > 0 && (
                <Alert severity="warning">
                    {pendingIndeterminateCount} indeterminate ballot(s) are still unresolved for
                    this election event. Importing is not blocked, but resolve them from the
                    "Indeterminate votes" tab first if possible - see "Indeterminate Ballot
                    Resolution" in DatafixPossibleImplementation.md.
                </Alert>
            )}

            {step === "idle" && (
                <>
                    {sequenceError && <Alert severity="error">{sequenceError}</Alert>}
                    <DropFile handleFiles={handleFiles} accept=".csv" formatLabel="CSV file" />
                </>
            )}

            {step === "processing" && (
                <Stack direction="row" spacing={2} alignItems="center">
                    <CircularProgress size={20} />
                    <Typography>Uploading {fileName} and calculating the diff...</Typography>
                </Stack>
            )}

            {(step === "review" || step === "generating" || step === "done") && meta && (
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
                        {round - 1 === 0 && " (fresh import - Datafix-side changes expected)"}
                        {round - 1 === 1 && " (clean on the Datafix side - Sequent patch only)"}
                        {round - 1 >= 2 && " (converged - both systems in sync)"}
                    </Typography>
                </Box>
            )}

            {step === "review" && (
                <>
                    <CategorySummary rows={diffRows} />
                    {failureRows.length > 0 && (
                        <Alert severity="warning">
                            {failureRows.length} row(s) have an unexpected CountyMun and will be
                            excluded from the patch - reported as row failures at the end.
                        </Alert>
                    )}
                    <SyncDiffTable rows={diffRows} showTarget />
                    <Stack direction="row" spacing={2} alignItems="center">
                        {patchTarget ? (
                            <Button variant="contained" onClick={handleGenerate}>
                                Generate {patchTarget === "datafix" ? "Datafix" : "Sequent"} patch
                            </Button>
                        ) : (
                            <Alert severity="success" sx={{flex: 1}}>
                                No differences to resolve - the two systems are already in sync.
                            </Alert>
                        )}
                        <Button onClick={reset}>Start over</Button>
                    </Stack>
                </>
            )}

            {(step === "generating" || step === "done") && patchTarget && (
                <MockTaskProgress
                    title={`Generate ${patchTarget === "datafix" ? "Datafix" : "Sequent"} patch`}
                    status={taskStatus}
                    logs={taskLogs}
                />
            )}

            {step === "done" && taskResult && (
                <Stack spacing={2}>
                    {taskResult.rowFailures.length > 0 && (
                        <Alert severity="warning">
                            {taskResult.rowFailures.length} row(s) failed and were not patched -
                            raise these with Datafix.
                        </Alert>
                    )}
                    <Stack direction="row" spacing={2} flexWrap="wrap">
                        <Button variant="contained" onClick={handleDownloadPatch}>
                            Download {patchTarget === "datafix" ? "Datafix" : "Sequent"} patch
                        </Button>
                        {taskResult.rowFailures.length > 0 && (
                            <Button onClick={handleDownloadFailures}>
                                Download row failures report
                            </Button>
                        )}
                        <Tooltip title="SFTP integration is not wired up in this prototype">
                            <span>
                                <Button disabled>Upload patch to SFTP</Button>
                            </span>
                        </Tooltip>
                        <Button onClick={reset}>Import next reconciliation file</Button>
                    </Stack>
                </Stack>
            )}
        </Stack>
    )
}

export default GeneratePatchesWizard
