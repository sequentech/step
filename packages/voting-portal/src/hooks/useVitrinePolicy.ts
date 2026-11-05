// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState} from "react"
import type {Policy} from "@vitrine/verify"
import {pinnedPolicy} from "../services/vitrine/boothCheck"
import type {ITransparency} from "../services/vitrine/presentation"
import {loadVitrineSdk} from "../services/vitrine/sdk"
import {ETransparencyMode} from "../services/vitrine/types"

/** The policy the event pins, for the screens that have no receipt to read it
 * from (the ballot tracker, the Logs tab). Only a public event's policy is in
 * its presentation; a private event's comes with the voter's session. */
export const useVitrinePolicy = (transparency: ITransparency): Policy | undefined =>
    useVitrinePolicyState(transparency).policy

export interface IVitrinePolicyState {
    policy?: Policy
    /** The policy is there, or will not be: the event pins none, or it did not load. */
    settled: boolean
}

/** As {@link useVitrinePolicy}, saying whether the wait is over, for what
 * falls back to another route when there is no policy. */
export const useVitrinePolicyState = (transparency: ITransparency): IVitrinePolicyState => {
    const mode = transparency.mode
    const vitrine = transparency.vitrine
    const [state, setState] = useState<IVitrinePolicyState>({
        settled: mode !== ETransparencyMode.PUBLIC || !vitrine?.policy,
    })

    useEffect(() => {
        if (mode !== ETransparencyMode.PUBLIC || !vitrine?.policy) {
            setState({settled: true})
            return
        }
        setState({settled: false})
        let current = true
        loadVitrineSdk()
            .then((sdk) => {
                const pinned = pinnedPolicy(sdk, mode, vitrine, undefined)
                if (current)
                    setState(pinned.ok ? {policy: pinned.policy, settled: true} : {settled: true})
            })
            .catch(() => {
                if (current) setState({settled: true})
            })
        return () => {
            current = false
        }
    }, [mode, vitrine])

    return state
}
