// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo, useState} from "react"
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
    setBallotSelectionBlankVote,
    setBallotSelectionInvalidVote,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {
    checkAllowWriteIns,
    checkIsInvalidVote,
    checkIsWriteIn,
    getImageUrl,
    getLinkUrl,
} from "../../services/ElectionConfigService"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "../../providers/SettingsContextProvider"
import {IDecodedVoteContest} from "sequent-core"
import {provideBallotService} from "../../services/BallotService"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"
import {ICountingAlgorithm} from "@sequentech/ui-core"

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
    explicitBlank: boolean
    setExplicitBlank: (value: boolean) => void
    setIsTouched: (value: boolean) => void
}

export const Answer: React.FC<IAnswerProps> = ({
    answer,
    contestId,
    ballotStyle,
    hasCategory,
    isActive,
    iconCheckboxPolicy,
    isReview,
    isInvalidVote: isInvalidVoteInput,
    isExplicitBlankVote,
    isInvalidWriteIns,
    isRadioSelection,
    contest,
    selectedChoicesSum,
    setSelectedChoicesSum,
    disableSelect,
    explicitBlank,
    setExplicitBlank,
    setIsTouched,
}) => {
    const isPreferentialVote = contest.counting_algorithm == ICountingAlgorithm.INSTANT_RUNOFF
    // TODO: WASM function wich calls is_preferencial()
    // WASM function that calls to check the validity of a vote, no gaps
    const totalCandidates = contest.candidates.length
    const [selectedPosition, setSelectedPosition] = useState<number | null>(null)

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
    const ballotService = provideBallotService()
    const isInvalidVote = useMemo(
        () => isInvalidVoteInput ?? checkIsInvalidVote(answer),
        [isInvalidVoteInput, answer]
    )

    const isChecked = (): boolean => {
        if (isInvalidVote) {
            return !isUndefined(questionState) && questionState.is_explicit_invalid
        } else if (isExplicitBlankVote) {
            return (
                !isUndefined(questionState) &&
                !!ballotService.checkIsBlank(questionState) &&
                explicitBlank
            )
        } else {
            return !isUndefined(selectionState) && selectionState.selected > -1
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

    const setBlankVote = () => {
        setExplicitBlank(true)
        dispatch(
            setBallotSelectionBlankVote({
                ballotStyle,
                contestId,
            })
        )
    }
    
    const handlePreferentialChange = (position: number | null ) => {
        if (!isActive || isReview) {
            return
        }
        setIsTouched(true)
        setSelectedPosition(position)
        let cleanedText =
            selectionState?.write_in_text && normalizeWriteInText(selectionState?.write_in_text)
        console.log("position: ", position)
        dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId,
                voteChoice: {
                    id: answer.id,
                    selected: position ? (position - 1) : -1,
                    write_in_text: cleanedText,
                },
            })
        ) 
    }
    const setChecked = (value: boolean) => {
        console.log("setChecked value: ", value)

        if (!isActive || isReview) {
            return
        }
        setIsTouched(true)
        if (isInvalidVote) {
            setInvalidVote(value)
            return
        }

        if (isExplicitBlankVote) {
            if (value) {
                setBlankVote()
            } else {
                setExplicitBlank(false)
            }
            return
        } else if (value && explicitBlank) {
            setExplicitBlank(false)
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

    const shouldDisable = disableSelect && selectionState?.selected === -1

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

    if (isReview && !!isExplicitBlankVote) {
        return null
    }

    return (
        <Candidate
            isPreferentialVote={isPreferentialVote}
            totalCandidates={totalCandidates}
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
            shouldDisable={shouldDisable}
            iconCheckboxPolicy={iconCheckboxPolicy}
            selectedPosition={selectedPosition}
            handlePreferentialChange={handlePreferentialChange}
        >
            {imageUrl ? (
                <Image src={`${globalSettings.PUBLIC_BUCKET_URL}${imageUrl}`} duration={100} />
            ) : null}
        </Candidate>
    )
}
