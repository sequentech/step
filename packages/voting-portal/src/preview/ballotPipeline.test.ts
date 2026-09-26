// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EElectionEventContestEncryptionPolicy,
    EInvalidPlaintextErrorType,
    type BallotSelection,
    type IAuditableMultiBallot,
    type IInvalidPlaintextError,
    type IAuditableSingleBallot,
    type IBallotStyle as BallotDefinition,
} from "@sequentech/ui-core"
import {electionFixture, IDS} from "@sequentech/ui-test-kit/fixtures"
import type {IBallotService} from "../services/BallotService"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {PipelineStatus, PipelineStep, runBallotPipeline, type VotingChecks} from "./ballotPipeline"

const ballotStyle = (multiContest = false): IBallotStyle => {
    const ballot = electionFixture().ballot as unknown as BallotDefinition
    if (multiContest)
        ballot.election_event_presentation = {
            ...ballot.election_event_presentation!,
            contest_encryption_policy: EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS,
        }
    return {
        id: IDS.style,
        election_id: IDS.election,
        election_event_id: IDS.event,
        tenant_id: IDS.tenant,
        ballot_eml: ballot,
        created_at: "",
        last_updated_at: "",
    }
}

const selection = (aliceMark = 0): BallotSelection => [
    {
        contest_id: IDS.contest,
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: [
            {id: IDS.bob, selected: -1},
            {id: IDS.alice, selected: aliceMark},
        ],
    },
]

// sequent-core declares its own copy of the error type enum without exporting it.
const overVote = {
    error_type: EInvalidPlaintextErrorType.Implicit,
    message_map: {},
} as unknown as IInvalidPlaintextError

function fakeService(decoded = selection()) {
    const ballot = {version: 1} as IAuditableSingleBallot
    const service = {
        interpretContestSelection: jest.fn((input: BallotSelection) =>
            input.map((contest) => ({
                ...contest,
                invalid_errors: [{...overVote, message: "errors.implicit.overVote"}],
                invalid_alerts: [
                    {...overVote, message: "errors.implicit.underVote"},
                    {...overVote, message: "errors.implicit.blankVote"},
                ],
            }))
        ),
        interpretMultiContestSelection: jest.fn((input: BallotSelection) => input),
        encryptBallotSelection: jest.fn(() => ballot),
        encryptMultiBallotSelection: jest.fn(() => ({version: 2}) as IAuditableMultiBallot),
        hashBallot: jest.fn(() => "a".repeat(64)),
        hashMultiBallot: jest.fn(() => "b".repeat(64)),
        decodeAuditableBallot: jest.fn(() => decoded),
        decodeAuditableMultiBallot: jest.fn(() => decoded),
    }
    return {service, ballot}
}

const checks = (nextBlocked: boolean, confirmBeforeReview: boolean) => ({
    nextBlocked: jest.fn(() => nextBlocked),
    confirmBeforeReview: jest.fn(() => confirmBeforeReview),
})

const run = (
    service: Partial<IBallotService>,
    votingChecks: VotingChecks = checks(false, true),
    style = ballotStyle()
) => runBallotPipeline(style, selection(), service as IBallotService, votingChecks)

