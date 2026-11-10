// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IAuditableBallot} from "@sequentech/ui-core"

export interface AuditableBallotEntry {
    auditableBallot: IAuditableBallot
    isBlankBallot: boolean
    // Its secret values were shown to audit it, so it can no longer be cast.
    isAudited?: boolean
    // The ballot box confirmed that it marked this ballot audited, so the
    // audit screen may show its secret values.
    isAuditConfirmed?: boolean
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
        // Only a newly encrypted ballot, which setAuditableBallot stores
        // without this mark, can be cast after an audit.
        markBallotAudited: (state, action: PayloadAction<string>): AuditableBallotsState => {
            const entry = state[action.payload]
            if (entry) {
                entry.isAudited = true
            }

            return state
        },
        // Like markBallotAudited, a newly encrypted ballot comes without it.
        confirmBallotAudited: (state, action: PayloadAction<string>): AuditableBallotsState => {
            const entry = state[action.payload]
            if (entry) {
                entry.isAuditConfirmed = true
            }

            return state
        },
    },
})

export const {setAuditableBallot, markBallotAudited, confirmBallotAudited} =
    auditableBallotsSlice.actions

export const selectAuditableBallot = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.auditableBallot

export const selectIsBallotAudited = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.isAudited ?? false

export const selectIsBallotAuditConfirmed = (electionId: string) => (state: RootState) =>
    state.auditableBallots[electionId]?.isAuditConfirmed ?? false

export default auditableBallotsSlice.reducer
