// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useRef, useState, useCallback} from "react"
import {useInterval} from "react-use"
import {HeadlessTrusteeContext} from "@/providers/HeadlessTrusteeProvider"
import {Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {IKeysCeremonyExecutionStatus as EStatus} from "@/services/KeyCeremony"

export interface UseHeadlessTrusteeProps {
    currentCeremony: Sequent_Backend_Keys_Ceremony
}

/**
 * Drives the braid protocol step loop while the ceremony is active.
 * Heartbeats are managed autonomously by the WASM session daemon.
 */
export const useHeadlessTrustee = ({currentCeremony}: UseHeadlessTrusteeProps) => {
    const {session} = useContext(HeadlessTrusteeContext)

    const loadingRef = useRef(false)
    const [running, setRunning] = useState(false)

    const isCeremonyActive =
        currentCeremony?.execution_status === EStatus.IN_PROGRESS ||
        currentCeremony?.execution_status === EStatus.AWAITING_TRUSTEE_KEYS

    useEffect(() => {
        if (!session) return
        setRunning(true)
        return () => {
            setRunning(false)
        }
    }, [session])

    // Stop step loop when ceremony ends
    useEffect(() => {
        if (!isCeremonyActive && running) {
            setRunning(false)
            console.info("[useHeadlessTrustee] Ceremony ended, stopping step loop")
        }
    }, [isCeremonyActive, running])

    const step = useCallback(async () => {
        if (!session || loadingRef.current) return
        loadingRef.current = true
        try {
            await session.step()
        } catch (e) {
            console.warn("[useHeadlessTrustee] Step error:", e)
        } finally {
            loadingRef.current = false
        }
    }, [session])

    useInterval(step, running && isCeremonyActive ? 1000 : null)
}
