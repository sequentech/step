// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"

export interface ExtraState {
    bypassChooser: boolean
}

const initialState: ExtraState = {
    bypassChooser: false,
}

export const extraSlice = createSlice({
    name: "extra",
    initialState,
    reducers: {
        setBypassChooser: (state: ExtraState, action: PayloadAction<boolean>): ExtraState => {
            state.bypassChooser = action.payload
            return state
        },
    },
})

export const {setBypassChooser} = extraSlice.actions

export const selectBypassChooser = () => (state: RootState) => state.extra.bypassChooser

export default extraSlice.reducer
