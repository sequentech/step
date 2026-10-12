// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EMobileCandidateLists} from "@sequentech/ui-core"

import {aSlateBallotStyle, PRESIDENT, SLATES, TRUSTEES} from "../testing/slateFixtures"
import {
    getSlateConfigurationProblem,
    getSlateName,
    resolveBallotStyleSlates,
    resolveSlates,
} from "./Slates"

describe("resolveSlates", () => {
    it("keeps the configured order of the slates", () => {
        const {slates} = resolveSlates(SLATES, [PRESIDENT, TRUSTEES])

        expect(slates.map((slate) => slate.id)).toEqual(["forward", "members", "voices"])
    })

    it("lists the members of each slate by contest, in ballot order", () => {
        const {slates} = resolveSlates(SLATES, [TRUSTEES, PRESIDENT])
        const [forward] = slates

        expect(forward.contests.map(({contest}) => contest.id)).toEqual(["trustees", "president"])
        expect(forward.contests[0].candidates.map((candidate) => candidate.name)).toEqual([
            "Rowan Scott",
            "Charlie Kim",
        ])
    })

    it("follows the configured position of the contests, not their order in the ballot style", () => {
        const president = {...PRESIDENT, presentation: {sort_order: 0}}
        const trustees = {...TRUSTEES, presentation: {sort_order: 1}}
        const ballot = resolveSlates(SLATES, [trustees, president])

        expect(ballot.contests.map(({id}) => id)).toEqual(["president", "trustees"])
        expect(ballot.slates[0].contests.map(({contest}) => contest.id)).toEqual([
            "president",
            "trustees",
        ])
    })

    it("lists the offices of a covered slate in the configured position of the contests", () => {
        const president = {...PRESIDENT, presentation: {sort_order: 0}}
        const trustees = {...TRUSTEES, presentation: {sort_order: 1}}
        const style = aSlateBallotStyle(JSON.stringify(SLATES))
        const ballot = resolveBallotStyleSlates({
            ...style.ballot_eml,
            contests: [trustees, president],
        })
        const voices = ballot?.slates.find((slate) => slate.id === "voices")

        expect(ballot?.contests.map(({id}) => id)).toEqual(["president", "trustees"])
        expect(voices?.offices?.map(({contest}) => contest.id)).toEqual(["president", "trustees"])
    })

    it("lists each slate's candidates in the order the ballot shows them", () => {
        const trustees = {
            ...TRUSTEES,
            presentation: {candidates_order: "custom"},
            candidates: TRUSTEES.candidates.map((candidate, index) => ({
                ...candidate,
                presentation: {sort_order: TRUSTEES.candidates.length - index},
            })),
        } as typeof TRUSTEES
        const {slates} = resolveSlates(SLATES, [PRESIDENT, trustees])

        expect(slates[0].contests[1].candidates.map(({id}) => id)).toEqual(["f-t2", "f-t1"])
    })

    it("lists a covered slate's candidates in the order the ballot shows them", () => {
        const trustees = {
            ...TRUSTEES,
            presentation: {candidates_order: "custom"},
            candidates: TRUSTEES.candidates.map((candidate, index) => ({
                ...candidate,
                presentation: {sort_order: TRUSTEES.candidates.length - index},
            })),
        } as typeof TRUSTEES
        const style = aSlateBallotStyle(JSON.stringify(SLATES))
        const ballot = resolveBallotStyleSlates({
            ...style.ballot_eml,
            contests: [PRESIDENT, trustees],
        })
        const forward = ballot?.slates.find((slate) => slate.id === "forward")

        expect(forward?.contests[1].candidates.map(({id}) => id)).toEqual(["f-t2", "f-t1"])
        expect(forward?.offices?.[1].candidates.map(({id}) => id)).toEqual(["f-t2", "f-t1"])
    })

    it("lists only the contests a slate has candidates in", () => {
        const {slates} = resolveSlates(SLATES, [PRESIDENT, TRUSTEES])
        const voices = slates[2]

        expect(voices.contests.map(({contest}) => contest.id)).toEqual(["trustees"])
    })

    it("lists the contests any slate has candidates in, in ballot order", () => {
        const secretary = {...PRESIDENT, id: "secretary", candidates: []}

        const {contests} = resolveSlates(SLATES, [TRUSTEES, secretary, PRESIDENT])

        expect(contests.map((contest) => contest.id)).toEqual(["trustees", "president"])
    })

    it("leaves out a slate with no candidate on this ballot", () => {
        const {slates} = resolveSlates(SLATES, [PRESIDENT])

        expect(slates.map((slate) => slate.id)).toEqual(["forward", "members"])
    })

    it("carries the names and the phone list mode of the configuration", () => {
        const resolved = resolveSlates(
            {...SLATES, mobile_candidate_lists: EMobileCandidateLists.EXPANDED},
            [PRESIDENT, TRUSTEES]
        )

        expect(resolved.mobileCandidateLists).toBe(EMobileCandidateLists.EXPANDED)
        expect(getSlateName(resolved.slates[0], "es")).toBe("Adelante Juntos")
    })
})

describe("resolveBallotStyleSlates", () => {
    it("is null for a ballot without slates", () => {
        expect(resolveBallotStyleSlates(aSlateBallotStyle().ballot_eml)).toBeNull()
    })

    it("reads the slates of the ballot style", () => {
        const ballotStyle = aSlateBallotStyle(JSON.stringify(SLATES))

        expect(resolveBallotStyleSlates(ballotStyle.ballot_eml)?.slates).toHaveLength(3)
    })
})

describe("getSlateConfigurationProblem", () => {
    it("finds no problem in a ballot without slates", () => {
        expect(getSlateConfigurationProblem(aSlateBallotStyle().ballot_eml)).toBeUndefined()
    })

    it("finds no problem in a sound configuration", () => {
        const ballotStyle = aSlateBallotStyle(JSON.stringify(SLATES))

        expect(getSlateConfigurationProblem(ballotStyle.ballot_eml)).toBeUndefined()
    })

    it("says why a configuration is refused", () => {
        const ballotStyle = aSlateBallotStyle("{")

        expect(getSlateConfigurationProblem(ballotStyle.ballot_eml)).toContain(
            "the slate configuration is not valid JSON"
        )
    })
})
