// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {configureStore, ThunkAction, Action} from "@reduxjs/toolkit"
import electionsReducer from "./elections/electionsSlice"
import ballotSelectionsReducer from "./ballotSelections/ballotSelectionsSlice"
import auditableBallotsReducer from "./auditableBallots/auditableBallotsSlice"

// note: use Immer, https://immerjs.github.io/immer/

export const store = configureStore({
    reducer: {
        elections: electionsReducer,
        ballotSelections: ballotSelectionsReducer,
        auditableBallots: auditableBallotsReducer,
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
