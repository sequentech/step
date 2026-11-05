// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EBallotBoxesReadiness,
    getTallyDisabledReason,
    summarizeBallotBoxes,
    summarizeBallotBoxesByElection,
} from "./tallyEligibility"
import {
    EBallotBoxClosedByKind,
    EBallotBoxSealStatus,
    type IBallotBoxSeal,
} from "@/types/ballotBoxSeal"
import {EAllowTally, EVotingStatus} from "@sequentech/ui-core"
jest.mock("@sequentech/ui-core", () => require("../../../ui-core/src/types/CoreTypes"))
const election = (id: string, voting_status = EVotingStatus.CLOSED) => ({
    id,
    status: {
        is_published: true,
        voting_status,
        allow_tally: EAllowTally.REQUIRES_VOTING_PERIOD_END,
    },
})

it("only checks selected elections, regardless of the other election's open state", () => {
    const elections = [election("closed"), election("open", EVotingStatus.OPEN)]
    expect(getTallyDisabledReason(elections, ["closed"])).toBeUndefined()
    expect(getTallyDisabledReason(elections, ["closed", "open"])).toBe("endVoting")
})
it.each(["voting_status", "kiosk_voting_status", "early_voting_status", "telephone_voting_status"])(
    "blocks an active %s",
    (channel) => {
        for (const status of [EVotingStatus.OPEN, EVotingStatus.PAUSED]) {
            const selected = election("selected")
            expect(
                getTallyDisabledReason(
                    [{...selected, status: {...selected.status, [channel]: status}}],
                    ["selected"]
                )
            ).toBe("endVoting")
        }
    }
)
it("requires a selection and publication and honors explicit policy", () => {
    expect(getTallyDisabledReason([], [])).toBe("selectElection")
    expect(getTallyDisabledReason(undefined, ["selected"])).toBe("selectElection")
    expect(getTallyDisabledReason([], ["missing"])).toBe("publishElection")
    const selected = election("selected", EVotingStatus.OPEN)
    expect(
        getTallyDisabledReason(
            [{...selected, status: {...selected.status, allow_tally: EAllowTally.ALLOWED}}],
            ["selected"]
        )
    ).toBeUndefined()
    expect(
        getTallyDisabledReason(
            [{...selected, status: {...selected.status, allow_tally: EAllowTally.DISALLOWED}}],
            ["selected"]
        )
    ).toBe("tallyDisallowed")
})

const seal = (
    status: EBallotBoxSealStatus,
    area = "area",
    election_id = "closed",
    grace_deadline = "2028-03-13T17:15:00Z"
): IBallotBoxSeal => ({
    id: `${election_id}-${area}`,
    election_id,
    area_id: area,
    status,
    closed_at: "2028-03-13T17:00:00Z",
    grace_deadline,
    closed_by: {kind: EBallotBoxClosedByKind.SCHEDULED},
})

describe("summarizeBallotBoxes (VOTE-FREEZE)", () => {
    it("is ready only when every ballot box is on the bulletin board", () => {
        expect(
            summarizeBallotBoxes([
                seal(EBallotBoxSealStatus.PUBLISHED, "a"),
                seal(EBallotBoxSealStatus.PUBLISHED, "b"),
            ])
        ).toEqual({readiness: EBallotBoxesReadiness.READY, total: 2, sealed: 2, published: 2})
        expect(
            summarizeBallotBoxes([
                seal(EBallotBoxSealStatus.PUBLISHED, "a"),
                seal(EBallotBoxSealStatus.SEALED, "b"),
            ])
        ).toEqual({readiness: EBallotBoxesReadiness.PUBLISHING, total: 2, sealed: 2, published: 1})
    })
    it("is sealing at the latest deadline while a box is pending", () => {
        expect(
            summarizeBallotBoxes([
                seal(EBallotBoxSealStatus.PENDING, "a", "closed", "2028-03-13T17:15:00Z"),
                seal(EBallotBoxSealStatus.PENDING, "b", "closed", "2028-03-13T17:20:00Z"),
                seal(EBallotBoxSealStatus.PUBLISHED, "c"),
            ])
        ).toMatchObject({
            readiness: EBallotBoxesReadiness.SEALING,
            deadline: "2028-03-13T17:20:00Z",
        })
    })
    it("is not sealed without rows, and an incident with a failed seal", () => {
        expect(summarizeBallotBoxes([]).readiness).toBe(EBallotBoxesReadiness.NOT_SEALED)
        expect(
            summarizeBallotBoxes([
                seal(EBallotBoxSealStatus.PUBLISHED, "a"),
                seal(EBallotBoxSealStatus.FAILED, "b"),
                seal(EBallotBoxSealStatus.PENDING, "c"),
            ]).readiness
        ).toBe(EBallotBoxesReadiness.FAILED)
    })
    it("is overdue once the latest deadline has passed", () => {
        const pending = [seal(EBallotBoxSealStatus.PENDING, "a", "closed", "2028-03-13T17:15:00Z")]
        expect(summarizeBallotBoxes(pending, new Date("2028-03-13T17:14:00Z")).readiness).toBe(
            EBallotBoxesReadiness.SEALING
        )
        expect(summarizeBallotBoxes(pending, new Date("2028-03-13T17:15:00Z")).readiness).toBe(
            EBallotBoxesReadiness.OVERDUE
        )
    })
})

