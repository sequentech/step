// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import {Sequent_Backend_Document} from "../../gql/graphql"

export interface DocumentState {
    [documentId: string]: Sequent_Backend_Document | undefined
}

const initialState: DocumentState = {}

export const documentSlice = createSlice({
    name: "documents",
    initialState,
    reducers: {
        setDocument: (
            state: DocumentState,
            action: PayloadAction<Sequent_Backend_Document>
        ): DocumentState => {
            state[action.payload.id] = action.payload
            return state
        },
    },
})

export const {setDocument} = documentSlice.actions

export const selectDocumentById = (documentId: string) => (state: RootState) =>
    state.documents[documentId]

export default documentSlice.reducer
