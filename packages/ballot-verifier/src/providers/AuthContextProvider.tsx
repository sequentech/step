// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"

import Keycloak, {KeycloakConfig, KeycloakInitOptions} from "keycloak-js"
import {createContext, useEffect, useState} from "react"
import {sleep} from "@sequentech/ui-core"
import {SettingsContext} from "./SettingsContextProvider"

/**
 * AuthContextValues defines the structure for the default values of the {@link AuthContext}.
 */
export interface AuthContextValues {
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
    logout: () => void
    /**
     * Check if the user has the given role
     */
    hasRole: (role: string) => boolean
    /**
     * Get Access Token
     */
    getAccessToken: () => string | undefined

    login: (tenantId: string, eventId: string) => void

    /**
     * Open accountManagement from Keycloak
     * @returns
     */
    openProfileLink: () => Promise<void>
}

/**
 * Default values for the {@link AuthContext}
 */
const defaultAuthContextValues: AuthContextValues = {
    isAuthenticated: false,
    userId: "",
    username: "",
    email: "",
    firstName: "",
    logout: () => {},
    login: (tenantId: string, eventId: string) => {},
    hasRole: () => false,
    getAccessToken: () => undefined,
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
    console.log("rendering AuthContextProvider")
    const {loaded, globalSettings} = useContext(SettingsContext)
    const [keycloak, setKeycloak] = useState<Keycloak | null>()
    const [isKeycloakInitialized, setIsKeycloakInitialized] = useState<boolean>(false)

    // Create the local state in which we will keep track if a user is authenticated
    const [isAuthenticated, setAuthenticated] = useState<boolean>(false)
    // Local state that will contain the users name once it is loaded
    const [userId, setUserId] = useState<string>("")
    const [username, setUsername] = useState<string>("")
    const [email, setEmail] = useState<string>("")
    const [firstName, setFirstName] = useState<string>("")
    const [tenantId, setTenantId] = useState<string | null>(null)
    const [eventId, setEventId] = useState<string | null>(null)
    const sleepSecs = 50
    const bufferSecs = 10

    const createKeycloak = () => {
        if (keycloak) {
            return
        }
        if (!tenantId || !eventId) {
            console.log("Received empty tenant or event id, ignoring..")
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
        setKeycloak(newKeycloak)
    }

    const initializeKeycloak = async () => {
        console.log("initialize Keycloak")
        if (!keycloak) {
            console.log("CAN'T initialize Keycloak")
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
                console.log("user is not yet authenticated. forwarding user to login.")
                await keycloak.login()
            }
            if (!keycloak.token) {
                console.log("error authenticating user")
                console.log("error initializing Keycloak")
                setAuthenticated(false)
                return
            }
            // If we get here the user is authenticated and we can update the state accordingly
            localStorage.setItem("token", keycloak.token)
            setAuthenticated(true)
            setTimeout(updateTokenPeriodically, 4e3)
            console.log("user is authenticated")
            setIsKeycloakInitialized(true)
        } catch (error) {
            console.log("error initializing Keycloak")
            console.log(error)
            setAuthenticated(false)
        }
    }

    useEffect(() => {
        if (keycloak || !loaded) {
            return
        }
        if (!tenantId || !eventId) {
            console.log("Received empty tenant or event id, ignoring..")
            return
        }
        if (isAuthenticated) {
            return
        }
        createKeycloak()
    }, [loaded, tenantId, eventId, keycloak])

    useEffect(() => {
        if (!keycloak || isKeycloakInitialized) {
            return
        }
        initializeKeycloak()
    }, [isKeycloakInitialized, keycloak])

    const updateTokenPeriodically = async () => {
        if (keycloak) {
            const refreshed = await keycloak.updateToken(sleepSecs + bufferSecs)
            if (!keycloak.token) {
                console.log(`error updating token`)
                return
            }
            if (refreshed) {
                localStorage.setItem("token", keycloak.token)
            }
        }
        await sleep(sleepSecs * 1e3)
        updateTokenPeriodically()
    }

    // This effect loads the users profile in order to extract the username
    useEffect(() => {
        /**
         * Load the profile for of the user from Keycloak
         */
        async function loadProfile() {
            if (!keycloak) {
                return
            }
            try {
                const profile = await keycloak.loadUserProfile()

                if (profile.id) {
                    setUserId(profile.id)
                }
                if (profile.email) {
                    setEmail(profile.email)
                }
                if (profile.firstName) {
                    setFirstName(profile.firstName)
                }
                if (profile.username) {
                    setUsername(profile.username)
                }

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
        if (isAuthenticated) {
            loadProfile()
        }
    }, [isAuthenticated, keycloak])

    /**
     * Initiate the logout
     */
    const logout = () => {
        if (!keycloak) {
            return
        }
        localStorage.removeItem("token")
        keycloak.logout()
    }

    const login = (tenantId: string, eventId: string) => {
        setTenantId(tenantId)
        setEventId(eventId)
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

    const getAccessToken = () => keycloak?.token

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
                isAuthenticated,
                userId,
                username,
                email,
                firstName,
                logout,
                login,
                hasRole,
                getAccessToken,
                openProfileLink,
            }}
        >
            {props.children}
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