describe("getTallyDisabledReason when nothing can be selected", () => {
    const elections = [election("closed")]
    const summary = (readiness: EBallotBoxesReadiness) => ({
        closed: {readiness, total: 1, sealed: 0, published: 0},
    })
    it("gives the seal reason, not 'select an election', while no election is ready", () => {
        for (const readiness of [
            EBallotBoxesReadiness.SEALING,
            EBallotBoxesReadiness.OVERDUE,
            EBallotBoxesReadiness.FAILED,
            EBallotBoxesReadiness.PUBLISHING,
        ]) {
            expect(getTallyDisabledReason(elections, [], summary(readiness))).toBe(
                "sealBallotBoxes"
            )
        }
        expect(
            getTallyDisabledReason(elections, [], summary(EBallotBoxesReadiness.UNAVAILABLE))
        ).toBe("ballotBoxesUnavailable")
    })
    it("asks to select an election when one is ready", () => {
        expect(getTallyDisabledReason(elections, [], summary(EBallotBoxesReadiness.READY))).toBe(
            "selectElection"
        )
        expect(getTallyDisabledReason(elections, [], undefined)).toBe("selectElection")
    })
})

describe("getTallyDisabledReason with the seal at close", () => {
    const elections = [election("closed")]
    it("keeps the rules as they were when the event does not seal", () => {
        expect(getTallyDisabledReason(elections, ["closed"], undefined)).toBeUndefined()
    })
    it("allows only elections whose ballot boxes are all published", () => {
        const ready = summarizeBallotBoxesByElection(
            ["closed"],
            [seal(EBallotBoxSealStatus.PUBLISHED)]
        )
        expect(getTallyDisabledReason(elections, ["closed"], ready)).toBeUndefined()
        for (const status of [
            EBallotBoxSealStatus.PENDING,
            EBallotBoxSealStatus.SEALED,
            EBallotBoxSealStatus.FAILED,
        ]) {
            const summary = summarizeBallotBoxesByElection(["closed"], [seal(status)])
            expect(getTallyDisabledReason(elections, ["closed"], summary)).toBe("sealBallotBoxes")
        }
        expect(getTallyDisabledReason(elections, ["closed"], {})).toBe("sealBallotBoxes")
    })
    it("says when the seals could not be read", () => {
        const unavailable = {
            closed: {
                readiness: EBallotBoxesReadiness.UNAVAILABLE,
                total: 0,
                sealed: 0,
                published: 0,
            },
        }
        expect(getTallyDisabledReason(elections, ["closed"], unavailable)).toBe(
            "ballotBoxesUnavailable"
        )
    })
    it("still asks to end voting first", () => {
        const open = [election("open", EVotingStatus.OPEN)]
        const ready = summarizeBallotBoxesByElection(
            ["open"],
            [seal(EBallotBoxSealStatus.PUBLISHED, "area", "open")]
        )
        expect(getTallyDisabledReason(open, ["open"], ready)).toBe("endVoting")
    })
})
