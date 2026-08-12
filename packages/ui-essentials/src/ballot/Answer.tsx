// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useBallotSelection} from "./selection"
import {
    stringToHtml,
    isUndefined,
    normalizeWriteInText,
    translate,
    ICandidate,
    IContest,
} from "@sequentech/ui-core"
import Candidate from "../components/Candidate/Candidate"
import Image from "mui-image"
import {
    checkAllowWriteIns,
    checkIsInvalidVote,
    checkIsWriteIn,
    getImageUrl,
    getLinkUrl,
} from "./presentation"
import {IBallotStyle} from "./types"
import {useTranslation} from "react-i18next"
import {IDecodedVoteContest} from "sequent-core"
import {useBallotEngine} from "./engine"
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
    const engine = useBallotEngine()
    const {isPreferential} = engine
    const isPreferentialVote = useMemo(() => {
        if (!contest.counting_algorithm) return false
        return isPreferential(contest.counting_algorithm)
    }, [contest.counting_algorithm])
    const totalCandidates = contest.candidates.length
    const selection = useBallotSelection()
    const selectionState = selection.choice(ballotStyle, contestId, answer.id)
    const questionState = selection.contest(ballotStyle, contestId)
    const question = ballotStyle.ballot_eml.contests.find((contest) => contest.id === contestId)
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
        // Explicit blank candidates intentionally use the standard
        // selection logic.
        return !isUndefined(selectionState) && selectionState.selected > -1
    }
    const setInvalidVote = (value: boolean) => {
        selection.setInvalid({
            ballotStyle,
            contestId,
            isExplicitInvalid: value,
        })
    }

    const setBlankVote = () => {
        setExplicitBlank(true)
        selection.setBlank({
            ballotStyle,
            contestId,
            candidateId: answer.id,
        })
    }

    const handlePreferentialChange = (position: number | null) => {
        if (!isSelectable || isReview) {
            return
        }
        setIsTouched(true)
        setSelectedPosition(position)
        let cleanedText =
            selectionState?.write_in_text && normalizeWriteInText(selectionState?.write_in_text)
        selection.setChoice({
            ballotStyle,
            contestId,
            voteChoice: {
                id: answer.id,
                selected: position ? position - 1 : -1,
                write_in_text: cleanedText,
            },
        })
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
                selection.setChoice({
                    ballotStyle,
                    contestId,
                    voteChoice: {
                        id: answer.id,
                        selected: -1,
                        write_in_text: selectionState?.write_in_text,
                    },
                })
            }
            return
        } else if (value && explicitBlank) {
            setExplicitBlank(false)
        }

        let cleanedText =
            selectionState?.write_in_text && normalizeWriteInText(selectionState?.write_in_text)

        if (isRadioSelection) {
            selection.reset({
                ballotStyle,
                force: true,
                contestId: contest.id,
            })
        }

        selection.setChoice({
            ballotStyle,
            contestId,
            voteChoice: {
                id: answer.id,
                selected: value ? 0 : -1,
                write_in_text: cleanedText,
            },
        })
    }

    const shouldDisable = disableSelect && !isChecked()

    const isWriteIn = checkIsWriteIn(answer)
    const allowWriteIns = question && checkAllowWriteIns(question)

    const setWriteInText = (writeInText: string): void => {
        if (!isWriteIn || !allowWriteIns || !isSelectable || isReview) {
            return
        }
        let cleanedText = normalizeWriteInText(writeInText)

        selection.setChoice({
            ballotStyle,
            contestId,
            voteChoice: {
                id: answer.id,
                selected: isUndefined(selectionState) ? -1 : selectionState.selected,
                write_in_text: cleanedText,
            },
        })
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
                <Image src={`${selection.imageBaseUrl}${imageUrl}`} duration={100} />
            ) : null}
        </Candidate>
    )
}
