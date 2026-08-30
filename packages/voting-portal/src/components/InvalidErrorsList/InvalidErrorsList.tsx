// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {WarnBox, EWarnBoxAnnouncement} from "@sequentech/ui-essentials"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {provideBallotService} from "../../services/BallotService"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {useTranslation} from "react-i18next"
import {
    IDecodedVoteContest,
    IInvalidPlaintextError,
    IContest,
    EElectionEventContestEncryptionPolicy,
    BallotSelection,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import {IInvalidPlaintextErrorType} from "../../types/errors"

const ErrorWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
`

// The write-in length error is the one message tied to a specific control, so it
// gets a predictable id that the write-in field can point aria-describedby at.
export const writeInErrorId = (contestId: string): string => `contest-${contestId}-writein-error`

// The contest's answer group points aria-describedby here, so the reason the
// voter cannot continue is read out along with the group.
export const contestErrorsId = (contestId: string): string => `contest-${contestId}-errors`

export interface IInvalidErrorsListProps {
    ballotStyle: IBallotStyle
    question: IContest
    hasWriteIns: boolean
    isInvalidWriteIns: boolean
    setIsInvalidWriteIns: (input: boolean) => void
    setDecodedContests: (input: IDecodedVoteContest) => void
    isReview: boolean
    errorSelectionState: BallotSelection
    isTouched: boolean
    setIsTouched: (value: boolean) => void
}

export const InvalidErrorsList: React.FC<IInvalidErrorsListProps> = ({
    ballotStyle,
    question,
    hasWriteIns,
    isInvalidWriteIns,
    setIsInvalidWriteIns,
    setDecodedContests,
    isReview,
    errorSelectionState,
    isTouched,
    setIsTouched,
}) => {
    const {t} = useTranslation()
    // Note that if we have reviewed, then we can asume we have touched
    const {getWriteInAvailableCharacters, filterVisibleMessages} = provideBallotService()

    const decodedContestSelection = errorSelectionState.find(
        (selection) => selection.contest_id === question.id
    )

    // Which of this contest's messages the voter sees on the screen being
    // rendered is decided by the validation rules in sequent-core, the same
    // ones that produced the messages.
    const filteredSelection = useMemo(
        () =>
            decodedContestSelection
                ? filterVisibleMessages(question, decodedContestSelection, isReview, isTouched)
                : undefined,
        [decodedContestSelection, question, isTouched, isReview]
    )

    useEffect(() => {
        if (decodedContestSelection) {
            setDecodedContests(decodedContestSelection)
        }
    }, [decodedContestSelection])

    useEffect(() => {
        if (isTouched || !decodedContestSelection) {
            return
        }
        let hasTouched = decodedContestSelection?.choices.some((choice) => choice.selected > -1)
        if (hasTouched) {
            setIsTouched(true)
        }
    }, [decodedContestSelection, isTouched])

    const numAvailableChars =
        hasWriteIns && decodedContestSelection
            ? getWriteInAvailableCharacters(decodedContestSelection, ballotStyle.ballot_eml)
            : 0

    useEffect(() => {
        let newInvalid = numAvailableChars < 0
        if (newInvalid !== isInvalidWriteIns) {
            setIsInvalidWriteIns(newInvalid)
        }
    }, [numAvailableChars, isInvalidWriteIns, setIsInvalidWriteIns])

    return (
        <ErrorWrapper className="error-list" id={contestErrorsId(question.id)} role="status">
            {numAvailableChars < 0 ? (
                <WarnBox
                    variant="warning"
                    id={writeInErrorId(question.id)}
                    // The write-in field points aria-describedby at this box, so
                    // announcing it as well would read the same text twice.
                    announcement={EWarnBoxAnnouncement.SILENT}
                    warnId="errors.encoding.writeInCharsExceeded"
                    warnType={IInvalidPlaintextErrorType.EncodingError}
                >
                    {t("errors.encoding.writeInCharsExceeded", {
                        numCharsExceeded: -numAvailableChars,
                    })}
                </WarnBox>
            ) : null}
            {filteredSelection?.invalid_errors.map((error, index) => (
                <WarnBox
                    variant="warning"
                    key={index}
                    announcement={EWarnBoxAnnouncement.SILENT}
                    warnId={error.message}
                    warnType={error.error_type}
                >
                    {t(error.message || "", error.message_map ?? {})}
                </WarnBox>
            ))}
            {filteredSelection?.invalid_alerts.map((error, index) => (
                <WarnBox
                    variant="info"
                    key={index}
                    announcement={EWarnBoxAnnouncement.SILENT}
                    warnId={error.message}
                    warnType={error.error_type}
                >
                    {t(error.message || "", error.message_map ?? {})}
                </WarnBox>
            ))}
        </ErrorWrapper>
    )
}
