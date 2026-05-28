// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useRef, useState, useCallback} from "react"
import {useInterval} from "react-use"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {HeadlessTrusteeContext} from "@/providers/HeadlessTrusteeProvider"
import {Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {IKeysCeremonyExecutionStatus as EStatus} from "@/services/KeyCeremony"

export interface UseHeadlessTrusteeProps {
    currentCeremony: Sequent_Backend_Keys_Ceremony
}

/**
 * Acquires exclusive control of the pre-initialized WasmSession from
 * HeadlessTrusteeProvider and drives the braid protocol step loop while
 * the ceremony is active. The provider's background heartbeat is paused
 * for the duration so only one caller touches the session at a time.
 */
export const useHeadlessTrustee = ({currentCeremony}: UseHeadlessTrusteeProps) => {
    const {session, acquireControl, releaseControl} = useContext(HeadlessTrusteeContext)
    const {trustee: trusteeName} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    const loadingRef = useRef(false)
    const lastHeartbeatRef = useRef<number>(0)
    const [running, setRunning] = useState(false)
    const heartbeatSecs = globalSettings.BRAID_B4_HEARTBEAT

    const isCeremonyActive =
        currentCeremony?.execution_status === EStatus.IN_PROGRESS ||
        currentCeremony?.execution_status === EStatus.STARTED

    // Acquire exclusive session control on mount, release on unmount
    useEffect(() => {
        if (!session) return
        acquireControl()
        setRunning(true)
        return () => {
            releaseControl()
            setRunning(false)
        }
    }, [session, acquireControl, releaseControl])

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
            const now = Date.now()
            if (now - lastHeartbeatRef.current >= heartbeatSecs * 1000) {
                lastHeartbeatRef.current = now
                try {
                    await session.heartbeat(trusteeName ?? "")
                } catch (e) {
                    console.warn("[useHeadlessTrustee] Heartbeat error:", e)
                }
            }
        } catch (e) {
            console.warn("[useHeadlessTrustee] Step error:", e)
        } finally {
            loadingRef.current = false
        }
    }, [session, heartbeatSecs, trusteeName])

    useInterval(step, running && isCeremonyActive ? 1000 : null)
}
