// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ISlate, ISlatesConfig} from "../types/Slates"

const primarySubtag = (language: string): string => language.split("-")[0]

/**
 * The name of a slate for a voter: in their language, else in the election's
 * default language, else in the first language the slate is named in.
 */
export const getSlateName = (
    slate: Pick<ISlate, "name">,
    language: string,
    defaultLanguage?: string
): string => {
    const candidates = [language, primarySubtag(language)]
    if (defaultLanguage) {
        candidates.push(defaultLanguage, primarySubtag(defaultLanguage))
    }
    for (const code of candidates) {
        const name = slate.name[code]
        if (name) {
            return name
        }
    }

    const [first] = Object.keys(slate.name).sort()
    return first ? slate.name[first] : ""
}

/** The slate a candidate belongs to, if any. */
export const getCandidateSlate = (
    config: ISlatesConfig,
    contestId: string,
    candidateId: string
): ISlate | undefined =>
    config.slates.find((slate) => slate.members[contestId]?.includes(candidateId))

/** Whether any slate has a candidate in this contest. */
export const hasSlateMembers = (config: ISlatesConfig, contestId: string): boolean =>
    config.slates.some((slate) => (slate.members[contestId]?.length ?? 0) > 0)
