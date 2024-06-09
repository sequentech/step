// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"

import Keycloak, {KeycloakConfig, KeycloakInitOptions} from "keycloak-js"
import {createContext, useEffect, useState} from "react"
import {sleep} from "@sequentech/ui-essentials"
import {SettingsContext} from "./SettingsContextProvider"

/**
 * AuthContextValues defines the structure for the default values of the {@link AuthContext}.
 */
export interface AuthContextValues {
    /**
     * Is Auth Context has been fully initialized
     */
    isAuthContextInitialized: boolean
    /**
     * Whether or not a user is currently authenticated
     */
    isAuthenticated: boolean
    /**
     * The id of the authenticated user
     */
    userId: string
    /**
     * The user name of the authenticated user
     */
    username: string
    /**
     * The email of the authenticated user
     */
    email: string
    /**
     * The first name of the authenticated user
     */
    firstName: string

    /**
     * Function to initiate the logout
     */
    logout: (redirectUrl?: string) => void

    /**
     * Check if the user has the given role
     */
    hasRole: (role: string) => boolean

    /**
     * Keycloak access token
     */
    keycloakAccessToken: string | undefined

    setTenantEvent: (tenantId: string, eventId: string) => void

    /**
     * Open accountManagement from Keycloak
     * @returns
     */
    openProfileLink: () => Promise<void>
}

interface UserProfile {
    userId?: string
    username?: string
    email?: string
    firstName?: string
}

/**
 * Default values for the {@link AuthContext}
 */
const defaultAuthContextValues: AuthContextValues = {
    isAuthContextInitialized: false,
    isAuthenticated: false,
    userId: "",
    username: "",
    email: "",
    firstName: "",
    keycloakAccessToken: undefined,
    logout: () => {},
    setTenantEvent: (_tenantId: string, _eventId: string) => {},
    hasRole: () => false,
    openProfileLink: () => new Promise(() => undefined),
}

/**
 * Create the AuthContext using the default values.
 */
export const AuthContext = createContext<AuthContextValues>(defaultAuthContextValues)

/**
 * The props that must be passed to create the {@link AuthContextProvider}.
 */
interface AuthContextProviderProps {
    /**
     * The elements wrapped by the auth context.
     */
    children: JSX.Element
}

/**
 * AuthContextProvider is responsible for managing the authentication state of the current user.
 *
 * @param props
 */
