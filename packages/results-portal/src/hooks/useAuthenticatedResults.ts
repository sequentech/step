// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useMemo, useState} from "react"
import Keycloak, {KeycloakProfile} from "keycloak-js"
import {GlobalSettings} from "@/providers/SettingsContextProvider"
import {ResultsUserProfile} from "@/providers/ResultsAuthContextProvider"

interface AuthState {
    loading: boolean
    token?: string
    userProfile?: ResultsUserProfile
    logout?: () => void
    error?: string
}

interface AuthSession {
    keycloak: Keycloak
    initPromise?: Promise<void>
    userProfile?: ResultsUserProfile
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
            clientId: settings.RESULTS_PORTAL_CLIENT_ID,
        }),
    }
    authSessions.set(sessionKey, session)
    return session
}

const tokenClaim = (keycloak: Keycloak, name: string): string | undefined => {
    const value = (keycloak.tokenParsed as Record<string, unknown> | undefined)?.[name]
    return typeof value === "string" && value ? value : undefined
}

const openAccountManagement = (keycloak: Keycloak) => {
    void keycloak.accountManagement()
}

const userProfileFromClaims = (keycloak: Keycloak): ResultsUserProfile => ({
    firstName: tokenClaim(keycloak, "given_name") ?? tokenClaim(keycloak, "name"),
    username:
        tokenClaim(keycloak, "preferred_username") ??
        tokenClaim(keycloak, "email") ??
        tokenClaim(keycloak, "sub") ??
        "user",
    email: tokenClaim(keycloak, "email"),
    openLink: () => openAccountManagement(keycloak),
})

const userProfileFromKeycloak = (
    profile: KeycloakProfile,
    keycloak: Keycloak
): ResultsUserProfile => {
    const fallbackProfile = userProfileFromClaims(keycloak)

    return {
        firstName: profile.firstName ?? fallbackProfile.firstName,
        username: profile.username ?? profile.email ?? fallbackProfile.username,
        email: profile.email ?? fallbackProfile.email,
        openLink: fallbackProfile.openLink,
    }
}

const loadUserProfile = async (keycloak: Keycloak): Promise<ResultsUserProfile> => {
    try {
        return userProfileFromKeycloak(await keycloak.loadUserProfile(), keycloak)
    } catch {
        return userProfileFromClaims(keycloak)
    }
}

const logoutSession = (keycloak: Keycloak) => {
    void keycloak.logout({
        redirectUri: window.location.href.split("#")[0],
    })
}

const authStateFromSession = async (session: AuthSession): Promise<AuthState> => {
    const token = session.keycloak.token
    if (!token) {
        return {
            loading: false,
        }
    }

    return {
        loading: false,
        token,
        userProfile:
            session.userProfile ?? (session.userProfile = await loadUserProfile(session.keycloak)),
        logout: () => logoutSession(session.keycloak),
    }
}

const authenticateSession = async (
    sessionKey: string,
    session: AuthSession
): Promise<AuthState> => {
    try {
        if (!session.initPromise) {
            session.initPromise = (async () => {
                const authenticated = await session.keycloak.init({
                    onLoad: "login-required",
                    checkLoginIframe: false,
                    flow: "standard",
                    responseMode: "fragment",
                    redirectUri: window.location.href.split("#")[0],
                })

                if (!authenticated || !session.keycloak.token) {
                    throw new Error("Authentication failed")
                }
            })()
        }

        await session.initPromise
        await session.keycloak.updateToken(30)

        return authStateFromSession(session)
    } catch (error) {
        if (authSessions.get(sessionKey) === session) {
            authSessions.delete(sessionKey)
        }
        return {
            loading: false,
            error: error instanceof Error ? error.message : "Authentication failed",
        }
    }
}

const sessionTokenState = (sessionKey?: string): AuthState => {
    const session = sessionKey ? authSessions.get(sessionKey) : undefined
    const token = session?.keycloak.token

    return {
        loading: false,
        token,
        logout: session ? () => logoutSession(session.keycloak) : undefined,
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
            ? [settings.KEYCLOAK_URL, realm, settings.RESULTS_PORTAL_CLIENT_ID].join("|")
            : undefined

        if (!required) {
            setState(sessionTokenState(sessionKey))
            return
        }

        if (!realm || !sessionKey) {
            setState({loading: true})
            return
        }

        let mounted = true
        const session = getAuthSession(sessionKey, settings, realm)
        let refreshInterval: number | undefined

        const updateStateFromSession = async () => {
            try {
                await session.keycloak.updateToken(60)
                const nextState = await authStateFromSession(session)
                if (mounted) {
                    setState(nextState)
                }
            } catch (error) {
                if (authSessions.get(sessionKey) === session) {
                    authSessions.delete(sessionKey)
                }
                if (mounted) {
                    setState({
                        loading: false,
                        error: error instanceof Error ? error.message : "Authentication failed",
                    })
                }
            }
        }

        const init = async () => {
            setState({loading: true})
            const nextState = await authenticateSession(sessionKey, session)
            if (mounted) {
                setState(nextState)
                if (nextState.token) {
                    session.keycloak.onTokenExpired = () => {
                        void updateStateFromSession()
                    }
                    refreshInterval = window.setInterval(() => {
                        void updateStateFromSession()
                    }, 30_000)
                }
            }
        }

        void init()

        return () => {
            mounted = false
            if (refreshInterval !== undefined) {
                window.clearInterval(refreshInterval)
            }
            session.keycloak.onTokenExpired = undefined
        }
    }, [required, realm, settings.KEYCLOAK_URL, settings.RESULTS_PORTAL_CLIENT_ID])

    return state
}
