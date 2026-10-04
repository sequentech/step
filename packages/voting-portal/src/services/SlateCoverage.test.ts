// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ESlateCoverageKind, IContest, ISlateCoverage} from "@sequentech/ui-core"
import {
    ESlateCoverageLabel,
    ESlateOfficesLayout,
    getCoveredContests,
    getSlateCoverageLabel,
    getSlateOffices,
    getUncoveredContests,
    isContestCoveredBySlate,
} from "./SlateCoverage"

const SINGLE_SEAT_OFFICES = ["president", "vp", "treasurer", "secretary"]
const TRUSTEES = "trustees"

const contest = (id: string, maxVotes: number, candidateIds: Array<string>): IContest =>
    ({
        id,
        max_votes: maxVotes,
        candidates: candidateIds.map((candidateId) => ({id: candidateId, contest_id: id})),
    }) as IContest

const contests: Array<IContest> = [
    ...SINGLE_SEAT_OFFICES.map((office) => contest(office, 1, [`f-${office}`, `i-${office}`])),
    contest(TRUSTEES, 3, ["f-t1", "f-t2", "f-t3", "v-t1", "v-t2", "v-t3"]),
]

const fullSlate: ISlateCoverage = {
    slate_id: "forward",
    kind: ESlateCoverageKind.COMPLETE,
    covered: [
        ...SINGLE_SEAT_OFFICES.map((office) => ({
            contest_id: office,
            candidate_ids: [`f-${office}`],
            seats: 1,
        })),
        {contest_id: TRUSTEES, candidate_ids: ["f-t1", "f-t2", "f-t3"], seats: 3},
    ],
    uncovered_contest_ids: [],
    members: 7,
    seats: 7,
}

const trusteesOnly: ISlateCoverage = {
    slate_id: "voices",
    kind: ESlateCoverageKind.PARTIAL,
    covered: [{contest_id: TRUSTEES, candidate_ids: ["v-t3", "v-t1"], seats: 3}],
    uncovered_contest_ids: SINGLE_SEAT_OFFICES,
    members: 2,
    seats: 7,
}

const oneTrustee: ISlateCoverage = {
    ...fullSlate,
    slate_id: "short",
    kind: ESlateCoverageKind.PARTIAL,
    covered: fullSlate.covered.map((covered) =>
        covered.contest_id === TRUSTEES ? {...covered, candidate_ids: ["f-t1"]} : covered
    ),
    members: 5,
}

describe("slate coverage label", () => {
    it("calls a slate that fills every seat a full slate", () => {
        expect(getSlateCoverageLabel(fullSlate, contests)).toEqual({
            label: ESlateCoverageLabel.FULL,
        })
    })

    it("names the only office of a slate that leaves the others", () => {
        const label = getSlateCoverageLabel(trusteesOnly, contests)

        expect(label.label).toBe(ESlateCoverageLabel.SINGLE_CONTEST)
        expect(label.contest?.id).toBe(TRUSTEES)
    })

    it("calls a slate in every office with fewer candidates than seats partial", () => {
        expect(getSlateCoverageLabel(oneTrustee, contests)).toEqual({
            label: ESlateCoverageLabel.PARTIAL,
        })
    })

    it("calls a slate in a single-office ballot with a free seat partial", () => {
        const onlyTrustees = contests.filter((entry) => entry.id === TRUSTEES)
        const coverage = {...trusteesOnly, uncovered_contest_ids: [], seats: 3}

        expect(getSlateCoverageLabel(coverage, onlyTrustees)).toEqual({
            label: ESlateCoverageLabel.PARTIAL,
        })
    })

    it("falls back to partial when the only office is not in the ballot", () => {
        expect(getSlateCoverageLabel(trusteesOnly, [])).toEqual({
            label: ESlateCoverageLabel.PARTIAL,
        })
    })
})

describe("slate offices", () => {
    it("aligns every office and leaves the uncovered ones without candidates", () => {
        const offices = getSlateOffices(trusteesOnly, contests, ESlateOfficesLayout.ALIGNED)

        expect(offices.map((office) => office.contest.id)).toEqual([
            ...SINGLE_SEAT_OFFICES,
            TRUSTEES,
        ])
        expect(offices.slice(0, 4).every((office) => office.candidates.length === 0)).toBe(true)
        expect(offices[4].candidates.map((candidate) => candidate.id)).toEqual(["v-t3", "v-t1"])
    })

    it("omits the uncovered offices when stacked", () => {
        const offices = getSlateOffices(trusteesOnly, contests, ESlateOfficesLayout.STACKED)

        expect(offices.map((office) => office.contest.id)).toEqual([TRUSTEES])
    })

    it("lists the same offices for a full slate in both layouts", () => {
        const aligned = getSlateOffices(fullSlate, contests, ESlateOfficesLayout.ALIGNED)
        const stacked = getSlateOffices(fullSlate, contests, ESlateOfficesLayout.STACKED)

        expect(aligned).toEqual(stacked)
        expect(aligned).toHaveLength(5)
    })

    it("leaves out a contest that is neither covered nor uncovered", () => {
        const acclaimed = contest("auditor", 1, ["a-auditor"])
        const offices = getSlateOffices(
            trusteesOnly,
            [acclaimed, ...contests],
            ESlateOfficesLayout.ALIGNED
        )

        expect(offices.map((office) => office.contest.id)).not.toContain("auditor")
    })

    it("skips a covered candidate the contest does not have", () => {
        const coverage = {
            ...trusteesOnly,
            covered: [{contest_id: TRUSTEES, candidate_ids: ["v-t1", "gone"], seats: 3}],
        }
        const [office] = getSlateOffices(coverage, contests, ESlateOfficesLayout.STACKED)

        expect(office.candidates.map((candidate) => candidate.id)).toEqual(["v-t1"])
    })
})

describe("covered and uncovered contests", () => {
    it("splits the ballot between what a partial slate covers and what it leaves", () => {
        expect(getCoveredContests(trusteesOnly, contests).map((entry) => entry.id)).toEqual([
            TRUSTEES,
        ])
        expect(getUncoveredContests(trusteesOnly, contests).map((entry) => entry.id)).toEqual(
            SINGLE_SEAT_OFFICES
        )
    })

    it("leaves nothing uncovered for a full slate", () => {
        expect(getUncoveredContests(fullSlate, contests)).toEqual([])
    })

    it("tells whether a contest is covered", () => {
        expect(isContestCoveredBySlate(trusteesOnly, TRUSTEES)).toBe(true)
        expect(isContestCoveredBySlate(trusteesOnly, "president")).toBe(false)
    })
})
