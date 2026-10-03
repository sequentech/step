// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BallotSelection, IDecodedVoteContest} from "@sequentech/ui-core"
import {ESlateSelectionStatus, getSlateSelectionSummary, SlateMembers} from "./SlateSelection"

const PRESIDENT = "president"
const VICE = "vice-president"
const TRUSTEES = "trustees"

const CANDIDATES: Record<string, string[]> = {
    [PRESIDENT]: ["f-president", "m-president", "i-president"],
    [VICE]: ["f-vice", "m-vice", "i-vice"],
    [TRUSTEES]: ["f-t1", "f-t2", "f-t3", "m-t1", "m-t2", "m-t3", "v-t1", "v-t2", "v-t3", "i-t1"],
}

const FORWARD: SlateMembers = {
    [PRESIDENT]: ["f-president"],
    [VICE]: ["f-vice"],
    [TRUSTEES]: ["f-t1", "f-t2", "f-t3"],
}
const MEMBERS_FIRST: SlateMembers = {
    [PRESIDENT]: ["m-president"],
    [VICE]: ["m-vice"],
    [TRUSTEES]: ["m-t1", "m-t2", "m-t3"],
}
const VOICES: SlateMembers = {
    [TRUSTEES]: ["v-t1", "v-t2", "v-t3"],
}

const ballot = (
    selected: string[],
    over: Record<string, Partial<IDecodedVoteContest>> = {}
): BallotSelection =>
    Object.entries(CANDIDATES).map(([contestId, candidateIds]) => ({
        contest_id: contestId,
        is_explicit_invalid: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: candidateIds.map((id) => ({id, selected: selected.includes(id) ? 0 : -1})),
        ...over[contestId],
    }))

describe("getSlateSelectionSummary", () => {
    it("reports nothing when no member is selected", () => {
        expect(getSlateSelectionSummary(FORWARD, ballot(["m-president", "i-t1"]))).toEqual({
            status: ESlateSelectionStatus.NONE,
            selected: 0,
            total: 5,
            selectedMemberIds: [],
        })
    })

    it("reports nothing when there is no ballot selection yet", () => {
        expect(getSlateSelectionSummary(FORWARD, undefined)).toEqual({
            status: ESlateSelectionStatus.NONE,
            selected: 0,
            total: 0,
            selectedMemberIds: [],
        })
    })

    it("reports all selected when every covered contest holds exactly the slate", () => {
        const summary = getSlateSelectionSummary(
            FORWARD,
            ballot(["f-president", "f-vice", "f-t1", "f-t2", "f-t3"])
        )

        expect(summary.status).toBe(ESlateSelectionStatus.ALL)
        expect(summary.selected).toBe(5)
        expect(summary.total).toBe(5)
        expect(summary.selectedMemberIds).toEqual(["f-president", "f-vice", "f-t1", "f-t2", "f-t3"])
    })

    it("keeps a partial slate as all selected when other offices have choices", () => {
        const summary = getSlateSelectionSummary(
            VOICES,
            ballot(["f-president", "i-vice", "v-t1", "v-t2", "v-t3"])
        )

        expect(summary).toMatchObject({status: ESlateSelectionStatus.ALL, selected: 3, total: 3})
    })

    it("reports partly selected when only its members are selected in its offices", () => {
        const summary = getSlateSelectionSummary(FORWARD, ballot(["f-president", "f-t2"]))

        expect(summary).toMatchObject({
            status: ESlateSelectionStatus.PARTLY,
            selected: 2,
            total: 5,
            selectedMemberIds: ["f-president", "f-t2"],
        })
    })

    it("reports partly selected for a partial slate whatever the other offices hold", () => {
        const summary = getSlateSelectionSummary(VOICES, ballot(["m-president", "v-t1", "v-t3"]))

        expect(summary).toMatchObject({
            status: ESlateSelectionStatus.PARTLY,
            selected: 2,
            total: 3,
        })
    })

    it("reports mixed when a covered office also holds a candidate outside the slate", () => {
        const summary = getSlateSelectionSummary(
            FORWARD,
            ballot(["i-president", "f-vice", "f-t1", "f-t2", "f-t3"])
        )

        expect(summary).toMatchObject({
            status: ESlateSelectionStatus.MIXED,
            selected: 4,
            total: 5,
            selectedMemberIds: ["f-vice", "f-t1", "f-t2", "f-t3"],
        })
    })

    it("counts one individually chosen trustee from each slate as mixed in all of them", () => {
        const selection = ballot(["f-t1", "m-t1", "v-t1"])

        expect(getSlateSelectionSummary(FORWARD, selection)).toMatchObject({
            status: ESlateSelectionStatus.MIXED,
            selected: 1,
            total: 5,
            selectedMemberIds: ["f-t1"],
        })
        expect(getSlateSelectionSummary(MEMBERS_FIRST, selection)).toMatchObject({
            status: ESlateSelectionStatus.MIXED,
            selected: 1,
            total: 5,
        })
        expect(getSlateSelectionSummary(VOICES, selection)).toMatchObject({
            status: ESlateSelectionStatus.MIXED,
            selected: 1,
            total: 3,
        })
    })

    it("is mixed, not all, when every member is selected next to an extra candidate", () => {
        const members: SlateMembers = {[TRUSTEES]: ["v-t1", "v-t2"]}

        expect(getSlateSelectionSummary(members, ballot(["v-t1", "v-t2", "i-t1"]))).toMatchObject({
            status: ESlateSelectionStatus.MIXED,
            selected: 2,
            total: 2,
        })
    })

    it("treats an explicitly invalid covered office as a choice outside the slate", () => {
        const summary = getSlateSelectionSummary(
            FORWARD,
            ballot(["f-vice", "f-t1", "f-t2", "f-t3"], {[PRESIDENT]: {is_explicit_invalid: true}})
        )

        expect(summary).toMatchObject({status: ESlateSelectionStatus.MIXED, selected: 4, total: 5})
    })

    it("leaves contests outside the voter's ballot out of the count", () => {
        const members: SlateMembers = {...FORWARD, "other-area-contest": ["x-1", "x-2"]}

        expect(
            getSlateSelectionSummary(
                members,
                ballot(["f-president", "f-vice", "f-t1", "f-t2", "f-t3"])
            )
        ).toMatchObject({status: ESlateSelectionStatus.ALL, selected: 5, total: 5})
    })

    it("ignores members that are not candidates of the contest", () => {
        const members: SlateMembers = {[PRESIDENT]: ["f-president", "ghost"]}

        expect(getSlateSelectionSummary(members, ballot(["f-president"]))).toMatchObject({
            status: ESlateSelectionStatus.ALL,
            selected: 1,
            total: 1,
        })
    })

    it("follows deselection back to nothing selected", () => {
        const all = ballot(["v-t1", "v-t2", "v-t3"])
        const none = ballot([])

        expect(getSlateSelectionSummary(VOICES, all).status).toBe(ESlateSelectionStatus.ALL)
        expect(getSlateSelectionSummary(VOICES, none)).toMatchObject({
            status: ESlateSelectionStatus.NONE,
            selected: 0,
            selectedMemberIds: [],
        })
    })
})
