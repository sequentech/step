// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {configureStore, ThunkAction, Action} from "@reduxjs/toolkit"
import ballotStylesReducer from "./ballotStyles/ballotStylesSlice"

// note: use Immer, https://immerjs.github.io/immer/

export const store = configureStore({
    reducer: {
        ballotStyles: ballotStylesReducer,
    },
})

export type AppDispatch = typeof store.dispatch
export type RootState = ReturnType<typeof store.getState>
export type AppThunk<ReturnType = void> = ThunkAction<
    ReturnType,
    RootState,
    unknown,
    Action<string>
>
