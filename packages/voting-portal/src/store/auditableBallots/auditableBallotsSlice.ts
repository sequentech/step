// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IAuditableBallot} from "@sequentech/ui-core"

export interface AuditableBallotEntry {
    auditableBallot: IAuditableBallot
    isBlankBallot: boolean
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
            }

            return state
        },
    },
})

export const {setAuditableBallot} = auditableBallotsSlice.actions

export const selectAuditableBallot = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.auditableBallot

export const selectIsBlankBallot = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.isBlankBallot ?? false

export default auditableBallotsSlice.reducer
