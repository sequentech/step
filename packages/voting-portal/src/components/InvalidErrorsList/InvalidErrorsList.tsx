// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {WarnBox} from "@sequentech/ui-essentials"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {provideBallotService} from "../../services/BallotService"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {useTranslation} from "react-i18next"
import {
    IDecodedVoteContest,
    IInvalidPlaintextError,
    IContest,
    EBlankVotePolicy,
    EUnderVotePolicy,
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
    const {
        interpretContestSelection,
        interpretMultiContestSelection,
        getWriteInAvailableCharacters,
    } = provideBallotService()

    let under_vote_policy: EUnderVotePolicy | undefined =
        question?.presentation?.under_vote_policy ?? undefined
    let blank_vote_policy: EBlankVotePolicy | undefined =
        question?.presentation?.blank_vote_policy ?? undefined

    const decodedContestSelection = errorSelectionState.find(
        (selection) => selection.contest_id === question.id
    )

    const filterErrorList = (
        state: IDecodedVoteContest | undefined,
        isTouched: boolean,
        isReview: boolean,
        under_vote_policy?: EUnderVotePolicy,
        blank_vote_policy?: EBlankVotePolicy
    ) => {
        if (!state) return undefined
        // An untouched contest shows nothing on the voting screen.
        if (!isReview && !isTouched) {
            return {...state, invalid_alerts: [], invalid_errors: []}
        }
        // Alert visibility — the only rules that depend on which screen is
        // showing: the warn-only-in-review policies hold their message back
        // until review, and the "maximum reached" hint is a voting-screen
        // aid only.
        let invalid_alerts = state.invalid_alerts.filter(
            (error) =>
                !(
                    ("errors.implicit.underVote" === error.message &&
                        !isReview &&
                        under_vote_policy === EUnderVotePolicy.WARN_ONLY_IN_REVIEW) ||
                    ("errors.implicit.blankVote" === error.message &&
                        !isReview &&
                        blank_vote_policy === EBlankVotePolicy.WARN_ONLY_IN_REVIEW) ||
                    (error.message === "errors.implicit.overVoteDisabled" && isReview)
                )
        )
        // Remove duplicates: an empty ballot shows the blank message rather
        // than the under-vote hint, and an alert whose message already
        // renders as an error is redundant (errors render first).
        const blankVotePresent =
            invalid_alerts.some((error) => error.message === "errors.implicit.blankVote") ||
            state.invalid_errors.some((error) => error.message === "errors.implicit.blankVote")
        invalid_alerts = invalid_alerts.filter(
            (error) =>
                !(
                    ("errors.implicit.underVote" === error.message && blankVotePresent) ||
                    state.invalid_errors.some((e) => e.message === error.message)
                )
        )
        // Errors always render: whatever the invalid-vote policy, the voter
        // is told about anything that affects how the ballot will be
        // counted. The policy's role is the dialog/gate ladder, not
        // information hiding.
        return {...state, invalid_alerts}
    }

    const filteredSelection = useMemo(
        () =>
            filterErrorList(
                decodedContestSelection,
                isTouched,
                isReview,
                under_vote_policy,
                blank_vote_policy
            ),
        [decodedContestSelection, isTouched, isReview, under_vote_policy, blank_vote_policy]
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
        <ErrorWrapper className="error-list">
            {numAvailableChars < 0 ? (
                <WarnBox
                    variant="warning"
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
                    warnId={error.message}
                    warnType={error.error_type}
                >
                    {t(error.message || "", error.message_map ?? {})}
                </WarnBox>
            ))}
        </ErrorWrapper>
    )
}
