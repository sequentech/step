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

/** The board service's address: the configured one, or the portal's own `/b4`. */
export const boardUrl = (configured: string | undefined, origin: string): string =>
    new URL(configured || "/b4", origin).href.replace(/\/+$/, "")

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

const message = (error: unknown) => (error instanceof Error ? error.message : String(error))

async function verifyBoard(
    verifier: TallyVerifier,
    url: string,
    board: TallyVerificationBoard
): Promise<BatchVerification> {
    try {
        const anchors = anchorsOf(verifier, board)
        const transcript = await verifier.fetchTranscript(url, board.dkg_board, board.tally_board)
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
    url: string,
    boards: TallyVerificationBoard[],
    onProgress: (done: number) => void
): Promise<BatchVerification[]> {
    const batches: BatchVerification[] = []
    for (const board of boards) {
        batches.push(await verifyBoard(verifier, url, board))
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
