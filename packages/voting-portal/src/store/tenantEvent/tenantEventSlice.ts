import {createSlice} from "@reduxjs/toolkit"
import type {PayloadAction} from "@reduxjs/toolkit"

export interface TenantEventState {
    tenantId: string
    eventId: string
}

const initialState: TenantEventState = {
    tenantId: "",
    eventId: "",
}

export const tenantEventSlice = createSlice({
    name: "tenant-event",
    initialState,
    reducers: {
        setTenantEvent: (state, action: PayloadAction<TenantEventState>) => {
            state = action.payload
        },
    },
})

// Action creators are generated for each case reducer function
export const {setTenantEvent} = tenantEventSlice.actions

export default tenantEventSlice.reducer
