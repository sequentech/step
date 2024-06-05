// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {WarnBox, IContest} from "@sequentech/ui-essentials"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {provideBallotService} from "../../services/BallotService"
import {useAppSelector} from "../../store/hooks"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {useTranslation} from "react-i18next"
import {IDecodedVoteContest, IInvalidPlaintextError} from "sequent-core"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"

const ErrorWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 4px;
`

export interface IInvalidErrorsListProps {
    ballotStyle: IBallotStyle
    question: IContest
    isInvalidWriteIns: boolean
    setIsInvalidWriteIns: (input: boolean) => void
    setDecodedContests: (input: IDecodedVoteContest) => void
    isReview: boolean
}

export const InvalidErrorsList: React.FC<IInvalidErrorsListProps> = ({
    ballotStyle,
    question,
    isInvalidWriteIns,
    setIsInvalidWriteIns,
    setDecodedContests,
    isReview,
}) => {
    const {t} = useTranslation()
    const [isTouched, setIsTouched] = useState(false)
    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle.election_id)
    )
    const {interpretContestSelection, getWriteInAvailableCharacters} = provideBallotService()
    const contestSelection = useMemo(()=> selectionState?.find((contest) => contest.contest_id === question.id),[selectionState])

    useEffect(() => {
        if (isTouched || !contestSelection) {
            return
        }
        let hasTouched = contestSelection?.choices.some((choice) => choice.selected > -1)
        if (hasTouched) {
            setIsTouched(true)
        }
    }, [contestSelection, isTouched])

    const decodedContestSelection = useMemo(()=>{
        return contestSelection && interpretContestSelection(contestSelection, ballotStyle.ballot_eml)
    },[contestSelection])

    useEffect(()=>{
            if (!isReview && !isTouched && decodedContestSelection) {
                decodedContestSelection.invalid_errors = decodedContestSelection?.invalid_errors.filter(
                    (error) => error.message !== "errors.implicit.selectedMin"
                )
            }
    },[isReview ,isTouched , decodedContestSelection])

    useEffect(() => {
        if (decodedContestSelection) {
            setDecodedContests(decodedContestSelection)
        }
    }, [decodedContestSelection, decodedContestSelection?.invalid_errors])

    const numAvailableChars = contestSelection
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
            {decodedContestSelection?.invalid_errors.map((error, index) => (
                <WarnBox variant="warning" key={index}>
                    {t(error.message || "", error.message_map ?? {})}
                </WarnBox>
            ))}
            {decodedContestSelection?.invalid_alerts.map((error, index) => (
                <WarnBox variant="info" key={index}>
                    {t(error.message || "", error.message_map ?? {})}
                </WarnBox>
            ))}
        </ErrorWrapper>
    )
}
