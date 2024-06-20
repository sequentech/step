// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"

import Keycloak, {KeycloakConfig, KeycloakInitOptions} from "keycloak-js"
import {createContext, useEffect, useState} from "react"
import {isArray, isNull, isString, sleep} from "@sequentech/ui-essentials"
import {IPermissions} from "@/types/keycloak"
import {SettingsContext} from "./SettingsContextProvider"

import {useParams} from "react-router"

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

    /**
     * Check whether the user has permissions for an action or data
     * @param tenantId
     * @param electionEventId
     * @param role
     * @returns
     */
    isAuthorized: (
        checkSuperAdmin: boolean,
        someTenantId: string | null,
        role: IPermissions | IPermissions[]
    ) => boolean

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
    tenantId: "",
    logout: () => {},
    hasRole: () => false,
    getAccessToken: () => undefined,
    isAuthorized: () => false,
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
    const [tenantId, setTenantId] = useState<string>("")
    const sleepSecs = 50
    const bufferSecs = 10

    const params= useParams()

    const createKeycloak = () => {
        if (keycloak) {
            return
        }
        /**
         * KeycloakConfig configures the connection to the Keycloak server.
         */
        const keycloakConfig: KeycloakConfig = {
            realm: `tenant-${globalSettings.DEFAULT_TENANT_ID}`,
            clientId: globalSettings.ONLINE_VOTING_CLIENT_ID,
            url: globalSettings.KEYCLOAK_URL,
        }

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
        createKeycloak()
    }, [loaded, keycloak])

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

    const isAuthorized = (
        checkSuperAdmin: boolean,
        someTenantId: string | null,
        role: string | string[]
    ): boolean => {
        const isSuperAdmin = globalSettings.DEFAULT_TENANT_ID === tenantId
        const isValidTenant = tenantId === someTenantId
        if (!((checkSuperAdmin && isSuperAdmin) || (!isNull(someTenantId) && isValidTenant))) {
            return false
        }
        const roleList: string[] = isString(role) ? [role] : role
        return roleList.find((roleItem) => hasRole(roleItem)) != undefined
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
                isAuthenticated,
                userId,
                username,
                email,
                firstName,
                tenantId,
                logout,
                hasRole,
                getAccessToken,
                isAuthorized,
                openProfileLink,
            }}
        >
            {props.children}
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
