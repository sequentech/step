// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IAuditableBallot} from "@sequentech/ui-core"

export interface AuditableBallotEntry {
    auditableBallot?: IAuditableBallot
    isBlankBallot: boolean
    auditStatus?: EBallotAuditStatus
}

export enum EBallotAuditStatus {
    PREPARED = "prepared",
    AUDITED = "audited",
}

export interface AuditableBallotsState {
    [ballotStyleId: string]: AuditableBallotEntry | undefined
}

const initialState: AuditableBallotsState = {}

export const auditableBallotsSlice = createSlice({
    name: "auditableBallots",
    initialState,
    reducers: {
        setAuditableBallot: (
            state,
            action: PayloadAction<{
                electionId: string
                auditableBallot: IAuditableBallot
                isBlankBallot: boolean
            }>
        ): AuditableBallotsState => {
            state[action.payload.electionId] = {
                auditableBallot: action.payload.auditableBallot,
                isBlankBallot: action.payload.isBlankBallot,
                auditStatus: EBallotAuditStatus.PREPARED,
            }

            return state
        },
        markBallotAudited: (
            state,
            action: PayloadAction<{electionId: string; ballotHash: string}>
        ) => {
            const ballot = state[action.payload.electionId]
            if (ballot?.auditableBallot?.ballot_hash === action.payload.ballotHash)
                ballot.auditStatus = EBallotAuditStatus.AUDITED
        },
        clearAuditableBallot: (state, action: PayloadAction<string>) => {
            const ballot = state[action.payload]
            // The confirmation keeps only its non-identifying blank-ballot flag.
            if (ballot) state[action.payload] = {isBlankBallot: ballot.isBlankBallot}
        },
    },
})

export const {setAuditableBallot, markBallotAudited, clearAuditableBallot} =
    auditableBallotsSlice.actions

export const selectAuditableBallot = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.auditStatus === EBallotAuditStatus.AUDITED
        ? undefined
        : state.auditableBallots[electionId]?.auditableBallot

export const selectBallotForAudit = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.auditableBallot

export const selectBallotAuditStatus = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.auditStatus

export const selectIsBlankBallot = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.isBlankBallot ?? false

export default auditableBallotsSlice.reducer
