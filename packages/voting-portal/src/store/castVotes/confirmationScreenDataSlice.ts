// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {isUndefined} from "@sequentech/ui-core"

export interface ConfirmationScreenData {
    ballotId: string
    isDemo: boolean
}

export interface ConfirmationScreenDataState {
    [electionId: string]: ConfirmationScreenData | undefined
}

const initialState: ConfirmationScreenDataState = {}

export const confirmationScreenData = createSlice({
    name: "confirmationScreenData",
    initialState,
    reducers: {
        setConfirmationScreenData: (
            state: ConfirmationScreenDataState,
            action: PayloadAction<{
                electionId: string
                confirmationScreenData: ConfirmationScreenData
            }>
        ): ConfirmationScreenDataState => {
            state[action.payload.electionId] = action.payload.confirmationScreenData

            return state
        },
    },
})

export const {setConfirmationScreenData} = confirmationScreenData.actions
export const selectConfirmationScreenData = (electionId: string) => (state: RootState) =>
    state.confirmationScreenData[electionId]

export default confirmationScreenData.reducer
