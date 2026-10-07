// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EMobileCandidateLists,
    IBallotStyle as IElectionDTO,
    IContest,
    ISlatesConfig,
    SLATES_ANNOTATION,
} from "@sequentech/ui-core"

import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {aBallotStyle, aCandidate, aContest} from "./ballotHarness"

export const PRESIDENT: IContest = aContest({
    id: "president",
    name: "President",
    candidates: [
        aCandidate("f-president", "Jordan Ellis", {contest_id: "president"}),
        aCandidate("m-president", "Morgan Hayes", {contest_id: "president"}),
        aCandidate("i-president", "Avery Brooks", {contest_id: "president"}),
    ],
})

export const TRUSTEES: IContest = aContest({
    id: "trustees",
    name: "Trustees",
    max_votes: 3,
    candidates: [
        aCandidate("f-t1", "Rowan Scott", {contest_id: "trustees"}),
        aCandidate("f-t2", "Charlie Kim", {contest_id: "trustees"}),
        aCandidate("m-t1", "Skyler James", {contest_id: "trustees"}),
        aCandidate("v-t1", "Harper Lane", {contest_id: "trustees"}),
        aCandidate("i-t1", "Blair Lewis", {contest_id: "trustees"}),
    ],
})

/** Two slates across both contests and one with trustees only. */
export const SLATES: ISlatesConfig = {
    version: 1,
    mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
    slates: [
        {
            id: "forward",
            name: {en: "Forward Together", es: "Adelante Juntos"},
            members: {president: ["f-president"], trustees: ["f-t1", "f-t2"]},
        },
        {
            id: "members",
            name: {en: "Members First"},
            members: {president: ["m-president"], trustees: ["m-t1"]},
        },
        {
            id: "voices",
            name: {en: "Independent Voices"},
            members: {trustees: ["v-t1"]},
        },
    ],
}

/** A ballot style with both contests, carrying `annotation` as its slates. */
export const aSlateBallotStyle = (annotation?: string): IBallotStyle => {
    const ballotStyle = aBallotStyle(PRESIDENT)
    const ballotEml: IElectionDTO = {
        ...ballotStyle.ballot_eml,
        contests: [PRESIDENT, TRUSTEES],
        election_annotations: annotation === undefined ? {} : {[SLATES_ANNOTATION]: annotation},
    }
    return {...ballotStyle, ballot_eml: ballotEml}
}
