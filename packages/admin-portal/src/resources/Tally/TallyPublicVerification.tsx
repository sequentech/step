// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useRef, useState} from "react"
import {useApolloClient} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    Chip,
    LinearProgress,
    List,
    ListItem,
    ListItemIcon,
    ListItemText,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
import {IPermissions} from "@/types/keycloak"
import {verifierThreads, workerVerifier, type Verifier} from "@/services/VerifierWorker"
import {usePresentationName} from "@/hooks/usePresentationName"
import {
    GET_TALLY_PUBLIC_VERIFICATION_BOARDS,
    type GetTallyPublicVerificationBoardsQuery,
} from "@/queries/GetTallyPublicVerificationBoards"
import {
    GET_TALLY_PUBLIC_VERIFICATION_TRANSCRIPT,
    type GetTallyPublicVerificationTranscriptQuery,
} from "@/queries/GetTallyPublicVerificationTranscript"
import {
    batchStatus,
    evidenceOf,
    overallStatus,
    verifyTallyBoards,
    type BatchStatus,
    type BatchVerification,
    type TallyVerificationBoard,
    type TranscriptLoader,
} from "@/services/TallyPublicVerification"

/**
 * The production verifier: the browser trustees' WASM bundle in a worker of
 * its own, so verifying never blocks the page.
 */
const startVerifier = (): Verifier =>
    workerVerifier(
        new URL("/wbraid-trustee/", window.location.origin).href,
        verifierThreads(navigator.hardwareConcurrency)
    )

const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error))

type State =
    | {kind: "idle"}
    | {kind: "running"; done: number; total: number | null}
    | {kind: "empty"}
    | {kind: "failed"; message: string}
    | {kind: "unavailable"; message: string}
    | {kind: "done"; batches: BatchVerification[]}

/** One click's run: stopped when the section goes away. */
interface Run {
    cancelled: boolean
    verifier?: Verifier
}

const SEVERITY: Record<BatchStatus, "success" | "warning" | "error"> = {
    verified: "success",
    incomplete: "warning",
    invalid: "error",
    error: "error",
}

// Warning chips fall below the WCAG contrast minimum; the summary's warning
// alert and the chip's label say a batch is incomplete.
const CHIP: Record<BatchStatus, "success" | "default" | "error"> = {
    verified: "success",
    incomplete: "default",
    invalid: "error",
    error: "error",
}

const CHECKS = ["ballots", "mixing", "decryption"] as const

export interface TallyPublicVerificationProps {
    electionEventId: string
    tallySessionId: string
    /** A tally's boards are complete once it has finished. */
    finished: boolean
    /** Starts a verifier for a run; the worker unless a story or test supplies one. */
    verifier?: () => Verifier
}

/**
 * Verifies every bulletin board batch of a tally session in the browser: the
 * ballots' zero-knowledge proofs, every shuffle and every decryption. The
 * platform reads each board for it, so the browser only needs the GraphQL API
 * it already uses, never the board service or its storage.
 */
export const TallyPublicVerification: React.FC<TallyPublicVerificationProps> = ({
    electionEventId,
    tallySessionId,
    finished,
    verifier = startVerifier,
}) => {
    const {t} = useTranslation()
    const client = useApolloClient()
    const [state, setState] = useState<State>({kind: "idle"})
    const active = useRef<Run | null>(null)

    // Leaving the section stops the run and its worker.
    useEffect(
        () => () => {
            const run = active.current
            if (!run) return
            run.cancelled = true
            run.verifier?.close()
        },
        []
    )
    const role = {headers: {"x-hasura-role": IPermissions.TALLY_RESULTS_READ}}

    // A transcript can be large and is read once: kept out of Apollo's cache.
    const load: TranscriptLoader = async (board) => {
        const {data} = await client.query<GetTallyPublicVerificationTranscriptQuery>({
            query: GET_TALLY_PUBLIC_VERIFICATION_TRANSCRIPT,
            variables: {electionEventId, tallySessionId, tallyBoard: board.tally_board},
            fetchPolicy: "no-cache",
            context: role,
        })
        return data.get_tally_public_verification_transcript
    }

    const verify = async () => {
        const run: Run = {cancelled: false}
        active.current = run
        setState({kind: "running", done: 0, total: null})
        try {
            const {data} = await client.query<GetTallyPublicVerificationBoardsQuery>({
                query: GET_TALLY_PUBLIC_VERIFICATION_BOARDS,
                variables: {electionEventId, tallySessionId},
                fetchPolicy: "network-only",
                context: role,
            })
            const boards = data.get_tally_public_verification_boards.boards
            if (!boards.length) {
                setState({kind: "empty"})
                return
            }
            run.verifier = verifier()
            try {
                await run.verifier.ready()
            } catch (error) {
                setState({kind: "unavailable", message: errorText(error)})
                return
            }
            const total = boards.length
            setState({kind: "running", done: 0, total})
            const batches = await verifyTallyBoards(
                run.verifier,
                load,
                boards,
                (done) => setState({kind: "running", done, total}),
                () => run.cancelled
            )
            setState({kind: "done", batches})
        } catch (error) {
            setState({kind: "failed", message: errorText(error)})
        } finally {
            run.verifier?.close()
        }
    }

    const download = (batches: BatchVerification[]) => {
        const blob = new Blob([evidenceOf(tallySessionId, batches)], {type: "application/json"})
        const url = URL.createObjectURL(blob)
        const link = document.createElement("a")
        link.href = url
        link.download = `tally-${tallySessionId}-public-verification.json`
        link.click()
        setTimeout(() => URL.revokeObjectURL(url), 0)
    }

    const running = state.kind === "running"
    const progress =
        state.kind !== "running"
            ? ""
            : state.total
              ? t("tally.publicVerification.progress", {
                    current: Math.min(state.done + 1, state.total),
                    total: state.total,
                })
              : t("tally.publicVerification.loading")
    return (
        <Stack spacing={2} className="seq-admin-tally-public-verification">
            <Typography variant="body2">{t("tally.publicVerification.description")}</Typography>
            <Stack direction="row" spacing={2} alignItems="center">
                <Button variant="contained" onClick={verify} disabled={!finished || running}>
                    {t(
                        state.kind === "done"
                            ? "tally.publicVerification.verifyAgain"
                            : "tally.publicVerification.verify"
                    )}
                </Button>
                {finished ? null : (
                    <Typography variant="body2" color="text.secondary">
                        {t("tally.publicVerification.unavailable")}
                    </Typography>
                )}
            </Stack>
            {state.kind === "running" ? (
                <Box role="status">
                    <LinearProgress
                        aria-label={progress}
                        variant={state.total ? "determinate" : "indeterminate"}
                        value={state.total ? (100 * state.done) / state.total : undefined}
                    />
                    <Typography variant="body2" sx={{mt: 1}}>
                        {progress}
                    </Typography>
                </Box>
            ) : null}
            {state.kind === "empty" ? (
                <Alert severity="info">{t("tally.publicVerification.empty")}</Alert>
            ) : null}
            {state.kind === "failed" ? (
                <Alert severity="error">
                    {t("tally.publicVerification.failed", {message: state.message})}
                </Alert>
            ) : null}
            {state.kind === "unavailable" ? (
                <Alert severity="error">
                    {t("tally.publicVerification.verifierUnavailable", {message: state.message})}
                </Alert>
            ) : null}
            {state.kind === "done" ? (
                <VerificationResults
                    batches={state.batches}
                    onDownload={() => download(state.batches)}
                />
            ) : null}
        </Stack>
    )
}

