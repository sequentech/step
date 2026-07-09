// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
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
import {provideBallotService} from "../../services/BallotService"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"

export interface IAnswerProps {
    answer: ICandidate
    contestId: string
    index: number
    ballotStyle: IBallotStyle
    hasCategory?: boolean
    isSelectable: boolean
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
    showWhenListSelected?: boolean
}

export const Answer: React.FC<IAnswerProps> = ({
    answer,
    contestId,
    ballotStyle,
    hasCategory,
    isSelectable,
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
    showWhenListSelected,
}) => {
    const {isPreferential} = provideBallotService()
    const isPreferentialVote = useMemo(() => {
        if (!contest.counting_algorithm) return false
        return isPreferential(contest.counting_algorithm)
    }, [contest.counting_algorithm])
    const totalCandidates = contest.candidates.length
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
    const isInvalidVote = useMemo(
        () => isInvalidVoteInput ?? checkIsInvalidVote(answer),
        [isInvalidVoteInput, answer]
    )
    const [selectedPosition, setSelectedPosition] = useState<number | null>(null)

    useEffect(() => {
        const sel = selectionState?.selected ?? -1
        setSelectedPosition(sel + 1) // Selected positions in the UI start at 1, not 0. And 0 means no selection
    }, [selectionState])

    const isChecked = (): boolean => {
        if (isInvalidVote) {
            return !isUndefined(questionState) && questionState.is_explicit_invalid
        }
        return !isUndefined(selectionState) && selectionState.selected > -1
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
                candidateId: answer.id,
            })
        )
    }

    const handlePreferentialChange = (position: number | null) => {
        if (!isSelectable || isReview) {
            return
        }
        setIsTouched(true)
        setSelectedPosition(position)
        let cleanedText =
            selectionState?.write_in_text && normalizeWriteInText(selectionState?.write_in_text)
        dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId,
                voteChoice: {
                    id: answer.id,
                    selected: position ? position - 1 : -1,
                    write_in_text: cleanedText,
                },
            })
        )
    }
    const setChecked = (value: boolean) => {
        if (!isSelectable || isReview || isPreferentialVote) {
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
                dispatch(
                    setBallotSelectionVoteChoice({
                        ballotStyle,
                        contestId,
                        voteChoice: {
                            id: answer.id,
                            selected: -1,
                            write_in_text: selectionState?.write_in_text,
                        },
                    })
                )
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
        if (!isWriteIn || !allowWriteIns || !isSelectable || isReview) {
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

    if (isReview && !isChecked() && !showWhenListSelected) {
        return null
    }

    return (
        <Candidate
            isPreferentialVote={isPreferentialVote}
            totalCandidates={totalCandidates}
            maxVotes={contest.max_votes}
            title={translate(answer, "name", i18n.language)}
            description={stringToHtml(translate(answer, "description", i18n.language) || "")}
            isSelectable={isSelectable}
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
