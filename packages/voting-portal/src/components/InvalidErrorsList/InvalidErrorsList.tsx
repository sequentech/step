// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {WarnBox} from "@sequentech/ui-essentials"
import {IQuestion} from "sequent-core"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {provideBallotService} from "../../services/BallotService"
import {useAppSelector} from "../../store/hooks"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {useTranslation} from "react-i18next"

export interface IInvalidErrorsListProps {
    ballotStyle: IBallotStyle
    question: IQuestion
}

export const InvalidErrorsList: React.FC<IInvalidErrorsListProps> = ({ballotStyle, question}) => {
    const {t} = useTranslation()
    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle.election_id)
    )
    const {interpretBallotSelection} = provideBallotService()

    const decodedSelection =
        selectionState && interpretBallotSelection(selectionState, ballotStyle.ballot_eml)
    const decodedContestSelection = decodedSelection?.find(
        (contest) => contest.contest_id === question.id
    )

    return (
        <>
            {decodedContestSelection?.invalid_errors.map((error, index) => (
                <WarnBox variant="warning" key={index}>
                    {t(
                        error.message || "",
                        error.message_map && Object.fromEntries(error.message_map)
                    )}
                </WarnBox>
            ))}
        </>
    )
}
