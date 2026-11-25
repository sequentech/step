// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PublicVerificationReport, TrusteeModule, VerificationAnchors} from "./BrowserTrustee"

/** Where elections and contests keep their names: `i18n.<language>.name`. */
export interface Presentation {
    i18n?: Record<string, Record<string, string | null | undefined>>
    language_conf?: {default_language_code?: string | null} | null
}

/** A tally board of a tally session, as `get_tally_public_verification_boards` lists it. */
export interface TallyVerificationBoard {
    dkg_board: string
    tally_board: string
    batch: number
    /** The election, contest and area that own the batch, when it has an owner. */
    election_presentation: Presentation | null
    contest_presentation: Presentation | null
    area_name: string | null
    /** The DKG board's `Configuration` message, in padded base64. */
    configuration_message: string
    /** The tally board's `Ballots` message, in padded base64. */
    ballots_message: string
}

export type TallyVerifier = TrusteeModule["WasmVerifier"]

/** A board's public transcript, as `get_tally_public_verification_transcript` reads it. */
export interface BoardMessages {
    dkg_messages: string[]
    tally_messages: string[]
}

/**
 * Reads a tally board's messages. The platform reads them from the board
 * service, so the browser needs to reach neither the board nor its storage.
 */
export type TranscriptLoader = (board: TallyVerificationBoard) => Promise<BoardMessages>

/** The transcript `verifyTranscript` and `trustee verify --transcript` read. */
export const transcriptOf = (messages: BoardMessages): string =>
    JSON.stringify({
        version: 1,
        dkg_messages: messages.dkg_messages,
        tally_messages: messages.tally_messages,
    })

/** One board's verification: its report, or why there is none. */
export type BatchVerification =
    | {
          board: TallyVerificationBoard
          anchors: VerificationAnchors
          transcript: string
          report: PublicVerificationReport
      }
    | {board: TallyVerificationBoard; error: string}

export type BatchStatus = PublicVerificationReport["status"] | "error"

export const batchStatus = (batch: BatchVerification): BatchStatus =>
    "report" in batch ? batch.report.status : "error"

/** The most serious status of the batches: a failed proof first. */
export function overallStatus(batches: BatchVerification[]): BatchStatus {
    const statuses = batches.map(batchStatus)
    return (
        (["invalid", "error", "incomplete"] as const).find((status) => statuses.includes(status)) ??
        "verified"
    )
}

/** Anchors derived from the platform's records of a board, never from the board itself. */
export const anchorsOf = (
    verifier: TallyVerifier,
    board: TallyVerificationBoard
): VerificationAnchors => ({
    configuration_hash: verifier.configurationHash(board.configuration_message),
    tally_id: verifier.tallyIdentifier(board.configuration_message, board.ballots_message),
    ballots_hash: verifier.ballotsHash(board.ballots_message),
    ballot_ciphertexts: [],
})

/**
 * An error's first line. The WASM verifier reports an error as
 * `JsValue(<error>\n<its stack>)`, which keeps just `<error>`.
 */
function message(error: unknown): string {
    const text = error instanceof Error ? error.message : String(error)
    const wrapped = /^JsValue\(([^\n]*)/.exec(text)
    return wrapped ? wrapped[1].replace(/\)$/, "") : text.split("\n")[0]
}

async function verifyBoard(
    verifier: TallyVerifier,
    load: TranscriptLoader,
    board: TallyVerificationBoard
): Promise<BatchVerification> {
    try {
        const anchors = anchorsOf(verifier, board)
        const transcript = transcriptOf(await load(board))
        const report: PublicVerificationReport = JSON.parse(
            verifier.verifyTranscript(transcript, JSON.stringify(anchors))
        )
        return {board, anchors, transcript, report}
    } catch (error) {
        return {board, error: message(error)}
    }
}

/** Verify the boards one after another; a board that fails does not stop the rest. */
export async function verifyTallyBoards(
    verifier: TallyVerifier,
    load: TranscriptLoader,
    boards: TallyVerificationBoard[],
    onProgress: (done: number) => void
): Promise<BatchVerification[]> {
    const batches: BatchVerification[] = []
    for (const board of boards) {
        batches.push(await verifyBoard(verifier, load, board))
        onProgress(batches.length)
    }
    return batches
}

/**
 * The evidence of a run, for re-checking offline: each batch's `transcript`
 * is the file `trustee verify --transcript` reads, with its `anchors`.
 */
export function evidenceOf(tallySessionId: string, batches: BatchVerification[]): string {
    return JSON.stringify(
        {
            tally_session_id: tallySessionId,
            batches: batches.map((batch) => ({
                dkg_board: batch.board.dkg_board,
                tally_board: batch.board.tally_board,
                batch: batch.board.batch,
                ...("report" in batch
                    ? {
                          anchors: batch.anchors,
                          report: batch.report,
                          transcript: JSON.parse(batch.transcript),
                      }
                    : {error: batch.error}),
            })),
        },
        null,
        2
    )
}
