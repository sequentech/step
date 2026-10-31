// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    ballotBoxSealPolicyLock,
    ESealPolicyLock,
    hasVotingEverOpened,
    votingStarted,
} from "./ElectionStatus"
jest.mock("@sequentech/ui-core", () => require("../../../ui-core/src/types/CoreTypes"))

const NULL_DATES = {
    first_started_at: null,
    last_started_at: null,
    first_paused_at: null,
    last_paused_at: null,
    first_stopped_at: null,
    last_stopped_at: null,
}

/** `serde_json::to_value(ElectionStatus::default())`, as windmill writes it on insert. */
const defaultElectionStatus = () => ({
    voting_status: "NOT_STARTED",
    kiosk_voting_status: "NOT_STARTED",
    early_voting_status: "NOT_STARTED",
    telephone_voting_status: "NOT_STARTED",
    voting_period_dates: {...NULL_DATES},
    kiosk_voting_period_dates: {...NULL_DATES},
    early_voting_period_dates: {...NULL_DATES},
    telephone_voting_period_dates: {...NULL_DATES},
})

describe("votingStarted mirrors trusted_voting_started", () => {
    it("does not count the default status, whose dates are all null", () => {
        expect(votingStarted(defaultElectionStatus())).toBe(false)
    })
    it("treats a null or missing channel status as NOT_STARTED", () => {
        expect(votingStarted({voting_status: null})).toBe(false)
        expect(votingStarted({})).toBe(false)
        expect(votingStarted(null)).toBe(false)
        expect(votingStarted("OPEN")).toBe(false)
    })
    it.each([
        "voting_status",
        "kiosk_voting_status",
        "early_voting_status",
        "telephone_voting_status",
    ])("counts %s other than NOT_STARTED", (channel) => {
        for (const value of ["OPEN", "PAUSED", "CLOSED"]) {
            expect(votingStarted({...defaultElectionStatus(), [channel]: value})).toBe(true)
        }
    })
    it.each([
        "voting_period_dates",
        "kiosk_voting_period_dates",
        "early_voting_period_dates",
        "telephone_voting_period_dates",
    ])("counts a non-null date in %s", (dates) => {
        expect(
            votingStarted({
                ...defaultElectionStatus(),
                [dates]: {...NULL_DATES, first_started_at: "2028-03-13T08:00:00Z"},
            })
        ).toBe(true)
    })
    it("ignores keys of other channels and empty or null date objects", () => {
        expect(votingStarted({other_voting_status: "OPEN"})).toBe(false)
        expect(votingStarted({voting_period_dates: {}})).toBe(false)
        expect(votingStarted({voting_period_dates: null})).toBe(false)
    })
})

describe("hasVotingEverOpened", () => {
    it("is false for a new event and its new elections", () => {
        expect(
            hasVotingEverOpened([{status: defaultElectionStatus()}], {
                status: {voting_status: "NOT_STARTED"},
            })
        ).toBe(false)
        expect(hasVotingEverOpened(undefined, undefined)).toBe(false)
    })
    it("is true once the event or any election has opened", () => {
        expect(hasVotingEverOpened([], {status: {voting_status: "OPEN"}})).toBe(true)
        expect(
            hasVotingEverOpened(
                [
                    {status: defaultElectionStatus()},
                    {status: {...defaultElectionStatus(), kiosk_voting_status: "CLOSED"}},
                ],
                {status: {}}
            )
        ).toBe(true)
    })
})

describe("ballotBoxSealPolicyLock", () => {
    const opened = {id: "e2", status: {voting_status: "OPEN"}}
    const fresh = {id: "e1", status: defaultElectionStatus()}
    it("is open before voting opens", () => {
        expect(ballotBoxSealPolicyLock([fresh], {status: {}})).toEqual({
            state: ESealPolicyLock.OPEN,
            elections: [],
        })
    })
    it("names the elections where voting opened", () => {
        expect(ballotBoxSealPolicyLock([fresh, opened], {status: {}})).toEqual({
            state: ESealPolicyLock.LOCKED,
            elections: [opened],
        })
        expect(ballotBoxSealPolicyLock([fresh], {status: {voting_status: "OPEN"}}).state).toBe(
            ESealPolicyLock.LOCKED
        )
    })
    it("says loading, or unknown when the elections can't be read", () => {
        expect(ballotBoxSealPolicyLock(undefined, undefined, true).state).toBe(
            ESealPolicyLock.LOADING
        )
        expect(ballotBoxSealPolicyLock(undefined, undefined, false).state).toBe(
            ESealPolicyLock.UNKNOWN
        )
    })
})
