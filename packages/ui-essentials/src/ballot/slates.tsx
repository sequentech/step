// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, PropsWithChildren, useContext} from "react"
import {useTranslation} from "react-i18next"
import {
    checkIsCategoryList,
    checkIsExplicitBlankVote,
    checkIsInvalidVote,
    checkIsWriteIn,
    getCandidateSlate,
    getSlateName,
    hasSlateMembers,
} from "@sequentech/ui-core"
import type {ICandidate, IContest, ISlatesConfig} from "@sequentech/ui-core"

import type {IBallotStyle} from "./types"

const BallotSlatesContext = createContext<ISlatesConfig | null>(null)

/**
 * The slates of the ballot below, as the host read them from the ballot style.
 *
 * A host without slates supplies nothing and the ballot shows no slate labels.
 */
export const BallotSlatesProvider = ({
    slates,
    children,
}: PropsWithChildren<{slates: ISlatesConfig | null}>): React.JSX.Element => (
    <BallotSlatesContext.Provider value={slates}>{children}</BallotSlatesContext.Provider>
)

export const useBallotSlates = (): ISlatesConfig | null => useContext(BallotSlatesContext)

export const getDefaultLanguageCode = (ballotStyle: IBallotStyle): string | undefined =>
    ballotStyle.ballot_eml.election_presentation?.language_conf?.default_language_code ??
    ballotStyle.ballot_eml.election_event_presentation?.language_conf?.default_language_code

const isBallotMarker = (candidate: ICandidate): boolean =>
    checkIsWriteIn(candidate) ||
    checkIsInvalidVote(candidate) ||
    checkIsExplicitBlankVote(candidate) ||
    checkIsCategoryList(candidate)

/**
 * What a candidate is shown as belonging to: the name of their slate, or the
 * independent label in a contest where other candidates are on a slate.
 */
export const useCandidateSlateLabel = (
    ballotStyle: IBallotStyle,
    contest: IContest,
    candidate: ICandidate
): string | undefined => {
    const slates = useBallotSlates()
    const {t, i18n} = useTranslation()

    if (!slates || !hasSlateMembers(slates, contest.id) || isBallotMarker(candidate)) {
        return undefined
    }

    const slate = getCandidateSlate(slates, contest.id, candidate.id)
    return slate
        ? getSlateName(slate, i18n.language, getDefaultLanguageCode(ballotStyle))
        : t("slates.independent")
}
