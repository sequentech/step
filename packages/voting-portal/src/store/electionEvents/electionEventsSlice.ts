// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
            state[action.payload.id] = action.payload
            return state
        },
    },
})

export const {setElectionEvent} = electionEventSlice.actions

export const selectElectionEventById =
    (electionEventId: string | undefined) => (state: RootState) => {
        if (electionEventId) {
            return state.electionEvent[electionEventId]
        }
        return undefined
    }

export default electionEventSlice.reducer
