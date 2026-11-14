// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    configureStore,
    ThunkAction,
    Action,
    combineReducers,
    createAction,
    createReducer,
} from "@reduxjs/toolkit"
import ballotStylesReducer from "./ballotStyles/ballotStylesSlice"
import castVotesReducer from "./castVotes/castVotesSlice"
import confirmationScreenDataReducer from "./castVotes/confirmationScreenDataSlice"
import ballotSelectionsReducer from "./ballotSelections/ballotSelectionsSlice"
import auditableBallotsReducer from "./auditableBallots/auditableBallotsSlice"
import electionsReducer from "./elections/electionsSlice"
import electionEventReducer, {
    ElectionEventState,
    seedElectionEvent,
} from "./electionEvents/electionEventsSlice"
import supportMaterialReducer from "./supportMaterials/supportMaterialsSlice"
import documentsReducer from "./documents/documentsSlice"
import extraReducer from "./extra/extraSlice"

// note: use Immer, https://immerjs.github.io/immer/

export const clearVoterSession = createAction("session/clear")

// What the public event configuration said about each event. It is not voter
// data, so it outlives a voter session.
const publicElectionEventReducer = createReducer({} as ElectionEventState, (builder) => {
    builder.addCase(seedElectionEvent, (state, action) => {
        state[action.payload.id] = action.payload
    })
})

const appReducer = combineReducers({
    elections: electionsReducer,
    castVotes: castVotesReducer,
    ballotStyles: ballotStylesReducer,
    ballotSelections: ballotSelectionsReducer,
    auditableBallots: auditableBallotsReducer,
    supportMaterials: supportMaterialReducer,
    electionEvent: electionEventReducer,
    publicElectionEvent: publicElectionEventReducer,
    extra: extraReducer,
    documents: documentsReducer,
    confirmationScreenData: confirmationScreenDataReducer,
})

type AppState = ReturnType<typeof appReducer>

export const rootReducer = (state: AppState | undefined, action: Action): AppState => {
    if (!state || !clearVoterSession.match(action)) {
        return appReducer(state, action)
    }
    // Logging in is itself a session change. Starting again from the public
    // configuration keeps the event's branding in place across it.
    return {
        ...appReducer(undefined, action),
        publicElectionEvent: state.publicElectionEvent,
        electionEvent: {...state.publicElectionEvent},
    }
}

export const store = configureStore({reducer: rootReducer})

export type AppDispatch = typeof store.dispatch
export type RootState = ReturnType<typeof store.getState>
export type AppThunk<ReturnType = void> = ThunkAction<
    ReturnType,
    RootState,
    unknown,
    Action<string>
>