const AuthContextProvider = (props: AuthContextProviderProps) => {
    const {loaded, globalSettings} = useContext(SettingsContext)
    const [keycloak, setKeycloak] = useState<Keycloak | null>()
    const [isKeycloakInitialized, setIsKeycloakInitialized] = useState<boolean>(false)
    const [keycloakAccessToken, setKeycloakAccessToken] = useState<string | undefined>()

    // Create the local state in which we will keep track if a user is authenticated
    const [isAuthenticated, setAuthenticated] = useState<boolean>(false)

    // Local state that will contain the users name once it is loaded
    const [userProfile, setUserProfile] = useState<UserProfile | null>(null)

    const [tenantId, setTenantId] = useState<string | null>(null)
    const [eventId, setEventId] = useState<string | null>(null)

    useEffect(() => {
        function createKeycloak() {
            if (keycloak || !tenantId || !eventId) {
                return
            }

            /**
             * KeycloakConfig configures the connection to the Keycloak server.
             */
            const createKeycloakConfig: (
                tenantId: string,
                eventId: string,
                keycloakUrl: string,
                clientId: string
            ) => KeycloakConfig = (tenantId, eventId, keycloakUrl, clientId) => {
                return {
                    realm: `tenant-${tenantId}-event-${eventId}`,
                    url: keycloakUrl,
                    clientId: clientId,
                }
            }
            const keycloakConfig = createKeycloakConfig(
                tenantId,
                eventId,
                globalSettings.KEYCLOAK_URL,
                globalSettings.ONLINE_VOTING_CLIENT_ID
            )

            // Create the Keycloak client instance
            const newKeycloak = new Keycloak(keycloakConfig)

            newKeycloak.onTokenExpired = async () => {
                const refreshed = await newKeycloak.updateToken(0)

                if (refreshed) {
                    setKeycloakAccessToken(newKeycloak.token)
                } else {
                    newKeycloak.logout()
                }
            }

            setKeycloak(newKeycloak)
        }

        if (!keycloak && loaded && tenantId && eventId) {
            createKeycloak()
        }
    }, [
        tenantId,
        eventId,
        keycloak,
        setKeycloak,
        loaded,
        globalSettings.KEYCLOAK_URL,
        globalSettings.ONLINE_VOTING_CLIENT_ID,
    ])

    useEffect(() => {
        async function updateTokenPeriodically() {
            const tokenLifespan = globalSettings.KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS
            const bufferSecs = 10
            const minValidity = tokenLifespan - bufferSecs

            if (keycloak) {
                const refreshed = await keycloak.updateToken(minValidity)

                if (refreshed) {
                    setKeycloakAccessToken(keycloak.token)
                }
            }

            await sleep(tokenLifespan * 1e3)

            updateTokenPeriodically()
        }

        async function initializeKeycloak() {
            if (!keycloak) {
                return
            }

            try {
                /**
                 * KeycloakInitOptions configures the Keycloak client.
                 */
                const keycloakInitOptions: KeycloakInitOptions = {
                    // Configure that Keycloak will check if a user is already authenticated (when
                    // opening the app or reloading the page). If not authenticated the user will
                    // be send to the login form. If already authenticated the webapp will open.
                    onLoad: "login-required",
                    checkLoginIframe: false,
                }
                const isAuthenticatedResponse = await keycloak.init(keycloakInitOptions)

                // If the authentication was not successfull the user is send back to the Keycloak login form
                if (!isAuthenticatedResponse) {
                    return await keycloak.login()
                }

                if (!keycloak.token) {
                    setAuthenticated(false)
                    return
                }

                setAuthenticated(true)
                setIsKeycloakInitialized(true)
                setKeycloakAccessToken(keycloak.token)
                updateTokenPeriodically()
            } catch (error) {
                console.log("error initializing Keycloak")
                console.log(error)
                setAuthenticated(false)
            }
        }

        if (keycloak && !isAuthenticated && !isKeycloakInitialized) {
            initializeKeycloak()
        }
    }, [keycloak, isAuthenticated, isKeycloakInitialized])

    useEffect(() => {
        async function loadProfile() {
            if (!keycloak) {
                return
            }

            try {
                const profile = await keycloak.loadUserProfile()
                setUserProfile((val) => ({
                    ...val,
                    userId: profile?.id || val?.userId,
                    email: profile?.email || val?.email,
                    firstName: profile?.firstName || val?.firstName,
                    username: profile?.username || val?.username,
                }))

                const newTenantId: string | undefined = (profile as any)?.attributes[
                    "tenant-id"
                ]?.[0]

                if (newTenantId) {
                    setTenantId(newTenantId)
                }
            } catch {
                console.log("error trying to load the users profile")
            }
        }

        // Only load the profile if a user is authenticated
        if (keycloak && isAuthenticated && isKeycloakInitialized) {
            loadProfile()
        }
    }, [keycloak, isAuthenticated, isKeycloakInitialized])

    const setTenantEvent = (tenantId: string, eventId: string) => {
        setTenantId(tenantId)
        setEventId(eventId)
    }

    const logout = (redirectUrl?: string) => {
        if (!keycloak) {
            if (redirectUrl) {
                window.location.href = redirectUrl
            }
            return
        }

        keycloak.logout({
            redirectUri: redirectUrl,
        })
    }

    /**
     * Check if the user has the given role
     * @param role to be checked
     * @returns whether or not if the user has the role
     */
    const hasRole = (role: string) => {
        if (!keycloak) {
            return false
        }

        return keycloak.hasRealmRole(role)
    }

    const openProfileLink = async () => {
        if (!keycloak) {
            return
        }

        await keycloak.accountManagement()
    }

    // Setup the context provider
    return (
        <AuthContext.Provider
            value={{
                isAuthContextInitialized: isKeycloakInitialized,
                isAuthenticated,
                userId: userProfile?.userId ?? "",
                username: userProfile?.username ?? "",
                email: userProfile?.email ?? "",
                firstName: userProfile?.firstName ?? "",
                setTenantEvent,
                logout,
                hasRole,
                openProfileLink,
                keycloakAccessToken,
            }}
        >
            {props.children}
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
