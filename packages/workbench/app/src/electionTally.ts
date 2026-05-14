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
 *     → JSON-stringified, fed verbatim to velvet-wasm's encode_ballot
 *       / tally_plaintext_ballots (which deserialize it as the Rust
 *       `sequent_core::ballot::Contest` — same serde shape, no
 *       transformation needed).
 *
 *   repairedCastVotes[castVoteId].selection
 *     → the `IDecodedVoteContest[]` the workbench snapshotted from
 *       Redux state.ballotSelections at cast time. Each element
 *       JSON-stringifies to the Rust `DecodedVoteContest` shape, so
 *       encode_ballot accepts it verbatim too.
 *
 * For each contest in the ballot style we:
 *   1. encode_ballot per cast vote → decimal BigUint string;
 *   2. tally_plaintext_ballots(contest, ballots[]) → ContestResult JSON.
 *
 * Voting-method support is whatever the velvet-wasm tally exposes
 * today (PluralityAtLarge, InstantRunoff). Anything else surfaces as
 * an `unsupported` status, not a crash.
 */

import {encodeBallot, runTally} from "./tally"

/** Subset of `IDecodedVoteContest` we rely on. Kept structural to
 *  avoid pulling the portal type into the workbench bridge layer —
 *  the JSON round-trip handles the rest. */
interface DecodedVoteContestLike {
    contest_id: string
}

export interface ContestTallyOutcome {
    contestId: string
    contestName?: string
    ballotsCounted: number
    /** "ok" — tally ran and produced a result.
     *  "no-data" — contest exists on the ballot but no captured cast
     *  vote contained a selection for it.
     *  "error" — encode or tally failed; `errorMessage` is set. */
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
 * @param selectionsByCastVote The bridge ledger filtered to this
 *   election: a list of `IDecodedVoteContest[]` blobs, one per cast
 *   vote, in cast order. The array itself may be empty (no votes
 *   yet); individual entries may be empty arrays (defensive).
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
    selectionsByCastVote: unknown[]
): Promise<ContestTallyOutcome[]> {
    const outcomes: ContestTallyOutcome[] = []

    for (const contest of ballotStyle.ballot_eml.contests) {
        const contestJson = JSON.stringify(contest)
        const contestId = contest.id
        const contestName = contest.name ?? contest.title

        // Collect the per-cast-vote decoded contest blobs that
        // match this contest_id. Skip cast votes that don't carry
        // one (defensive — should not happen with the current
        // single-contest seed, but the bridge schema doesn't
        // enforce it).
        const decodedContestsForThisContest: unknown[] = []
        for (const selection of selectionsByCastVote) {
            if (!Array.isArray(selection)) continue
            const match = (selection as DecodedVoteContestLike[]).find(
                (dvc) => dvc?.contest_id === contestId
            )
            if (match) {
                decodedContestsForThisContest.push(match)
            }
        }

        if (decodedContestsForThisContest.length === 0) {
            outcomes.push({
                contestId,
                contestName,
                ballotsCounted: 0,
                status: "no-data",
            })
            continue
        }

        try {
            const ballots: string[] = []
            for (const dvc of decodedContestsForThisContest) {
                ballots.push(
                    await encodeBallot(contestJson, JSON.stringify(dvc))
                )
            }
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
                ballotsCounted: decodedContestsForThisContest.length,
                status: "error",
                errorMessage: e instanceof Error ? e.message : String(e),
            })
        }
    }

    return outcomes
}
