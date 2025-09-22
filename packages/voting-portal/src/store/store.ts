// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {configureStore, ThunkAction, Action} from "@reduxjs/toolkit"
import ballotStylesReducer from "./ballotStyles/ballotStylesSlice"
import castVotesReducer from "./castVotes/castVotesSlice"
import confirmationScreenDataReducer from "./castVotes/confirmationScreenDataSlice"
import ballotSelectionsReducer from "./ballotSelections/ballotSelectionsSlice"
import auditableBallotsReducer from "./auditableBallots/auditableBallotsSlice"
import electionsReducer from "./elections/electionsSlice"
import electionEventReducer from "./electionEvents/electionEventsSlice"
import supportMaterialReducer from "./supportMaterials/supportMaterialsSlice"
import documentsReducer from "./documents/documentsSlice"
import extraReducer from "./extra/extraSlice"

// note: use Immer, https://immerjs.github.io/immer/

export const store = configureStore({
    reducer: {
        elections: electionsReducer,
        castVotes: castVotesReducer,
        ballotStyles: ballotStylesReducer,
        ballotSelections: ballotSelectionsReducer,
        auditableBallots: auditableBallotsReducer,
        supportMaterials: supportMaterialReducer,
        electionEvent: electionEventReducer,
        extra: extraReducer,
        documents: documentsReducer,
        confirmationScreenData: confirmationScreenDataReducer,
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