test("a single-contest selection passes every step with its results", () => {
    const {service, ballot} = fakeService()
    const votingChecks = checks(true, false)
    const style = ballotStyle()
    const report = run(service, votingChecks, style)
    expect(report.steps.map(({step, status, detail}) => [step, status, detail])).toEqual([
        [PipelineStep.INTERPRET, PipelineStatus.PASSED, "1 errors and 2 alerts in 1 contests"],
        [PipelineStep.CHECK, PipelineStatus.PASSED, "Next blocked, without a confirmation dialog"],
        [PipelineStep.ENCRYPT, PipelineStatus.PASSED, "Auditable ballot version 1"],
        [PipelineStep.HASH, PipelineStatus.PASSED, "a".repeat(64)],
        [PipelineStep.DECODE, PipelineStatus.PASSED, "The decoded ballot matches the selection"],
    ])
    expect(report.validation).toEqual({
        contests: [
            {
                contestId: IDS.contest,
                errors: ["errors.implicit.overVote"],
                alerts: ["errors.implicit.underVote", "errors.implicit.blankVote"],
            },
        ],
        nextBlocked: true,
        confirmBeforeReview: false,
    })
    expect(report.ballotHash).toBe("a".repeat(64))
    expect(report.decoded).toEqual(selection())
    // The checks receive the interpreted contests keyed by contest.
    const [[contests, decoded]] = votingChecks.nextBlocked.mock.calls as unknown as [
        [unknown, Record<string, {invalid_errors: unknown[]}>],
    ]
    expect(contests).toBe(style.ballot_eml.contests)
    expect(Object.keys(decoded)).toEqual([IDS.contest])
    expect(decoded[IDS.contest].invalid_errors).toHaveLength(1)
    expect(service.hashBallot).toHaveBeenCalledWith(ballot)
    expect(service.encryptMultiBallotSelection).not.toHaveBeenCalled()
})

test("a multiple-contest ballot uses the multi-contest operations", () => {
    const {service} = fakeService()
    const report = run(service, checks(false, true), ballotStyle(true))
    expect(report.steps.every(({status}) => status === PipelineStatus.PASSED)).toBe(true)
    expect(report.ballotHash).toBe("b".repeat(64))
    expect(service.interpretMultiContestSelection).toHaveBeenCalled()
    expect(service.encryptMultiBallotSelection).toHaveBeenCalled()
    expect(service.decodeAuditableMultiBallot).toHaveBeenCalled()
    expect(service.encryptBallotSelection).not.toHaveBeenCalled()
    expect(report.steps[1].detail).toBe("Next allowed, with a confirmation dialog")
})

test("the round trip compares marks regardless of candidate order", () => {
    const [contest] = selection()
    const {service} = fakeService([{...contest, choices: [...contest.choices].reverse()}])
    expect(run(service).steps[4].status).toBe(PipelineStatus.PASSED)
})

test("a decoded ballot with other marks fails the round trip", () => {
    const {service} = fakeService(selection(1))
    const report = run(service)
    expect(report.steps[4]).toMatchObject({
        step: PipelineStep.DECODE,
        status: PipelineStatus.FAILED,
        detail: "The decoded ballot does not match the selection",
    })
    expect(report.decoded).toEqual(selection(1))
})

test("the first failing step reports its error and skips the rest", () => {
    const {service} = fakeService()
    service.encryptBallotSelection.mockImplementation(() => {
        throw "unsupported contest encryption policy"
    })
    const report = run(service)
    expect(report.steps.map(({step, status, detail}) => [step, status, detail])).toEqual([
        [PipelineStep.INTERPRET, PipelineStatus.PASSED, "1 errors and 2 alerts in 1 contests"],
        [PipelineStep.CHECK, PipelineStatus.PASSED, "Next allowed, with a confirmation dialog"],
        [PipelineStep.ENCRYPT, PipelineStatus.FAILED, "unsupported contest encryption policy"],
        [PipelineStep.HASH, PipelineStatus.SKIPPED, ""],
        [PipelineStep.DECODE, PipelineStatus.SKIPPED, ""],
    ])
    expect(service.hashBallot).not.toHaveBeenCalled()
    expect(report.ballotHash).toBeUndefined()
})

test("errors of every shape are described", () => {
    for (const [thrown, detail] of [
        [new Error("interpretation failed"), "interpretation failed"],
        [{code: 7}, '{"code":7}'],
    ] as const) {
        const {service} = fakeService()
        service.interpretContestSelection.mockImplementation(() => {
            throw thrown
        })
        const report = run(service)
        expect(report.steps[0]).toMatchObject({status: PipelineStatus.FAILED, detail})
        expect(report.validation).toBeUndefined()
        expect(report.steps.slice(1).map(({status}) => status)).toEqual(
            Array(4).fill(PipelineStatus.SKIPPED)
        )
    }
})