const VerificationResults: React.FC<{batches: BatchVerification[]; onDownload: () => void}> = ({
    batches,
    onDownload,
}) => {
    const {t} = useTranslation()
    const name = usePresentationName()
    const ownerOf = (board: TallyVerificationBoard) =>
        [
            name({presentation: board.election_presentation}),
            name({presentation: board.contest_presentation}),
            board.area_name,
        ]
            .filter(Boolean)
            .join(" · ")
    // The verifier explains failures in English; a pass reads in the interface's language.
    const detailOf = (batch: BatchVerification) => {
        if (!("report" in batch)) return batch.error
        const {status, detail} = batch.report
        return status === "invalid" ? detail : t(`tally.publicVerification.detail.${status}`)
    }
    const status = overallStatus(batches)
    const count = batches.filter((batch) => batchStatus(batch) === status).length
    return (
        <Stack spacing={2}>
            <Alert severity={SEVERITY[status]}>
                {t(`tally.publicVerification.summary.${status}`, {count, total: batches.length})}
            </Alert>
            {status === "verified" ? (
                <Box>
                    <Typography variant="subtitle2">
                        {t("tally.publicVerification.checks.title")}
                    </Typography>
                    <List dense>
                        {CHECKS.map((check) => (
                            <ListItem key={check}>
                                <ListItemIcon>
                                    <CheckCircleIcon color="success" />
                                </ListItemIcon>
                                <ListItemText
                                    primary={t(`tally.publicVerification.checks.${check}`)}
                                />
                            </ListItem>
                        ))}
                    </List>
                </Box>
            ) : null}
            <Table size="small">
                <TableHead>
                    <TableRow>
                        <TableCell>{t("tally.publicVerification.columns.batch")}</TableCell>
                        <TableCell align="right">
                            {t("tally.publicVerification.columns.ballots")}
                        </TableCell>
                        <TableCell>{t("tally.publicVerification.columns.result")}</TableCell>
                        <TableCell>{t("tally.publicVerification.columns.detail")}</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {batches.map((batch) => {
                        const batchState = batchStatus(batch)
                        // A contest area owns several batches under weighted
                        // voting: its batch number tells them apart.
                        const owner = ownerOf(batch.board)
                        const number = t("tally.publicVerification.batch", {
                            batch: batch.board.batch,
                        })
                        return (
                            <TableRow key={batch.board.tally_board}>
                                <TableCell>
                                    <Typography variant="body2">{owner || number}</Typography>
                                    <Typography variant="caption" color="text.secondary">
                                        {owner
                                            ? `${number} · ${batch.board.tally_board}`
                                            : batch.board.tally_board}
                                    </Typography>
                                </TableCell>
                                <TableCell align="right">
                                    {"report" in batch ? batch.report.ballot_count : null}
                                </TableCell>
                                <TableCell>
                                    <Chip
                                        size="small"
                                        color={CHIP[batchState]}
                                        label={t(`tally.publicVerification.status.${batchState}`)}
                                    />
                                </TableCell>
                                <TableCell>{detailOf(batch)}</TableCell>
                            </TableRow>
                        )
                    })}
                </TableBody>
            </Table>
            <Box>
                <Button variant="outlined" onClick={onDownload}>
                    {t("tally.publicVerification.download")}
                </Button>
            </Box>
        </Stack>
    )
}
