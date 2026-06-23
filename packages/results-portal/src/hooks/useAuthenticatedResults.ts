// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useMemo, useState} from "react"
import Keycloak from "keycloak-js"
import {GlobalSettings} from "@/providers/SettingsContextProvider"

interface AuthState {
    loading: boolean
    token?: string
    error?: string
}

interface AuthSession {
    keycloak: Keycloak
    initPromise?: Promise<AuthState>
}

const authSessions = new Map<string, AuthSession>()

const getAuthSession = (
    sessionKey: string,
    settings: GlobalSettings,
    realm: string
): AuthSession => {
    const existing = authSessions.get(sessionKey)
    if (existing) {
        return existing
    }

    const session = {
        keycloak: new Keycloak({
            url: settings.KEYCLOAK_URL,
            realm,
            clientId: settings.ONLINE_VOTING_CLIENT_ID,
        }),
    }
    authSessions.set(sessionKey, session)
    return session
}

const authenticateSession = (session: AuthSession): Promise<AuthState> => {
    if (session.initPromise) {
        return session.initPromise
    }

    session.initPromise = (async () => {
        try {
            const authenticated = await session.keycloak.init({
                onLoad: "login-required",
                checkLoginIframe: false,
                flow: "standard",
                responseMode: "fragment",
            })

            if (!authenticated || !session.keycloak.token) {
                return {
                    loading: false,
                    error: "Authentication failed",
                }
            }

            await session.keycloak.updateToken(30).catch(() => undefined)

            return {
                loading: false,
                token: session.keycloak.token,
            }
        } catch (error) {
            session.initPromise = undefined
            return {
                loading: false,
                error: error instanceof Error ? error.message : "Authentication failed",
            }
        }
    })()

    return session.initPromise
}

const sessionTokenState = (sessionKey?: string): AuthState => {
    const token = sessionKey ? authSessions.get(sessionKey)?.keycloak.token : undefined

    return {
        loading: false,
        token,
    }
}

export const useAuthenticatedResults = (
    settings: GlobalSettings,
    tenantId?: string,
    eventId?: string,
    required = false
): AuthState => {
    const [state, setState] = useState<AuthState>({loading: required})

    const realm = useMemo(
        () => (tenantId && eventId ? `tenant-${tenantId}-event-${eventId}` : undefined),
        [tenantId, eventId]
    )

    useEffect(() => {
        const sessionKey = realm
            ? [settings.KEYCLOAK_URL, realm, settings.ONLINE_VOTING_CLIENT_ID].join("|")
            : undefined

        if (!required || settings.DISABLE_AUTH) {
            setState(sessionTokenState(sessionKey))
            return
        }

        if (!realm) {
            setState({loading: true})
            return
        }

        let mounted = true
        const session = getAuthSession(sessionKey, settings, realm)

        const init = async () => {
            const nextState = await authenticateSession(session)
            if (mounted) {
                setState(nextState)
            }
        }

        void init()

        return () => {
            mounted = false
        }
    }, [
        required,
        realm,
        settings.DISABLE_AUTH,
        settings.KEYCLOAK_URL,
        settings.ONLINE_VOTING_CLIENT_ID,
    ])

    return state
}
