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
        userProfile: await loadUserProfile(session.keycloak),
        logout: () => logoutSession(session.keycloak),
    }
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

            return authStateFromSession(session)
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
            ? [settings.KEYCLOAK_URL, realm, settings.ONLINE_VOTING_CLIENT_ID].join("|")
            : undefined

        if (!required || settings.DISABLE_AUTH) {
            setState(sessionTokenState(sessionKey))
            return
        }

        if (!realm || !sessionKey) {
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
