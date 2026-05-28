// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useCallback, useContext, useEffect, useRef, useState} from "react"
import {useQuery} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ETrusteeModePolicy, getDefaultTrusteeModePolicy} from "@sequentech/ui-core"
import {GET_TRUSTEE_CONFIG} from "@/queries/GetTrusteeConfig"
import init, {initThreadPool, WasmSession} from "braid-wasm"

// Module-level WASM init guard — runs once per page load regardless of re-renders
let wasmReady = false
let wasmInitPromise: Promise<void> | null = null

export const ensureWasmReady = (): Promise<void> => {
    if (wasmReady) return Promise.resolve()
    if (wasmInitPromise) return wasmInitPromise
    wasmInitPromise = (async () => {
        try {
            await init({})
            try {
                await initThreadPool(navigator.hardwareConcurrency || 4)
                console.info("[HeadlessTrusteeProvider] WASM initialized with thread pool")
            } catch (threadErr) {
                console.warn(
                    "[HeadlessTrusteeProvider] Thread pool init failed, running single-threaded:",
                    threadErr
                )
            }
            wasmReady = true
        } catch (e) {
            wasmInitPromise = null
            throw e
        }
    })()
    return wasmInitPromise
}

export interface HeadlessTrusteeContextValue {
    session: WasmSession | null
    isConnected: boolean
    acquireControl: () => void
    releaseControl: () => void
}

export const HeadlessTrusteeContext = createContext<HeadlessTrusteeContextValue>({
    session: null,
    isConnected: false,
    acquireControl: () => {},
    releaseControl: () => {},
})

interface HeadlessTrusteeProviderProps {
    boardName: string | undefined
    children: React.ReactNode
}

export const HeadlessTrusteeProvider: React.FC<HeadlessTrusteeProviderProps> = ({
    boardName,
    children,
}) => {
    const {accessToken, trustee: trusteeName, tenantId} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    const [session, setSession] = useState<WasmSession | null>(null)
    const [isConnected, setIsConnected] = useState(false)
    // Ref keeps the live session accessible to the cleanup function without waiting for React state
    const sessionRef = useRef<WasmSession | null>(null)
    // true = provider owns the session (runs heartbeat); false = wizard has acquired control
    const hasControlRef = useRef(true)
    const lastHeartbeatRef = useRef<number>(0)
    const heartbeatSecs = globalSettings.BRAID_B4_HEARTBEAT

    const {data: trusteeData} = useQuery(GET_TRUSTEE_CONFIG, {
        variables: {tenantId, name: trusteeName},
        skip: !trusteeName || !tenantId,
    })
    const trusteeRecord = trusteeData?.sequent_backend_trustee?.[0]
    const isBrowserBased =
        (trusteeRecord?.annotations?.trustee_mode_policy ?? getDefaultTrusteeModePolicy()) ===
        ETrusteeModePolicy.BROWSER_BASED

    // Initialize WASM + session whenever board or trustee config becomes available
    useEffect(() => {
        if (!boardName || !trusteeRecord || !accessToken || !trusteeName || !isBrowserBased) return

        let cancelled = false

        const initialize = async () => {
            try {
                await ensureWasmReady()

                // WIP - signing keys are hardcoded for development
                const config = {
                    name: trusteeName,
                    signing_key_sk:
                        "MC4CAQAwBQYDK2VwBCIEIJAtmrHtGFYiS5tUQepIlrFtCCcKHeSzzuJ2pZqH4bat",
                    signing_key_pk: trusteeRecord.public_key ?? "",
                    encryption_key: "lQr2vrVuZJ5PAoOkVSfLfuIG7mxt8exlgAnRMBi+4rg",
                    b4_url: globalSettings.B4_URL,
                    access_token: accessToken,
                }
                const newSession = new WasmSession(JSON.stringify(config))
                await newSession.init_session(boardName)
                await newSession.connect_to_board()

                if (!cancelled) {
                    console.info(
                        `[HeadlessTrusteeProvider] Connected to board "${boardName}"`
                    )
                    sessionRef.current = newSession
                    setSession(newSession)
                    setIsConnected(true)
                } else {
                    newSession.free()
                }
            } catch (e) {
                console.warn("[HeadlessTrusteeProvider] Initialization failed:", e)
            }
        }

        initialize()
        return () => {
            cancelled = true
            sessionRef.current?.free()
            sessionRef.current = null
            setSession(null)
            setIsConnected(false)
        }
    }, [
        boardName,
        trusteeRecord,
        accessToken,
        trusteeName,
        isBrowserBased,
        globalSettings.B4_URL,
    ])

    // Keep access token current in the active session
    useEffect(() => {
        if (!session || !accessToken) return
        try {
            session.update_access_token(accessToken)
        } catch (e) {
            console.warn("[HeadlessTrusteeProvider] Failed to update access token:", e)
        }
    }, [accessToken, session])

    // Heartbeat loop — runs only when the provider has control (wizard not active)
    useEffect(() => {
        if (!session || !isConnected) return

        const tick = async () => {
            if (!hasControlRef.current) return
            const now = Date.now()
            if (now - lastHeartbeatRef.current < heartbeatSecs * 1000) return
            lastHeartbeatRef.current = now
            try {
                await session.heartbeat(trusteeName ?? "")
            } catch (e) {
                console.warn("[HeadlessTrusteeProvider] Heartbeat error:", e)
            }
        }

        const interval = setInterval(tick, 1000)
        return () => clearInterval(interval)
    }, [session, isConnected, heartbeatSecs, trusteeName])

    const acquireControl = useCallback(() => {
        hasControlRef.current = false
    }, [])

    const releaseControl = useCallback(() => {
        hasControlRef.current = true
    }, [])

    return (
        <HeadlessTrusteeContext.Provider
            value={{session, isConnected, acquireControl, releaseControl}}
        >
            {children}
        </HeadlessTrusteeContext.Provider>
    )
}
