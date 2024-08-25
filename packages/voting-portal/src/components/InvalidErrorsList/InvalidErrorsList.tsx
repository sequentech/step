// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {WarnBox} from "@sequentech/ui-essentials"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {provideBallotService} from "../../services/BallotService"
import {useAppSelector} from "../../store/hooks"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {useTranslation} from "react-i18next"
import {IDecodedVoteContest, IInvalidPlaintextError, IContest} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import {isVotedByElectionId} from "../../store/extra/extraSlice"
import {useParams} from "react-router-dom"

const ErrorWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 4px;
`

export interface IInvalidErrorsListProps {
    ballotStyle: IBallotStyle
    question: IContest
    hasWriteIns: boolean
    isInvalidWriteIns: boolean
    setIsInvalidWriteIns: (input: boolean) => void
    setDecodedContests: (input: IDecodedVoteContest) => void
    isReview: boolean
}

export const InvalidErrorsList: React.FC<IInvalidErrorsListProps> = ({
    ballotStyle,
    question,
    hasWriteIns,
    isInvalidWriteIns,
    setIsInvalidWriteIns,
    setDecodedContests,
    isReview,
}) => {
    const {t} = useTranslation()
    // Note that if we have reviewed, then we can asume we have touched
    const [isTouched, setIsTouched] = useState(isReview)
    const [decodedContestSelection, setDecodedContestSelection] = useState<
        IDecodedVoteContest | undefined
    >(undefined)
    const [filteredSelection, setFilteredSelection] = useState<IDecodedVoteContest | undefined>(
        undefined
    )
    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle.election_id)
    )
    const {electionId} = useParams<{electionId?: string}>()
    const isVotedState = useAppSelector(isVotedByElectionId(electionId))
    const {interpretContestSelection, getWriteInAvailableCharacters} = provideBallotService()
    const contestSelection = useMemo(
        () => selectionState?.find((contest) => contest.contest_id === question.id),
        [selectionState]
    )

    const filterErrorList = (state: IDecodedVoteContest | undefined) => {
        if (!state) return undefined
        return {
            ...state,
            invalid_errors:
                state?.invalid_errors.filter(
                    // !() is used so that function instead of behaving like
                    // "show error when this happens" behaves more like "hide
                    // error when this happens"
                    (error) => {
                        let ret = !(
                            // If no interaction is made and not in review screen,
                            // filter out selectedMin & blank vote errors
                            (
                                ["errors.implicit.selectedMin", "errors.implicit.blankVote"].find(
                                    (e) => e === error.message
                                ) &&
                                !isReview &&
                                !isTouched &&
                                !isVotedState
                            )
                        )
                        if (!ret) {
                            console.log(`filtering out error: ${error.message}`)
                        } else {
                            console.log(`NOT filtering out error: ${error.message}`)
                        }
                        return ret
                    }
                ) || [],
            invalid_alerts:
                state?.invalid_alerts.filter(
                    // !() is used so that function instead of behaving like
                    // "show error when this happens" behaves more like "hide
                    // error when this happens"
                    (error) => {
                        let ret = !(
                            // If no interaction is made and not in review screen,
                            // filter out some errors
                            (
                                ([
                                    "errors.implicit.selectedMin",
                                    "errors.implicit.underVote",
                                    "errors.implicit.blankVote",
                                ].find((e) => e === error.message) &&
                                    !isReview &&
                                    !isTouched &&
                                    !isVotedState) ||
                                (error.message === "errors.implicit.overVoteDisabled" && isReview)
                            )
                        )
                        if (!ret) {
                            console.log(`filtering out alert: ${error.message}`)
                        } else {
                            console.log(`NOT filtering out error: ${error.message}`)
                        }
                        return ret
                    }
                ) || [],
        }
    }

    useEffect(() => {
        if (isTouched || !contestSelection) {
            return
        }
        let hasTouched = contestSelection?.choices.some((choice) => choice.selected > -1)
        if (hasTouched) {
            setIsTouched(true)
        }
    }, [contestSelection, isTouched])

    useEffect(() => {
        let state =
            contestSelection && interpretContestSelection(contestSelection, ballotStyle.ballot_eml)
        setDecodedContestSelection(state)
        setFilteredSelection((_) => filterErrorList(state))
    }, [contestSelection])

    useEffect(() => {
        setFilteredSelection(filterErrorList)
    }, [isReview, isTouched, isVotedState])

    useEffect(() => {
        if (decodedContestSelection) {
            setDecodedContests(decodedContestSelection)
        }
    }, [decodedContestSelection])

    const numAvailableChars =
        hasWriteIns && contestSelection
            ? getWriteInAvailableCharacters(contestSelection, ballotStyle.ballot_eml)
            : 0

    useEffect(() => {
        let newInvalid = numAvailableChars < 0
        if (newInvalid !== isInvalidWriteIns) {
            setIsInvalidWriteIns(newInvalid)
        }
    }, [numAvailableChars, isInvalidWriteIns, setIsInvalidWriteIns])

    return (
        <ErrorWrapper>
            {numAvailableChars < 0 ? (
                <WarnBox variant="warning">
                    {t("errors.encoding.writeInCharsExceeded", {
                        numCharsExceeded: -numAvailableChars,
                    })}
                </WarnBox>
            ) : null}
            {filteredSelection?.invalid_errors.map((error, index) => (
                <WarnBox variant="warning" key={index}>
                    {t(error.message || "", error.message_map ?? {})}
                </WarnBox>
            ))}
            {filteredSelection?.invalid_alerts.map((error, index) => (
                <WarnBox variant="info" key={index}>
                    {t(error.message || "", error.message_map ?? {})}
                </WarnBox>
            ))}
        </ErrorWrapper>
    )
}
