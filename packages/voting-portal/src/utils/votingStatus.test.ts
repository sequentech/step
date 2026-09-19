// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EVotingStatus, IElectionEvent} from "@sequentech/ui-core"
import {isElectionEventVotingClosed, isVotingClosedForChannels} from "./votingStatus"

const NOT_STARTED = EVotingStatus.NOT_STARTED
const OPEN = EVotingStatus.OPEN
const PAUSED = EVotingStatus.PAUSED
const CLOSED = EVotingStatus.CLOSED

const event = (
    voting: EVotingStatus,
    kiosk: EVotingStatus = NOT_STARTED,
    early: EVotingStatus = NOT_STARTED,
    telephone: EVotingStatus = NOT_STARTED
): IElectionEvent =>
    ({
        status: {
            voting_status: voting,
            kiosk_voting_status: kiosk,
            early_voting_status: early,
            telephone_voting_status: telephone,
        },
    }) as unknown as IElectionEvent

describe("isVotingClosedForChannels", () => {
    it("is false when nothing ever ran", () => {
        expect(isVotingClosedForChannels([NOT_STARTED, NOT_STARTED, NOT_STARTED])).toBe(false)
        expect(isVotingClosedForChannels([])).toBe(false)
        expect(isVotingClosedForChannels([undefined, undefined])).toBe(false)
    })

    it("is false while a channel is open or paused", () => {
        expect(isVotingClosedForChannels([OPEN])).toBe(false)
        expect(isVotingClosedForChannels([PAUSED])).toBe(false)
        expect(isVotingClosedForChannels([CLOSED, OPEN])).toBe(false)
        expect(isVotingClosedForChannels([CLOSED, PAUSED])).toBe(false)
    })

    it("ignores channels that never ran", () => {
        expect(isVotingClosedForChannels([CLOSED, NOT_STARTED, NOT_STARTED])).toBe(true)
        expect(isVotingClosedForChannels([NOT_STARTED, CLOSED])).toBe(true)
    })

    it("requires every channel that ran to be closed", () => {
        expect(isVotingClosedForChannels([CLOSED, CLOSED])).toBe(true)
        expect(isVotingClosedForChannels([CLOSED, CLOSED, NOT_STARTED])).toBe(true)
    })
})

describe("isElectionEventVotingClosed", () => {
    // The regression this guards: an online-only event created through the admin
    // UI keeps the other three channels at NOT_STARTED forever, so a predicate
    // demanding all four be CLOSED would hide the results link permanently.
    it("is true for a closed online-only event", () => {
        expect(isElectionEventVotingClosed(event(CLOSED))).toBe(true)
    })

    it("is false for an event that has not started, which is META-12780", () => {
        expect(isElectionEventVotingClosed(event(NOT_STARTED))).toBe(false)
    })

    it("is false while voting is open or paused", () => {
        expect(isElectionEventVotingClosed(event(OPEN))).toBe(false)
        expect(isElectionEventVotingClosed(event(PAUSED))).toBe(false)
    })

    it("is true for a kiosk-only event whose kiosk channel closed", () => {
        expect(isElectionEventVotingClosed(event(NOT_STARTED, CLOSED))).toBe(true)
    })

    it("is false when early voting is still open after online closed", () => {
        expect(isElectionEventVotingClosed(event(CLOSED, NOT_STARTED, OPEN))).toBe(false)
    })

    it("is true when every channel that ran has closed", () => {
        expect(isElectionEventVotingClosed(event(CLOSED, CLOSED, CLOSED, CLOSED))).toBe(true)
    })

    it("is false for a missing event or missing status", () => {
        expect(isElectionEventVotingClosed(undefined)).toBe(false)
        expect(isElectionEventVotingClosed({} as IElectionEvent)).toBe(false)
    })
})
