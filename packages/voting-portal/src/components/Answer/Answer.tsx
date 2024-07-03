// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
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
import {
    resetBallotSelection,
    selectBallotSelectionQuestion,
    selectBallotSelectionVoteChoice,
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {
    checkAllowWriteIns,
    checkIsWriteIn,
    getImageUrl,
    getLinkUrl,
} from "../../services/ElectionConfigService"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "../../providers/SettingsContextProvider"

export interface IAnswerProps {
    answer: ICandidate
    contestId: string
    index: number
    ballotStyle: IBallotStyle
    hasCategory?: boolean
    isActive: boolean
    isReview: boolean
    isInvalidVote?: boolean
    isInvalidWriteIns?: boolean
    isRadioSelection?: boolean
    contest: IContest
}

export const Answer: React.FC<IAnswerProps> = ({
    answer,
    contestId,
    ballotStyle,
    hasCategory,
    isActive,
    isReview,
    isInvalidVote,
    isInvalidWriteIns,
    isRadioSelection,
    contest,
}) => {
    const selectionState = useAppSelector(
        selectBallotSelectionVoteChoice(ballotStyle.election_id, contestId, answer.id)
    )
    const questionState = useAppSelector(
        selectBallotSelectionQuestion(ballotStyle.election_id, contestId)
    )
    const question = ballotStyle.ballot_eml.contests.find((contest) => contest.id === contestId)
    const dispatch = useAppDispatch()
    const {globalSettings} = useContext(SettingsContext)
    const imageUrl = getImageUrl(answer)
    const infoUrl = getLinkUrl(answer)
    const {i18n} = useTranslation()

    const isChecked = (): boolean => {
        if (!isInvalidVote) {
            return !isUndefined(selectionState) && selectionState.selected > -1
        } else {
            return !isUndefined(questionState) && questionState.is_explicit_invalid
        }
    }
    const setInvalidVote = (value: boolean) => {
        dispatch(
            setBallotSelectionInvalidVote({
                ballotStyle,
                contestId,
                isExplicitInvalid: value,
            })
        )
    }
    const setChecked = (value: boolean) => {
        if (!isActive || isReview) {
            return
        }
        if (isInvalidVote) {
            setInvalidVote(value)
            return
        }

        let cleanedText =
            selectionState?.write_in_text && normalizeWriteInText(selectionState?.write_in_text)

        if (isRadioSelection) {
            dispatch(
                resetBallotSelection({
                    ballotStyle,
                    force: true,
                    contestId: contest.id,
                })
            )
        }

        dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId,
                voteChoice: {
                    id: answer.id,
                    selected: value ? 0 : -1,
                    write_in_text: cleanedText,
                },
            })
        )
    }

    const isWriteIn = checkIsWriteIn(answer)
    const allowWriteIns = question && checkAllowWriteIns(question)

    const setWriteInText = (writeInText: string): void => {
        if (!isWriteIn || !allowWriteIns || !isActive || isReview) {
            return
        }
        let cleanedText = normalizeWriteInText(writeInText)

        dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId,
                voteChoice: {
                    id: answer.id,
                    selected: isUndefined(selectionState) ? -1 : selectionState.selected,
                    write_in_text: cleanedText,
                },
            })
        )
    }

    if (isReview && !isChecked()) {
        return null
    }

    return (
        <Candidate
            title={translate(answer, "name", i18n.language)}
            description={stringToHtml(translate(answer, "description", i18n.language) || "")}
            isActive={isActive}
            checked={isChecked()}
            setChecked={setChecked}
            url={infoUrl}
            hasCategory={hasCategory}
            isWriteIn={allowWriteIns && isWriteIn}
            writeInValue={selectionState?.write_in_text}
            setWriteInText={setWriteInText}
            isInvalidVote={isInvalidVote}
            isInvalidWriteIn={!!selectionState?.write_in_text && isInvalidWriteIns}
        >
            {imageUrl ? (
                <Image src={`${globalSettings.PUBLIC_BUCKET_URL}${imageUrl}`} duration={100} />
            ) : null}
        </Candidate>
    )
}
