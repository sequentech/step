// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback} from "react"
import {
    BallotSelection,
    EVoterSigningPolicy,
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    isUndefined,
} from "@sequentech/ui-core"
import {provideBallotService} from "../services/BallotService"
import {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {setBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {setAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {useAppDispatch} from "../store/hooks"

export const useEncryptBallotForReview = () => {
    const dispatch = useAppDispatch()
    const {
        encryptBallotSelection,
        encryptMultiBallotSelection,
        decodeAuditableBallot,
        decodeAuditableMultiBallot,
        signHashableMultiBallot,
        signHashableBallot,
        hashMultiBallot,
        hashBallot,
    } = provideBallotService()

    const encryptAndStoreBallot = useCallback(
        (
            ballotStyle: IBallotStyle,
            selectionState: BallotSelection,
            isMultiContest: boolean
        ): boolean => {
            try {
                const doSignBallot =
                    ballotStyle.ballot_eml.election_event_presentation?.voter_signing_policy ===
                    EVoterSigningPolicy.WITH_SIGNATURE

                const auditableBallot = isMultiContest
                    ? encryptMultiBallotSelection(selectionState, ballotStyle.ballot_eml)
                    : encryptBallotSelection(selectionState, ballotStyle.ballot_eml)

                const ballotId = isMultiContest
                    ? hashMultiBallot(auditableBallot as IAuditableMultiBallot)
                    : hashBallot(auditableBallot as IAuditableSingleBallot)

                if (doSignBallot) {
                    const signedContent = isMultiContest
                        ? signHashableMultiBallot(
                              ballotId,
                              ballotStyle.election_id,
                              auditableBallot as IAuditableMultiBallot
                          )
                        : signHashableBallot(
                              ballotId,
                              ballotStyle.election_id,
                              auditableBallot as IAuditableSingleBallot
                          )
                    auditableBallot.voter_signing_pk = signedContent?.public_key
                    auditableBallot.voter_ballot_signature = signedContent?.signature
                }

                let decodedSelectionState = isMultiContest
                    ? decodeAuditableMultiBallot(auditableBallot as IAuditableMultiBallot)
                    : decodeAuditableBallot(auditableBallot as IAuditableSingleBallot)

                const isBlankBallot = Boolean(
                    decodedSelectionState?.length &&
                    decodedSelectionState.every((contest) => contest.is_blank_ballot)
                )

                dispatch(
                    setAuditableBallot({
                        electionId: ballotStyle.election_id,
                        auditableBallot,
                        isBlankBallot,
                    })
                )

                if (!isUndefined(decodedSelectionState) && decodedSelectionState !== null) {
                    dispatch(
                        setBallotSelection({
                            ballotStyle,
                            ballotSelection: decodedSelectionState,
                        })
                    )
                }

                return true
            } catch (error) {
                console.error("Failed to encrypt ballot for review:", error)
                return false
            }
        },
        [
            dispatch,
            encryptBallotSelection,
            encryptMultiBallotSelection,
            decodeAuditableBallot,
            decodeAuditableMultiBallot,
            signHashableMultiBallot,
            signHashableBallot,
            hashMultiBallot,
            hashBallot,
        ]
    )

    return {encryptAndStoreBallot}
}
