// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * End-to-end tally for an election whose cast votes have been
 * captured by the workbench bridge.
 *
 * The pipeline (entirely in-browser, no portal-source changes):
 *
 *   ballotStyle.ballot_eml.contests[i]
 *     → JSON-stringified, fed verbatim to velvet-wasm's
 *       tally_plaintext_ballots (which deserializes it as the Rust
 *       `sequent_core::ballot::Contest` — same serde shape, no
 *       transformation needed).
 *
 *   repairedCastVotes[id].decodedBigInts[contestId]
 *     → the decimal `BigUint` recovered by decrypting
 *       `castVote.content` with the workbench-owned secret key. This
 *       is the exact byte `encode_ballot` would produce from the
 *       matching selection, so it feeds `tally_plaintext_ballots`
 *       directly and exercises the *real* encrypt -> decrypt path
 *       rather than re-encoding from the plaintext selection.
 *
 * Voting-method support is whatever the velvet-wasm tally exposes
 * today (PluralityAtLarge, InstantRunoff). Anything else surfaces as
 * an `unsupported` status, not a crash.
 */

import {runTally} from "./tally"

export interface ContestTallyOutcome {
    contestId: string
    contestName?: string
    ballotsCounted: number
    /** "ok" — tally ran and produced a result.
     *  "no-data" — contest exists on the ballot but no captured cast
     *  vote contained a decrypted BigUint for it.
     *  "error" — tally failed; `errorMessage` is set. */
    status: "ok" | "no-data" | "error"
    /** Parsed `ContestResult` JSON when status is "ok". */
    result?: unknown
    errorMessage?: string
}

/**
 * Run the per-contest tally for one election.
 *
 * @param ballotStyle The `IBallotStyle` for this election (workbench
 *   reads it from portal Redux via `state.ballotStyles`). Its
 *   `ballot_eml.contests[i]` JSON is the exact shape velvet-wasm
 *   deserializes as `Contest`.
 * @param decodedBigIntsByCastVote The bridge ledger filtered to this
 *   election: a list of `Record<contestId, decimalBigUint>` blobs,
 *   one per cast vote, in cast order. Entries for contests the bridge
 *   could not decrypt are simply absent and skipped.
 */
export async function runElectionTally(
    ballotStyle: {
        ballot_eml: {
            contests: Array<{
                id: string
                name?: string
                title?: string
            }>
        }
    },
    decodedBigIntsByCastVote: Array<Record<string, string>>
): Promise<ContestTallyOutcome[]> {
    const outcomes: ContestTallyOutcome[] = []

    for (const contest of ballotStyle.ballot_eml.contests) {
        const contestJson = JSON.stringify(contest)
        const contestId = contest.id
        const contestName = contest.name ?? contest.title

        const ballots: string[] = []
        for (const decoded of decodedBigIntsByCastVote) {
            const big = decoded[contestId]
            if (typeof big === "string" && big.length > 0) ballots.push(big)
        }

        if (ballots.length === 0) {
            outcomes.push({
                contestId,
                contestName,
                ballotsCounted: 0,
                status: "no-data",
            })
            continue
        }

        try {
            const result = await runTally(contestJson, ballots)
            outcomes.push({
                contestId,
                contestName,
                ballotsCounted: ballots.length,
                status: "ok",
                result,
            })
        } catch (e) {
            outcomes.push({
                contestId,
                contestName,
                ballotsCounted: ballots.length,
                status: "error",
                errorMessage: e instanceof Error ? e.message : String(e),
            })
        }
    }

    return outcomes
}
