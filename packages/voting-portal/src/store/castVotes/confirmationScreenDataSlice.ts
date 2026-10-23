// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createSlice, PayloadAction} from "@reduxjs/toolkit"
import {RootState} from "../store"
import type {EVotingPortalAuditButtonCfg} from "@sequentech/ui-core"
import type {ITransparency} from "../../services/vitrine/presentation"
import type {IVitrineCast} from "../../services/vitrine/types"

/** Vitrine's part of the confirmation (step-integration.md, section 2.6). */
export interface IConfirmationVitrine {
    /** Harvest's answer at cast, as data; null when it gave none. */
    cast: IVitrineCast | null
    /** The event's transparency settings when the ballot was cast. */
    transparency: ITransparency
}

export interface ConfirmationScreenData {
    ballotId: string
    isDemo: boolean
    auditButtonCfg?: EVotingPortalAuditButtonCfg
    vitrine?: IConfirmationVitrine
}

export interface ConfirmationScreenDataState {
    [electionId: string]: ConfirmationScreenData | undefined
}

const initialState: ConfirmationScreenDataState = {}

export const confirmationScreenData = createSlice({
    name: "confirmationScreenData",
    initialState,
    reducers: {
        setConfirmationScreenData: (
            state: ConfirmationScreenDataState,
            action: PayloadAction<{
                electionId: string
                confirmationScreenData: ConfirmationScreenData
            }>
        ): ConfirmationScreenDataState => {
            state[action.payload.electionId] = action.payload.confirmationScreenData

            return state
        },
    },
})

export const {setConfirmationScreenData} = confirmationScreenData.actions
export const selectConfirmationScreenData = (electionId: string) => (state: RootState) =>
    state.confirmationScreenData[electionId]

export default confirmationScreenData.reducer
