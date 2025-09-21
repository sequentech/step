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
import {
    IDecodedVoteContest,
    IInvalidPlaintextError,
    IContest,
    EBlankVotePolicy,
    EUnderVotePolicy,
    EElectionEventContestEncryptionPolicy,
    BallotSelection,
    EInvalidVotePolicy,
    EOverVotePolicy,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {Box} from "@mui/material"
import {isVotedByElectionId} from "../../store/extra/extraSlice"
import {useParams} from "react-router-dom"
import {error} from "console"
import {IInvalidPlaintextErrorType} from "sequent-core"

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
    const {electionId} = useParams<{electionId?: string}>()
    const isVotedState = useAppSelector(isVotedByElectionId(electionId))
    const {
        interpretContestSelection,
        interpretMultiContestSelection,
        getWriteInAvailableCharacters,
    } = provideBallotService()

    let under_vote_policy: EUnderVotePolicy | undefined =
        question?.presentation?.under_vote_policy ?? undefined
    let blank_vote_policy: EBlankVotePolicy | undefined =
        question?.presentation?.blank_vote_policy ?? undefined
    let invalid_vote_policy: EInvalidVotePolicy | undefined =
        question?.presentation?.invalid_vote_policy ?? undefined
    let over_vote_policy: EOverVotePolicy | undefined =
        question?.presentation?.over_vote_policy ?? undefined

    const decodedContestSelection = errorSelectionState.find(
        (selection) => selection.contest_id === question.id
    )

    const containsError = (state: IDecodedVoteContest | undefined, message: string) => {
        if (!state) return false
        return (
            state.invalid_alerts.find((error) => error.message === message) ||
            state.invalid_errors.find((error) => error.message === message)
        )
    }

    const filterErrorList = (
        state: IDecodedVoteContest | undefined,
        isTouched: boolean,
        isVotedState: boolean,
        isReview: boolean,
        under_vote_policy?: EUnderVotePolicy,
        blank_vote_policy?: EBlankVotePolicy,
        invalid_vote_policy?: EInvalidVotePolicy,
        over_vote_policy?: EOverVotePolicy
    ) => {
        if (!state) return undefined
        var ret = {
            ...state,
            invalid_alerts:
                state?.invalid_alerts.filter(
                    // !() is used so that function instead of behaving like
                    // "show error when this happens" behaves more like "hide
                    // error when this happens"
                    (error) => {
                        let ret = !(
                            ("errors.implicit.underVote" === error.message &&
                                !isReview &&
                                under_vote_policy === EUnderVotePolicy.WARN_ONLY_IN_REVIEW) ||
                            ("errors.implicit.blankVote" === error.message &&
                                !isReview &&
                                blank_vote_policy === EBlankVotePolicy.WARN_ONLY_IN_REVIEW) ||
                            (error.message === "errors.implicit.overVoteDisabled" && isReview)
                        )
                        if (!ret) {
                            console.log(`
                                invalid_alerts: filtering out alert: ${error.message}.
                                - error.message: ${error.message}
                                - isReview: ${isReview}
                                - isTouched: ${isTouched}
                                - isVotedState: ${isVotedState}
                                - under_vote_policy: ${under_vote_policy}
                                - blank_vote_policy: ${blank_vote_policy}
                            `)
                        } else {
                            console.log(`invalid_alerts: NOT filtering out error: ${error.message}`)
                        }
                        return ret
                    }
                ) || [],
        }
        if (!isReview && !isTouched) {
            ret.invalid_alerts = []
            ret.invalid_errors = []
        }

        // remove duplicates
        ret.invalid_alerts = ret.invalid_alerts.filter(
            (error) =>
                !(
                    // if there's blank vote, remove underVote
                    (
                        ("errors.implicit.underVote" === error.message &&
                            containsError(ret, "errors.implicit.blankVote")) ||
                        // if overvote is an error, remove the info message
                        ("errors.implicit.selectedMax" === error.message &&
                            containsError(ret, "errors.implicit.selectedMax"))
                    )
                )
        )
        ret.invalid_errors = ret.invalid_errors.filter((error) => {
            let ret = !(
                invalid_vote_policy === EInvalidVotePolicy.ALLOWED &&
                !(
                    "errors.implicit.selectedMax" === error.message &&
                    over_vote_policy !== EOverVotePolicy.ALLOWED
                ) &&
                !(
                    "errors.implicit.blankVote" === error.message &&
                    blank_vote_policy === EBlankVotePolicy.NOT_ALLOWED
                )
            )
            return ret
        })

        return ret
    }

    const filteredSelection = useMemo(
        () =>
            filterErrorList(
                decodedContestSelection,
                isTouched,
                isVotedState,
                isReview,
                under_vote_policy,
                blank_vote_policy,
                invalid_vote_policy,
                over_vote_policy
            ),
        [
            decodedContestSelection,
            isTouched,
            isVotedState,
            isReview,
            under_vote_policy,
            blank_vote_policy,
        ]
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
