// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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
