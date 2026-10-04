// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EReceiptsPolicy, EVotingStatus} from "@sequentech/ui-core"
import {
    areReceiptsSignedByBallotBox,
    hasVotingStarted,
    parseChecksAvailableUntil,
    receiptsHelperTextKey,
} from "./BallotReceipts"

// The portal's jest resolves only ui-core's voting channels; these are the
// wire values of the enums in ui-core.
jest.mock("@sequentech/ui-core", () => ({
    EReceiptsPolicy: {DISABLED: "disabled", SIGNED_BY_BALLOT_BOX: "signed-by-ballot-box"},
    EVotingStatus: {NOT_STARTED: "NOT_STARTED", OPEN: "OPEN", PAUSED: "PAUSED", CLOSED: "CLOSED"},
}))

describe("areReceiptsSignedByBallotBox", () => {
    it("is off for events saved before the setting existed", () => {
        expect(areReceiptsSignedByBallotBox()).toBe(false)
        expect(areReceiptsSignedByBallotBox({})).toBe(false)
        expect(areReceiptsSignedByBallotBox({receipts: {}})).toBe(false)
    })

    it("follows the receipts policy", () => {
        expect(areReceiptsSignedByBallotBox({receipts: {policy: EReceiptsPolicy.DISABLED}})).toBe(
            false
        )
        expect(
            areReceiptsSignedByBallotBox({
                receipts: {policy: EReceiptsPolicy.SIGNED_BY_BALLOT_BOX},
            })
        ).toBe(true)
    })
})

describe("hasVotingStarted", () => {
    it("has not started without a status", () => {
        expect(hasVotingStarted()).toBe(false)
        expect(hasVotingStarted(null)).toBe(false)
        expect(hasVotingStarted({})).toBe(false)
    })

    it("has not started while every channel is yet to open", () => {
        expect(
            hasVotingStarted({
                voting_status: EVotingStatus.NOT_STARTED,
                kiosk_voting_status: EVotingStatus.NOT_STARTED,
                early_voting_status: EVotingStatus.NOT_STARTED,
                telephone_voting_status: EVotingStatus.NOT_STARTED,
            })
        ).toBe(false)
    })

    it.each([
        ["voting_status", EVotingStatus.OPEN],
        ["kiosk_voting_status", EVotingStatus.PAUSED],
        ["early_voting_status", EVotingStatus.CLOSED],
        ["telephone_voting_status", EVotingStatus.OPEN],
    ])("has started once %s is %s", (channel, status) => {
        expect(
            hasVotingStarted({voting_status: EVotingStatus.NOT_STARTED, [channel]: status})
        ).toBe(true)
    })
})

describe("receiptsHelperTextKey", () => {
    it("explains the setting until voting starts, and the lock afterwards", () => {
        expect(receiptsHelperTextKey(false)).toBe(
            "electionEventScreen.field.receiptsPolicy.helperText"
        )
        expect(receiptsHelperTextKey(true)).toBe(
            "electionEventScreen.field.receiptsPolicy.lockedHelperText"
        )
    })
})

describe("parseChecksAvailableUntil", () => {
    it("stores the chosen local time as an RFC 3339 instant", () => {
        expect(parseChecksAvailableUntil("2028-06-07T23:59:00+08:00")).toBe(
            "2028-06-07T15:59:00.000Z"
        )
    })

    it("stores nothing for an empty or unreadable date", () => {
        expect(parseChecksAvailableUntil("")).toBeNull()
        expect(parseChecksAvailableUntil(null)).toBeNull()
        expect(parseChecksAvailableUntil(undefined)).toBeNull()
        expect(parseChecksAvailableUntil("not a date")).toBeNull()
    })
})
