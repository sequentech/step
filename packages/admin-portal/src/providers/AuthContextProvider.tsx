// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Keycloak, {KeycloakConfig, KeycloakInitOptions} from "keycloak-js"
import {createContext, useEffect, useState} from "react"
import {sleep} from "@sequentech/ui-essentials"

const DEFAULT_TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"

/**
 * KeycloakConfig configures the connection to the Keycloak server.
 */
const keycloakConfig: KeycloakConfig = {
    realm: `tenant-${DEFAULT_TENANT}`,
    clientId: "admin-portal",
    url: "http://127.0.0.1:8090/",
}

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

// Create the Keycloak client instance
const keycloak = new Keycloak(keycloakConfig)

/**
 * AuthContextValues defines the structure for the default values of the {@link AuthContext}.
 */
interface AuthContextValues {
    /**
     * Whether or not a user is currently authenticated
     */
    isAuthenticated: boolean
    /**
     * The name of the authenticated user
     */
    username: string
    /**
     * The tenant id of the authenticated user
     */
    tenantId: string
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
}

/**
 * Default values for the {@link AuthContext}
 */
const defaultAuthContextValues: AuthContextValues = {
    isAuthenticated: false,
    username: "",
    tenantId: "",
    logout: () => {},
    hasRole: (role) => false,
    getAccessToken: () => undefined,
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

    // Create the local state in which we will keep track if a user is authenticated
    const [isAuthenticated, setAuthenticated] = useState<boolean>(false)
    // Local state that will contain the users name once it is loaded
    const [username, setUsername] = useState<string>("")
    const [tenantId, setTenantId] = useState<string>("")
    const sleepSecs = 50
    const bufferSecs = 10

    const updateTokenPeriodically = async () => {
        const refreshed = await keycloak.updateToken(sleepSecs + bufferSecs)
        if (!keycloak.token) {
            console.log(`error updating token`)
            return
        }
        if (refreshed) {
            localStorage.setItem("token", keycloak.token)
        }
        await sleep(sleepSecs * 1e3)
        updateTokenPeriodically()
    }

    // Effect used to initialize the Keycloak client. It has no dependencies so it is only rendered when the app is (re-)loaded.
    useEffect(() => {
        /**
         * Initialize the Keycloak instance
         */
        async function initializeKeycloak() {
            console.log("initialize Keycloak")
            try {
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
            } catch (error) {
                console.log("error initializing Keycloak")
                console.log(error)
                setAuthenticated(false)
            }
        }

        initializeKeycloak()
    }, [])

    // This effect loads the users profile in order to extract the username
    useEffect(() => {
        /**
         * Load the profile for of the user from Keycloak
         */
        async function loadProfile() {
            try {
                const profile = await keycloak.loadUserProfile()
                if (profile.firstName) {
                    setUsername(profile.firstName)
                } else if (profile.username) {
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
    }, [isAuthenticated])

    /**
     * Initiate the logout
     */
    const logout = () => {
        localStorage.removeItem("token")
        keycloak.logout()
    }

    /**
     * Check if the user has the given role
     * @param role to be checked
     * @returns whether or not if the user has the role
     */
    const hasRole = (role: string) => {
        return keycloak.hasRealmRole(role)
    }

    const getAccessToken = () => keycloak.token

    // Setup the context provider
    return (
        <AuthContext.Provider
            value={{isAuthenticated, username, tenantId, logout, hasRole, getAccessToken}}
        >
            {props.children}
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
