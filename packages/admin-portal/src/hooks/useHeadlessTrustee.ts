// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useEffect, useRef, useState, useCallback} from "react"
import {useInterval} from "react-use"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {IKeysCeremonyExecutionStatus as EStatus} from "@/services/KeyCeremony"
import init, {initThreadPool, WasmSession} from "braid-wasm"

// Module-level WASM init guard so init+threadPool only run once per page load
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
                console.info("[useHeadlessTrustee] WASM initialized with thread pool")
            } catch (threadErr) {
                console.warn(
                    "[useHeadlessTrustee] Thread pool init failed, running single-threaded:",
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

export interface UseHeadlessTrusteeProps {
    electionEvent?: Sequent_Backend_Election_Event
    currentCeremony: Sequent_Backend_Keys_Ceremony
    trusteeRecord: any
}

/**
 * Silently runs the braid WASM trustee protocol in the background for
 * browser-based trustees when manual keys ceremony is in progress.
 * Mirrors the manual steps a user would take in TrusteeDashboard:
 *   1. Load WASM + thread pool
 *   2. Initialize WasmSession with trustee config
 *   3. init_session(boardName)
 *   4. connect_to_board()
 *   5. Auto-run step() every 1 second while ceremony is active
 */
export const useHeadlessTrustee = ({
    electionEvent,
    currentCeremony,
    trusteeRecord,
}: UseHeadlessTrusteeProps) => {
    const {accessToken, trustee: trusteeName} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    const sessionRef = useRef<WasmSession | null>(null)
    const initializedRef = useRef(false)
    const connectedRef = useRef(false)
    const loadingRef = useRef(false)
    const lastHeartbeatRef = useRef<number>(0)
    const [running, setRunning] = useState(false)

    // Board name is stored in the election event's bulletin board reference
    const boardName: string | undefined = electionEvent?.bulletin_board_reference?.database_name

    const isCeremonyActive =
        currentCeremony?.execution_status === EStatus.IN_PROGRESS ||
        currentCeremony?.execution_status === EStatus.STARTED

    const heartbeatSecs = globalSettings.BRAID_B4_HEARTBEAT

    const step = useCallback(async () => {
        console.info("[headless-trustee] Running protocol step...")
        console.info(
            `[headless-trustee] Current session state: sessionRef=${sessionRef.current}, loading=${loadingRef.current}`
        )
        if (!sessionRef.current || loadingRef.current) return
        loadingRef.current = true
        try {
            await sessionRef.current.step()
            const now = Date.now()
            if (now - lastHeartbeatRef.current >= heartbeatSecs * 1000) {
                lastHeartbeatRef.current = now
                try {
                    await sessionRef.current.heartbeat(trusteeName ?? "")
                } catch (e) {
                    console.warn("[headless-trustee] Heartbeat error:", e)
                }
            }
        } catch (e) {
            console.warn("[headless-trustee] Step error:", e)
        } finally {
            loadingRef.current = false
        }
    }, [heartbeatSecs, trusteeName])

    // Step 1-4: Initialize WASM and connect to board once config is available
    useEffect(() => {
        if (!trusteeRecord || !boardName) return
        if (!accessToken || !trusteeName) return
        if (initializedRef.current) return

        initializedRef.current = true

        const initialize = async () => {
            try {
                console.info("[headless-trustee] Initializing headless trustee protocol runner...")
                // Step 1: Load WASM module and thread pool
                await ensureWasmReady()

                console.info("[headless-trustee] WASM ready, creating session...")
                // WIP - Config set up for development with packages/braid/scripts/trustee1.toml credentials
                // WIP - Configure trustee1 as browser-based and trustee2 as server-based
                // Step 2: Create WasmSession with trustee config
                const config = {
                    name: trusteeName,
                    signing_key_sk:
                        "MC4CAQAwBQYDK2VwBCIEIJAtmrHtGFYiS5tUQepIlrFtCCcKHeSzzuJ2pZqH4bat",
                    signing_key_pk: trusteeRecord?.public_key ?? "",
                    encryption_key: "lQr2vrVuZJ5PAoOkVSfLfuIG7mxt8exlgAnRMBi+4rg",
                    b4_url: globalSettings.B4_URL,
                    access_token: accessToken,
                }
                sessionRef.current = new WasmSession(JSON.stringify(config))

                // Step 3: Initialize session for the ceremony board
                await sessionRef.current.init_session(boardName)
                console.info(`[headless-trustee] Session initialized for board "${boardName}"`)

                // Step 4: Connect to board and sync pending messages
                await sessionRef.current.connect_to_board()
                connectedRef.current = true

                console.info(`[headless-trustee] Connected to board "${boardName}"`)
                setRunning(true)
            } catch (e) {
                initializedRef.current = false
                console.warn("[headless-trustee] Initialization failed:", e)
            }
        }

        initialize()
    }, [
        boardName,
        accessToken,
        trusteeName,
        trusteeRecord,
        globalSettings.B4_URL,
    ])

    // Step 5: Update access token in the active session whenever it is refreshed
    useEffect(() => {
        if (!initializedRef.current || !sessionRef.current) return
        if (!accessToken) return
        try {
            sessionRef.current.update_access_token(accessToken)
        } catch (e) {
            console.warn("[headless-trustee] Failed to update access token:", e)
        }
    }, [accessToken])

    // Stop auto-run when ceremony ends
    useEffect(() => {
        if (!isCeremonyActive && running) {
            setRunning(false)
            console.info("[headless-trustee] Ceremony ended, stopping auto-run")
        }
    }, [isCeremonyActive, running])

    // Auto-run: execute a protocol step every second while connected and running
    useInterval(step, running && connectedRef.current ? 1000 : null)
}
