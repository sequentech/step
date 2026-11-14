// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {IElectionEventPresentation} from "@sequentech/ui-core"

export interface IElectionEvent {
    alias?: string | null
    annotations?: string | null
    audit_election_event_id?: string
    bulletin_board_reference?: string | null
    created_at?: string | null
    dates?: string | null
    description?: string | null
    elections?: Array<string | null>
    elections_aggregate?: string | null
    encryption_protocol?: string | null
    id: string
    is_archived?: boolean | null
    is_audit?: boolean | null
    labels?: string | null
    name?: string | null
    presentation?: IElectionEventPresentation | null
    public_key?: string | null
    statistics?: string | null
    status?: string | null
    tenant_id?: string
    updated_at?: string | null
    user_boards?: string | null
    voting_channels?: string | null
}

export interface ElectionEventState {
    [electionEventId: string]: IElectionEvent | undefined
}

const initialState: ElectionEventState = {}

export const electionEventSlice = createSlice({
    name: "electionEvent",
    initialState,
    reducers: {
        setElectionEvent: (
            state: ElectionEventState,
            action: PayloadAction<IElectionEvent>
        ): ElectionEventState => {
            // Published event records may carry no presentation; keep the one
            // already known so the event's branding never drops mid-session.
            state[action.payload.id] = {
                ...action.payload,
                presentation: action.payload.presentation ?? state[action.payload.id]?.presentation,
            }
            return state
        },
        seedElectionEvent: (
            state: ElectionEventState,
            action: PayloadAction<IElectionEvent>
        ): ElectionEventState => {
            if (!state[action.payload.id]) {
                state[action.payload.id] = action.payload
            }
            return state
        },
    },
})

export const {seedElectionEvent, setElectionEvent} = electionEventSlice.actions

export const selectElectionEventById =
    (electionEventId: string | undefined) => (state: RootState) => {
        if (electionEventId) {
            return state.electionEvent[electionEventId]
        }
        return undefined
    }

// The portal serves one election event per page load, so outside the event's
// own routes (the error page) the event is whichever one is known.
export const selectKnownElectionEvent = (state: RootState): IElectionEvent | undefined =>
    Object.values(state.electionEvent).find((event) => event !== undefined)

export default electionEventSlice.reducer
