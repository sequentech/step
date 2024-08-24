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
    const [isTouched, setIsTouched] = useState(false)
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
        const state =
            contestSelection && interpretContestSelection(contestSelection, ballotStyle.ballot_eml)
        setDecodedContestSelection(state)
        setFilteredSelection(state)
    }, [contestSelection])

    useEffect(() => {
        if (!isReview && !isTouched && !isVotedState) {
            // Filter min selection error in case where no user interaction was yet made
            setFilteredSelection((prev) => {
                if (!prev) return undefined
                return {
                    ...prev,
                    invalid_errors:
                        prev?.invalid_errors.filter(
                            (error) =>
                                error.message !== "errors.implicit.selectedMin" &&
                                error.message !== "errors.implicit.blankVote"
                        ) || [],
                    invalid_alerts:
                        prev?.invalid_alerts.filter(
                            (error) =>
                                error.message !== "errors.implicit.underVote" &&
                                error.message !== "errors.implicit.blankVote"
                        ) || [],
                }
            })
        }
    }, [isReview, isTouched])

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
