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
export const useVitrinePolicy = (transparency: ITransparency): Policy | undefined => {
    const [policy, setPolicy] = useState<Policy>()
    const mode = transparency.mode
    const vitrine = transparency.vitrine

    useEffect(() => {
        setPolicy(undefined)
        if (mode !== ETransparencyMode.PUBLIC || !vitrine?.policy) return
        let current = true
        loadVitrineSdk()
            .then((sdk) => {
                const pinned = pinnedPolicy(sdk, mode, vitrine, undefined)
                if (current && pinned.ok) setPolicy(pinned.policy)
            })
            .catch(() => undefined)
        return () => {
            current = false
        }
    }, [mode, vitrine])

    return policy
}
