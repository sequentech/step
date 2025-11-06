// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"

export interface ISupportMaterial {
    annotations?: string | null
    created_at?: string | null
    data?: string | null
    document_id?: string | null
    election_event_id: string
    id: string
    kind?: string | null
    labels?: string | null
    last_updated_at?: string | null
    tenant_id: string
}

export interface SupportMaterialState {
    [supportMaterialId: string]: ISupportMaterial | undefined
}

const initialState: SupportMaterialState = {}

export const supportMaterialSlice = createSlice({
    name: "supportMaterials",
    initialState,
    reducers: {
        setSupportMaterial: (
            state: SupportMaterialState,
            action: PayloadAction<ISupportMaterial>
        ): SupportMaterialState => {
            state[action.payload.id] = action.payload
            return state
        },
    },
})

export const {setSupportMaterial} = supportMaterialSlice.actions

export const selectSupportMaterialById = (supportMaterialId: string) => (state: RootState) =>
    state.supportMaterials[supportMaterialId]

export const getSupportMaterialsList = () => (state: RootState) => {
    return state.supportMaterials
}

export default supportMaterialSlice.reducer
