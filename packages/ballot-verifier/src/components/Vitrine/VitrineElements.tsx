// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Loads the Vitrine UI kit, whose words say what a trust policy holds.

import {useEffect, useState} from "react"
import {loadVitrineUi, TVitrineUi} from "../../services/vitrine/sdk"

export interface IVitrineUiState {
    /** The kit, once it is loaded. */
    ui?: TVitrineUi
    /** True when the kit couldn't be loaded. */
    failed: boolean
}

/** Loads the kit. */
export const useVitrineUi = (): IVitrineUiState => {
    const [state, setState] = useState<IVitrineUiState>({failed: false})
    useEffect(() => {
        let active = true
        loadVitrineUi().then(
            (ui) => {
                if (active) setState({ui, failed: false})
            },
            () => {
                if (active) setState({failed: true})
            }
        )
        return () => {
            active = false
        }
    }, [])
    return state
}
