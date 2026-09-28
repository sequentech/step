// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    check_voting_error_dialog_bool,
    check_voting_not_allowed_next_bool,
    EElectionEventContestEncryptionPolicy,
    type BallotSelection,
    type IAuditableBallot,
    type IAuditableMultiBallot,
    type IAuditableSingleBallot,
} from "@sequentech/ui-core"
import {provideBallotService, type IBallotService} from "../services/BallotService"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

export enum PipelineStep {
    INTERPRET = "interpret",
    CHECK = "check",
    ENCRYPT = "encrypt",
    HASH = "hash",
    DECODE = "decode",
}

export enum PipelineStatus {
    PASSED = "passed",
    FAILED = "failed",
    SKIPPED = "skipped",
}

export interface ContestValidation {
    contestId: string
    errors: string[]
    alerts: string[]
}

export interface SelectionValidation {
    contests: ContestValidation[]
    /** The voting screen would block Next. */
    nextBlocked: boolean
    /** The voting screen would ask for confirmation before review. */
    confirmBeforeReview: boolean
}

export interface PipelineStepResult {
    step: PipelineStep
    status: PipelineStatus
    detail: string
    durationMs: number
}

export interface PipelineReport {
    steps: PipelineStepResult[]
    validation?: SelectionValidation
    ballotHash?: string
    /** Choices recovered from the encrypted ballot, per contest and candidate. */
    decoded?: BallotSelection
}

export type PipelineRun = (ballotStyle: IBallotStyle, selection: BallotSelection) => PipelineReport

/** The voting screen's Next and warning dialog checks, both in sequent-core. */
export interface VotingChecks {
    nextBlocked: typeof check_voting_not_allowed_next_bool
    confirmBeforeReview: typeof check_voting_error_dialog_bool
}

const votingChecks = (): VotingChecks => ({
    nextBlocked: check_voting_not_allowed_next_bool,
    confirmBeforeReview: check_voting_error_dialog_bool,
})

export const isMultiContestStyle = ({ballot_eml}: IBallotStyle) =>
    ballot_eml.election_event_presentation?.contest_encryption_policy ===
    EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS

const byId = (a: {id: string}, b: {id: string}) => a.id.localeCompare(b.id)

const markedChoices = (selection: BallotSelection) =>
    selection
        .map(({contest_id, choices}) => ({
            id: contest_id,
            marks: choices
                .filter(({selected}) => selected > -1)
                .map(({id, selected}) => ({id, selected}))
                .sort(byId),
        }))
        .sort(byId)

const describeError = (error: unknown) =>
    error instanceof Error
        ? error.message
        : typeof error === "string"
          ? error
          : JSON.stringify(error)

function interpretSelection(
    ballotStyle: IBallotStyle,
    selection: BallotSelection,
    service: IBallotService
): BallotSelection {
    return isMultiContestStyle(ballotStyle)
        ? service.interpretMultiContestSelection(selection, ballotStyle.ballot_eml)
        : service.interpretContestSelection(selection, ballotStyle.ballot_eml)
}

const contestValidation = (interpreted: BallotSelection): ContestValidation[] =>
    interpreted.map(({contest_id, invalid_errors, invalid_alerts}) => ({
        contestId: contest_id,
        errors: invalid_errors.map(({message}) => message ?? ""),
        alerts: invalid_alerts.map(({message}) => message ?? ""),
    }))

function screenChecks(
    ballotStyle: IBallotStyle,
    interpreted: BallotSelection,
    checks: VotingChecks
) {
    const decoded = Object.fromEntries(interpreted.map((contest) => [contest.contest_id, contest]))
    const {contests} = ballotStyle.ballot_eml
    return {
        nextBlocked: checks.nextBlocked(contests, decoded),
        confirmBeforeReview: checks.confirmBeforeReview(contests, decoded),
    }
}

/** The voting screen's validation of a selection, without encrypting it. */
export function validateSelection(
    ballotStyle: IBallotStyle,
    selection: BallotSelection,
    service: IBallotService = provideBallotService(),
    checks: VotingChecks = votingChecks()
): SelectionValidation {
    const interpreted = interpretSelection(ballotStyle, selection, service)
    return {
        contests: contestValidation(interpreted),
        ...screenChecks(ballotStyle, interpreted, checks),
    }
}

/**
 * Runs the voting screen's ballot operations on a selection: the re-encoding check with its
 * errors, the Next and warning checks, then encryption, the ballot hash and decoding, which
 * must return the same marks. The first failing step skips the rest.
 */
export function runBallotPipeline(
    ballotStyle: IBallotStyle,
    selection: BallotSelection,
    service: IBallotService = provideBallotService(),
    checks: VotingChecks = votingChecks()
): PipelineReport {
    const report: PipelineReport = {steps: []}
    const multi = isMultiContestStyle(ballotStyle)
    const election = ballotStyle.ballot_eml
    let interpreted: BallotSelection = []
    let ballot: IAuditableBallot | undefined

    const steps: [PipelineStep, () => string][] = [
        [
            PipelineStep.INTERPRET,
            () => {
                interpreted = interpretSelection(ballotStyle, selection, service)
                const contests = contestValidation(interpreted)
                report.validation = {contests, nextBlocked: false, confirmBeforeReview: false}
                const errors = contests.reduce((sum, {errors}) => sum + errors.length, 0)
                const alerts = contests.reduce((sum, {alerts}) => sum + alerts.length, 0)
                return `${errors} errors and ${alerts} alerts in ${contests.length} contests`
            },
        ],
        [
            PipelineStep.CHECK,
            () => {
                const {nextBlocked, confirmBeforeReview} = screenChecks(
                    ballotStyle,
                    interpreted,
                    checks
                )
                if (report.validation)
                    Object.assign(report.validation, {nextBlocked, confirmBeforeReview})
                return `Next ${nextBlocked ? "blocked" : "allowed"}, ${
                    confirmBeforeReview ? "with" : "without"
                } a confirmation dialog`
            },
        ],
        [
            PipelineStep.ENCRYPT,
            () => {
                ballot = multi
                    ? service.encryptMultiBallotSelection(selection, election)
                    : service.encryptBallotSelection(selection, election)
                return `Auditable ballot version ${ballot.version}`
            },
        ],
        [
            PipelineStep.HASH,
            () => {
                report.ballotHash = multi
                    ? service.hashMultiBallot(ballot as IAuditableMultiBallot)
                    : service.hashBallot(ballot as IAuditableSingleBallot)
                return report.ballotHash
            },
        ],
        [
            PipelineStep.DECODE,
            () => {
                const decoded = multi
                    ? service.decodeAuditableMultiBallot(ballot as IAuditableMultiBallot)
                    : service.decodeAuditableBallot(ballot as IAuditableSingleBallot)
                report.decoded = decoded ?? []
                if (
                    JSON.stringify(markedChoices(report.decoded)) !==
                    JSON.stringify(markedChoices(selection))
                )
                    throw new Error("The decoded ballot does not match the selection")
                return "The decoded ballot matches the selection"
            },
        ],
    ]

    let failed = false
    for (const [step, run] of steps) {
        if (failed) {
            report.steps.push({step, status: PipelineStatus.SKIPPED, detail: "", durationMs: 0})
            continue
        }
        const started = performance.now()
        try {
            const detail = run()
            report.steps.push({
                step,
                status: PipelineStatus.PASSED,
                detail,
                durationMs: performance.now() - started,
            })
        } catch (error) {
            failed = true
            report.steps.push({
                step,
                status: PipelineStatus.FAILED,
                detail: describeError(error),
                durationMs: performance.now() - started,
            })
        }
    }
    return report
}
