// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useContext, useEffect, useRef, useState} from "react"
import {useMutation, useQuery} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ETrusteeModePolicy, getDefaultTrusteeModePolicy} from "@sequentech/ui-core"
import {GET_TRUSTEE_CONFIG} from "@/queries/GetTrusteeConfig"
import {REGISTER_TRUSTEE_KEY} from "@/queries/RegisterTrusteeKey"
import init, {generate_trustee_keys, initThreadPool, WasmSession} from "braid-wasm"

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

// Module-level session registry — one entry per ceremony, keyed by keysCeremonyId.
const sessionRegistry = new Map<string, WasmSession>()
// Tracks the current trustee so we can flush all sessions on user change.
let registryIdentity = ""

// Module-level keys registry — in-memory only, keyed by keysCeremonyId.
// Module scope outlives any single component mount, so keys survive the user
// navigating between ceremonies or election events within the same tab.
// Keys are cleared only when the trustee identity changes (different user).
// They are lost on tab close, which is intentional — the downloaded TOML
// backup is the only durable copy.
interface TrusteeKeys {
    signing_key_sk: string
    signing_key_pk: string
    encryption_key: string
}

const keysRegistry = new Map<string, TrusteeKeys>()

const ensureKeys = (keysCeremonyId: string): TrusteeKeys => {
    const existing = keysRegistry.get(keysCeremonyId)
    if (existing) return existing
    const generated = generate_trustee_keys()
    keysRegistry.set(keysCeremonyId, generated)
    console.info(
        `[HeadlessTrusteeProvider] Generated new trustee keys for ceremony "${keysCeremonyId}"`
    )
    return generated
}

export interface HeadlessTrusteeContextValue {
    session: WasmSession | null
    isConnected: boolean
    initError: string | null
    keyMismatchWarning: string | null
}

export const HeadlessTrusteeContext = createContext<HeadlessTrusteeContextValue>({
    session: null,
    isConnected: false,
    initError: null,
    keyMismatchWarning: null,
})

interface HeadlessTrusteeProviderProps {
    boardName: string | undefined
    electionEventId: string | undefined
    keysCeremonyId: string | undefined
    children: React.ReactNode
}

export const HeadlessTrusteeProvider: React.FC<HeadlessTrusteeProviderProps> = ({
    boardName,
    electionEventId,
    keysCeremonyId,
    children,
}) => {
    const {accessToken, trustee: trusteeName, tenantId} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    const [session, setSession] = useState<WasmSession | null>(null)
    const [isConnected, setIsConnected] = useState(false)
    const [initError, setInitError] = useState<string | null>(null)
    const [keyMismatchWarning, setKeyMismatchWarning] = useState<string | null>(null)

    const [registerTrusteeKey] = useMutation(REGISTER_TRUSTEE_KEY)

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

    // TEMP FIX: detect when the key held in this browser tab won't match what's
    // already registered in the DB for this ceremony, before we silently
    // generate/overwrite it. This does not block anything yet — it only warns.
    // See Tasks.txt "UI: Key restore flow" for the real fix.
    useEffect(() => {
        if (!keysCeremonyId || !trusteeRecord) {
            setKeyMismatchWarning(null)
            return
        }
        if (trusteeRecord.keys_ceremony_id !== keysCeremonyId || !trusteeRecord.public_key) {
            setKeyMismatchWarning(null)
            return
        }
        const localKey = keysRegistry.get(keysCeremonyId)?.signing_key_pk
        if (!localKey) {
            setKeyMismatchWarning(
                "TEMP FIX WARNING: a public key is already registered in the database for this ceremony, but this browser tab has no matching key in memory. A new key is about to be generated and will NOT match the key already baked into the on-board Configuration — protocol steps for this trustee will fail. Recreate the ceremony or restore the original key backup."
            )
        } else if (localKey !== trusteeRecord.public_key) {
            setKeyMismatchWarning(
                "TEMP FIX WARNING: the key held in this browser tab does not match the public key registered in the database for this ceremony. The on-board Configuration was built with a different key — protocol steps for this trustee will fail. Recreate the ceremony or restore the original key backup."
            )
        } else {
            setKeyMismatchWarning(null)
        }
    }, [keysCeremonyId, trusteeRecord])

    // Initialize WASM + session when the user enters a specific ceremony screen.
    // The provider is only mounted for the active ceremony, so keysCeremonyId is
    // the stable registry key for this session's lifetime.
    useEffect(() => {
        if (
            !boardName ||
            !electionEventId ||
            !keysCeremonyId ||
            !trusteeRecord ||
            !trusteeName ||
            !isBrowserBased
        )
            return

        // Detect trustee identity change (different user) — flush all sessions and keys.
        if (trusteeName !== registryIdentity) {
            sessionRegistry.forEach((s) => s.free())
            sessionRegistry.clear()
            keysRegistry.clear()
            registryIdentity = trusteeName
        }

        // Reuse the existing session for this ceremony if already initialized.
        // Restart the heartbeat daemon — it was stopped when the provider last unmounted.
        const existing = sessionRegistry.get(keysCeremonyId)
        if (existing) {
            existing.start_heartbeat_daemon(globalSettings.BRAID_B4_HEARTBEAT)
            setSession(existing)
            setIsConnected(true)
            return
        }

        let cancelled = false

        const initialize = async () => {
            try {
                await ensureWasmReady()

                // Load or generate keys for this ceremony (in-memory only).
                const keys = ensureKeys(keysCeremonyId)
                await registerTrusteeKey({
                    variables: {
                        publicKey: keys.signing_key_pk,
                        electionEventId,
                        keysCeremonyId,
                    },
                })

                const config = {
                    name: trusteeName,
                    signing_key_sk: keys.signing_key_sk,
                    signing_key_pk: keys.signing_key_pk,
                    encryption_key: keys.encryption_key,
                    b4_url: globalSettings.B4_URL,
                    access_token: accessTokenRef.current,
                }
                const newSession = new WasmSession(JSON.stringify(config))
                await newSession.init_session(boardName)
                await newSession.connect_to_board()

                if (!cancelled) {
                    newSession.start_heartbeat_daemon(globalSettings.BRAID_B4_HEARTBEAT)
                    sessionRegistry.set(keysCeremonyId, newSession)
                    console.info(
                        `[HeadlessTrusteeProvider] Connected to board "${boardName}" for ceremony "${keysCeremonyId}"`
                    )
                    setSession(newSession)
                    setIsConnected(true)
                } else {
                    newSession.free()
                }
            } catch (e) {
                console.warn("[HeadlessTrusteeProvider] Initialization failed:", e)
                if (!cancelled) {
                    setInitError(e instanceof Error ? e.message : String(e))
                }
            }
        }

        initialize()
        return () => {
            cancelled = true
            // Stop the heartbeat — the user left the ceremony screen.
            // Session stays in registry so it can be reused on return; the heartbeat
            // restarts when the provider remounts for the same keysCeremonyId.
            const s = sessionRegistry.get(keysCeremonyId!)
            if (s) {
                try {
                    s.stop_heartbeat_daemon()
                } catch {
                    // ignore — session may already be freed
                }
            }
            setSession(null)
            setIsConnected(false)
            setInitError(null)
        }
    }, [
        boardName,
        electionEventId,
        keysCeremonyId,
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
        <HeadlessTrusteeContext.Provider
            value={{session, isConnected, initError, keyMismatchWarning}}
        >
            {children}
        </HeadlessTrusteeContext.Provider>
    )
}
