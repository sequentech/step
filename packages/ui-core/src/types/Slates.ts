// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IDecodedVoteContest} from "sequent-core"

export const SLATES_ANNOTATION = "sequent.slates"

export enum EMobileCandidateLists {
    COLLAPSED = "collapsed",
    EXPANDED = "expanded",
}

export interface ISlate {
    id: string
    /** The display name by language code. */
    name: Record<string, string>
    /** The candidate ids of this slate by contest id. */
    members: Record<string, Array<string>>
}

/** The slates of one election, in the order they are shown. */
export interface ISlatesConfig {
    version: number
    mobile_candidate_lists: EMobileCandidateLists
    slates: Array<ISlate>
}

/** One reason a slate configuration is refused, as sequent-core reports it. */
export interface ISlateProblem {
    severity: string
    code: string
    path: string
    message: string
}

/** The marks a slate adds to and removes from one contest. */
export interface ISlateContestChange {
    contest_id: string
    added: Array<string>
    removed: Array<string>
}

/** The ballot that choosing a slate produces, and what it changes. */
export interface ISlateChoices {
    selection: Array<IDecodedVoteContest>
    /** The contests whose marks change, in ballot order. */
    changes: Array<ISlateContestChange>
}
