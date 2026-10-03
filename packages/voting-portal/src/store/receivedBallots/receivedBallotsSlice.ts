// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IReceivedBallot} from "@sequentech/ui-core"

// Receipts this device has checked against the published ballot box key.
export interface ReceivedBallotsState {
    [electionId: string]: IReceivedBallot | undefined
}

const initialState: ReceivedBallotsState = {}

export const receivedBallotsSlice = createSlice({
    name: "receivedBallots",
    initialState,
    reducers: {
        setReceivedBallot: (
            state,
            action: PayloadAction<{
                electionId: string
                receivedBallot: IReceivedBallot
            }>
        ): ReceivedBallotsState => {
            state[action.payload.electionId] = action.payload.receivedBallot

            return state
        },
    },
})

export const {setReceivedBallot} = receivedBallotsSlice.actions

export const selectReceivedBallot = (electionId: string) => (state: RootState) =>
    state.receivedBallots[electionId]

// The Ballot ID the ballot box signed for this ballot, if it has received it.
export const selectReceivedBallotId =
    (electionId: string, ballotHash: string | undefined) =>
    (state: RootState): string | undefined => {
        const receivedBallot = state.receivedBallots[electionId]
        return receivedBallot && receivedBallot.ballot_hash === ballotHash
            ? receivedBallot.ballot_id
            : undefined
    }

export default receivedBallotsSlice.reducer
