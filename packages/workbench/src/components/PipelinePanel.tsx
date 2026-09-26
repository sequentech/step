// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {
    Alert,
    Box,
    Button,
    Chip,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import type {BallotSelection} from "@sequentech/ui-core"
import {
    PipelineStatus,
    runBallotPipeline,
    type PipelineReport,
    type PipelineRun,
} from "voting-portal/src/preview/ballotPipeline"
import type {IBallotStyle} from "voting-portal/src/store/ballotStyles/ballotStylesSlice"

/** Where a pipeline's results come from; only the WASM runner exercises sequent-core. */
export enum ComputationKind {
    WASM = "wasm",
    MOCK = "mock",
}

export interface PipelineRunner {
    kind: ComputationKind
    run: PipelineRun
}

export const WASM_RUNNER: PipelineRunner = {kind: ComputationKind.WASM, run: runBallotPipeline}

export interface PipelineInput {
    ballotStyle: IBallotStyle
    selection: BallotSelection
}

export interface PipelinePanelProps {
    runner: PipelineRunner
    /** The ballot and the voter's current choices; absent where no ballot is loaded. */
    input?: PipelineInput
    wasmReady: boolean
}

const STATUS_COLORS: Record<PipelineStatus, "success" | "error" | "default"> = {
    [PipelineStatus.PASSED]: "success",
    [PipelineStatus.FAILED]: "error",
    [PipelineStatus.SKIPPED]: "default",
}

/** One line per contest naming each marked candidate and its mark or rank. */
const describeMarks = (selection: BallotSelection, {ballot_eml}: IBallotStyle) =>
    selection.map(({contest_id, choices}) => {
        const contest = ballot_eml.contests.find(({id}) => id === contest_id)
        const name = (id: string) => contest?.candidates.find((option) => option.id === id)?.name
        const marked = choices.filter(({selected}) => selected > -1)
        return `${contest?.name ?? contest_id}: ${
            marked.length
                ? marked.map(({id, selected}) => `${name(id) ?? id} (${selected})`).join(", ")
                : "no marks"
        }`
    })

/** Runs the voting screen's ballot operations on the current selection. */
export const PipelinePanel: React.FC<PipelinePanelProps> = ({runner, input, wasmReady}) => {
    const [result, setResult] = useState<{report: PipelineReport; input: PipelineInput}>()
    const isMock = runner.kind === ComputationKind.MOCK
    return (
        <Box className="pipeline-panel">
            <Stack direction="row" spacing={1} alignItems="center" sx={{mb: 1}}>
                <Chip
                    label={isMock ? "Mocked computation" : "sequent-core WASM"}
                    color={isMock ? "secondary" : "primary"}
                    variant={isMock ? "filled" : "outlined"}
                />
                {!wasmReady && !isMock ? (
                    <Typography variant="body2">Loading sequent-core…</Typography>
                ) : null}
            </Stack>
            <Typography variant="body2" color="text.secondary" sx={{mb: 1}}>
                Interprets the selection, applies the Next and warning checks, encrypts it, hashes
                the ballot and decodes it back, as the voting screen does.
            </Typography>
            {input ? (
                <Box component="ul" sx={{m: 0, mb: 1, pl: 2, fontSize: 12}}>
                    {describeMarks(input.selection, input.ballotStyle).map((line) => (
                        <li key={line}>{line}</li>
                    ))}
                </Box>
            ) : (
                <Alert severity="info" role="status" sx={{mb: 1}}>
                    Open an election screen to load a ballot.
                </Alert>
            )}
            <Button
                variant="contained"
                disabled={!input || (!wasmReady && !isMock)}
                onClick={() =>
                    input &&
                    setResult({report: runner.run(input.ballotStyle, input.selection), input})
                }
            >
                Run ballot pipeline
            </Button>
            {result ? (
                <Box sx={{mt: 2}}>
                    {result.input !== input ? (
                        <Alert severity="info" role="status" sx={{mb: 1}}>
                            The selection changed after this run.
                        </Alert>
                    ) : null}
                    <Table size="small" aria-label="Pipeline steps">
                        <TableHead>
                            <TableRow>
                                <TableCell>Step</TableCell>
                                <TableCell>Result</TableCell>
                                <TableCell align="right">ms</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {result.report.steps.map(({step, status, detail, durationMs}) => (
                                <TableRow key={step}>
                                    <TableCell>{step}</TableCell>
                                    <TableCell sx={{wordBreak: "break-all"}}>
                                        <Chip
                                            size="small"
                                            label={status}
                                            color={STATUS_COLORS[status]}
                                            sx={{mr: 1}}
                                        />
                                        {detail}
                                    </TableCell>
                                    <TableCell align="right">{durationMs.toFixed(1)}</TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                    {result.report.decoded ? (
                        <Box sx={{mt: 1, fontSize: 12}}>
                            Decoded:{" "}
                            {describeMarks(result.report.decoded, result.input.ballotStyle).join(
                                "; "
                            )}
                        </Box>
                    ) : null}
                </Box>
            ) : null}
        </Box>
    )
}
