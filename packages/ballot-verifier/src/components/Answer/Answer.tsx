// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import {useAppDispatch, useAppSelector} from "../../store/hooks"
import {
    stringToHtml,
    isUndefined,
    normalizeWriteInText,
    translate,
    ICandidate,
    IContest,
} from "@sequentech/ui-core"
import {Candidate} from "@sequentech/ui-essentials"
import Image from "mui-image"
// import {
//     resetBallotSelection,
//     selectBallotSelectionQuestion,
//     selectBallotSelectionVoteChoice,
//     setBallotSelectionBlankVote,
//     setBallotSelectionInvalidVote,
//     setBallotSelectionVoteChoice,
// } from "../../store/ballotSelections/ballotSelectionsSlice"
import {
    // checkAllowWriteIns,
    checkIsWriteIn,
    getImageUrl,
    // getLinkUrl,
} from "../../services/ElectionConfigService"
import {provideBallotService} from "../../services/BallotService"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {IDecodedVoteContest} from "sequent-core"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"

export interface IAnswerProps {
    answer: ICandidate
    contestId: string
    index: number
    ballotStyle: IBallotStyle
    hasCategory?: boolean
    isActive: boolean
    iconCheckboxPolicy?: ECandidatesIconCheckboxPolicy
    isReview: boolean
    isInvalidVote?: boolean
    isExplicitBlankVote?: boolean
    isInvalidWriteIns?: boolean
    isRadioSelection?: boolean
    contest: IContest
    selectedChoicesSum: number
    setSelectedChoicesSum: (num: number) => void
    disableSelect: boolean
}

export const Answer: React.FC<IAnswerProps> = ({
    answer,
    contestId,
    ballotStyle,
    hasCategory,
    isActive,
    iconCheckboxPolicy,
    isReview,
    isInvalidVote,
    isExplicitBlankVote,
    isInvalidWriteIns,
    isRadioSelection,
    contest,
    selectedChoicesSum,
    setSelectedChoicesSum,
    disableSelect,
}) => {
    const imageUrl = getImageUrl(answer)

    const isWriteIn = checkIsWriteIn(answer)
    const writeInValue = (answer as any).write_in_text

    return (
        <Candidate
            title={answer?.name || ""}
            description={answer?.description}
            isWriteIn={isWriteIn}
            writeInValue={writeInValue}
            shouldDisable={false}
        >
            {imageUrl ? <Image src={imageUrl} duration={100} /> : null}
        </Candidate>
    )
}
