// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {EAllowTally, EVotingStatus, IElectionStatus} from "@sequentech/ui-core"
import {EBallotBoxSealStatus, type IBallotBoxSeal} from "@/types/ballotBoxSeal"

type TallyElection = {
    id: string
    status?: (Partial<IElectionStatus> & {allow_tally?: EAllowTally}) | null
}

/** Where the ballot boxes of one election are on their way to the tally (VOTE-FREEZE). */
export enum EBallotBoxesReadiness {
    /** Every ballot box is sealed and its seal is on the bulletin board. */
    READY = "ready",
    /** Every ballot box is sealed; some seals are still being posted. */
    PUBLISHING = "publishing",
    /** Voting closed; the ballot boxes are sealed when the grace period ends. */
    SEALING = "sealing",
    /** Past the deadline and still not sealed: the card says why. */
    OVERDUE = "overdue",
    /** A seal failed: an incident. The ballot box stays locked. */
    FAILED = "failed",
    /** Voting has not closed: no ballot box has a seal row yet. */
    NOT_SEALED = "not-sealed",
    /** The seals are still loading. */
    LOADING = "loading",
    /** The seals could not be read: their state is unknown. */
    UNAVAILABLE = "unavailable",
}

export interface IBallotBoxesSummary {
    readiness: EBallotBoxesReadiness
    /** Ballot boxes of the election: one seal row each, from the close on. */
    total: number
    /** Sealed, whether or not on the bulletin board yet. */
    sealed: number
    /** Sealed and on the bulletin board. */
    published: number
    /** The latest end of the grace period, while some ballot box is waiting for it. */
    deadline?: string
}

/**
 * The ballot boxes of one election, from its seal rows. The close inserts a
 * row for every ballot box the tally expects, so an election without rows has
 * not closed. Follows the server's rule (`validate_ballot_boxes_sealed`): it
 * can be tallied only when every row is published. The server checks every
 * expected box (published ballot-style areas and areas with ballots); a box
 * added after the close has no row here, and only the server refuses it.
 */
export const summarizeBallotBoxes = (
    seals: IBallotBoxSeal[],
    now: Date = new Date()
): IBallotBoxesSummary => {
    const count = (...statuses: EBallotBoxSealStatus[]) =>
        seals.filter((seal) => statuses.includes(seal.status)).length
    const published = count(EBallotBoxSealStatus.PUBLISHED)
    const sealed = count(EBallotBoxSealStatus.SEALED, EBallotBoxSealStatus.PUBLISHED)
    const pending = seals.filter((seal) => seal.status === EBallotBoxSealStatus.PENDING)
    const summary = {total: seals.length, sealed, published}
    if (count(EBallotBoxSealStatus.FAILED)) {
        return {...summary, readiness: EBallotBoxesReadiness.FAILED}
    }
    if (!seals.length) {
        return {...summary, readiness: EBallotBoxesReadiness.NOT_SEALED}
    }
    if (pending.length) {
        const deadline = pending
            .map((seal) => seal.grace_deadline)
            .reduce((latest, value) =>
                new Date(value).getTime() > new Date(latest).getTime() ? value : latest
            )
        return {
            ...summary,
            readiness:
                now.getTime() < new Date(deadline).getTime()
                    ? EBallotBoxesReadiness.SEALING
                    : EBallotBoxesReadiness.OVERDUE,
            deadline,
        }
    }
    return {
        ...summary,
        readiness:
            published === seals.length
                ? EBallotBoxesReadiness.READY
                : EBallotBoxesReadiness.PUBLISHING,
    }
}

/** The summary of each election (by id) of the given seal rows. */
export const summarizeBallotBoxesByElection = (
    electionIds: string[],
    seals: IBallotBoxSeal[],
    now: Date = new Date()
): Record<string, IBallotBoxesSummary> =>
    Object.fromEntries(
        electionIds.map((id) => [
            id,
            summarizeBallotBoxes(
                seals.filter((seal) => seal.election_id === id),
                now
            ),
        ])
    )

/**
 * Why the selected elections cannot be tallied, as a `tally.eligibility.*`
 * key, or undefined when they can. `ballotBoxes` is given when the event
 * seals its ballot boxes at close; an election is then tallied only once all
 * its ballot boxes are sealed and on the bulletin board, as the server checks.
 */
export const getTallyDisabledReason = (
    elections: TallyElection[] | undefined,
    selectedIds: string[] | undefined,
    ballotBoxes?: Record<string, IBallotBoxesSummary>
): string | undefined => {
    // With the seal at close, nothing can be selected while no election is
    // ready: say why rather than asking to select one.
    if (!selectedIds?.length && ballotBoxes && elections?.length) {
        const readiness = elections.map(({id}) => ballotBoxes[id]?.readiness)
        if (readiness.every((value) => value !== EBallotBoxesReadiness.READY)) {
            return readiness.every((value) => value === EBallotBoxesReadiness.UNAVAILABLE)
                ? "ballotBoxesUnavailable"
                : "sealBallotBoxes"
        }
    }
    if (!elections || !selectedIds?.length) return "selectElection"
    for (const id of selectedIds) {
        const election = elections.find((item) => item.id === id)
        if (!election?.status?.is_published) return "publishElection"
        const status = election.status
        const policy = status.allow_tally ?? EAllowTally.ALLOWED
        if (policy !== EAllowTally.ALLOWED) {
            if (policy !== EAllowTally.REQUIRES_VOTING_PERIOD_END) return "tallyDisallowed"
            const secondaryClosed = [
                status.kiosk_voting_status,
                status.early_voting_status,
                status.telephone_voting_status,
            ].every(
                (value) =>
                    !value || value === EVotingStatus.NOT_STARTED || value === EVotingStatus.CLOSED
            )
            if (status.voting_status !== EVotingStatus.CLOSED || !secondaryClosed) {
                return "endVoting"
            }
        }
        const readiness = ballotBoxes?.[id]?.readiness
        if (ballotBoxes && readiness !== EBallotBoxesReadiness.READY) {
            return readiness === EBallotBoxesReadiness.UNAVAILABLE
                ? "ballotBoxesUnavailable"
                : "sealBallotBoxes"
        }
    }
    return undefined
}
