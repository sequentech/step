// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useContext, useEffect, useRef, useState} from "react"
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

// Module-level session registry — one entry per board, persists across route
// changes for the entire browser tab session. Sessions keep their heartbeat
// daemons running even when the user navigates away to a different election event.
const sessionRegistry = new Map<string, WasmSession>()
// Tracks the trustee identity paired with the registry so we can detect key
// rotation and flush stale sessions.
let registryIdentity = ""

export interface HeadlessTrusteeContextValue {
    session: WasmSession | null
    isConnected: boolean
}

export const HeadlessTrusteeContext = createContext<HeadlessTrusteeContextValue>({
    session: null,
    isConnected: false,
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

    // Keeps the latest accessToken available to the init effect without making
    // it a dependency (token refreshes are handled by update_access_token below).
    const accessTokenRef = useRef<string>(accessToken ?? "")
    useEffect(() => {
        accessTokenRef.current = accessToken ?? ""
    }, [accessToken])

    const {data: trusteeData} = useQuery(GET_TRUSTEE_CONFIG, {
        variables: {tenantId, name: trusteeName},
        skip: !trusteeName || !tenantId,
    })
    const trusteeRecord = trusteeData?.sequent_backend_trustee?.[0]
    const isBrowserBased =
        (trusteeRecord?.annotations?.trustee_mode_policy ?? getDefaultTrusteeModePolicy()) ===
        ETrusteeModePolicy.BROWSER_BASED

    // Initialize WASM + session whenever board or trustee config becomes available.
    // Sessions are stored in the module-level registry so they survive route changes.
    useEffect(() => {
        if (!boardName || !trusteeRecord || !trusteeName || !isBrowserBased) return

        // Detect trustee identity change (e.g. key rotation) — flush all sessions.
        const identity = `${trusteeName}::${trusteeRecord.public_key ?? ""}`
        if (identity !== registryIdentity) {
            sessionRegistry.forEach((s) => s.free())
            sessionRegistry.clear()
            registryIdentity = identity
        }

        // Reuse the existing session for this board if already initialized.
        const existing = sessionRegistry.get(boardName)
        if (existing) {
            setSession(existing)
            setIsConnected(true)
            return
        }

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
                    access_token: accessTokenRef.current,
                }
                const newSession = new WasmSession(JSON.stringify(config))
                await newSession.init_session(boardName)
                await newSession.connect_to_board()

                if (!cancelled) {
                    newSession.start_heartbeat_daemon(globalSettings.BRAID_B4_HEARTBEAT)
                    sessionRegistry.set(boardName, newSession)
                    console.info(`[HeadlessTrusteeProvider] Connected to board "${boardName}"`)
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
            // Don't remove from registry — session keeps running across route changes.
            setSession(null)
            setIsConnected(false)
        }
    }, [
        boardName,
        trusteeRecord,
        trusteeName,
        isBrowserBased,
        globalSettings.B4_URL,
        globalSettings.BRAID_B4_HEARTBEAT,
    ])

    // Keep access token current in all registry sessions.
    useEffect(() => {
        if (!accessToken) return
        sessionRegistry.forEach((s) => {
            try {
                s.update_access_token(accessToken)
            } catch (e) {
                console.warn("[HeadlessTrusteeProvider] Failed to update access token:", e)
            }
        })
    }, [accessToken])

    return (
        <HeadlessTrusteeContext.Provider value={{session, isConnected}}>
            {children}
        </HeadlessTrusteeContext.Provider>
    )
}
