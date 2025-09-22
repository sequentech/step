// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IAuditableBallot} from "@sequentech/ui-core"

export interface AuditableBallotsState {
    [ballotStyleId: string]: IAuditableBallot | undefined
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
            }>
        ): AuditableBallotsState => {
            state[action.payload.electionId] = action.payload.auditableBallot

            return state
        },
    },
})

export const {setAuditableBallot} = auditableBallotsSlice.actions

export const selectAuditableBallot = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]

export default auditableBallotsSlice.reducer
