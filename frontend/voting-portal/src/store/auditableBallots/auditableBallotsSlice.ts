// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IAuditableBallot} from "sequent-core"

export interface AuditableBallotsState {
    [electionId: number]: IAuditableBallot | undefined
}

const initialState: AuditableBallotsState = {}

export const auditableBallotsSlice = createSlice({
    name: "auditableBallots",
    initialState,
    reducers: {
        setAuditableBallot: (
            state,
            action: PayloadAction<IAuditableBallot>
        ): AuditableBallotsState => {
            state[action.payload.config.id] = action.payload

            return state
        },
    },
})

export const {setAuditableBallot} = auditableBallotsSlice.actions

export const selectAuditableBallot = (electionId: number) => (state: RootState) =>
    state.auditableBallots[electionId]

export default auditableBallotsSlice.reducer
