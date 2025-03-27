// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"

import Keycloak, {KeycloakConfig, KeycloakInitOptions} from "keycloak-js"
import {createContext, useEffect, useState} from "react"
import {sleep} from "@sequentech/ui-core"
import {SettingsContext} from "./SettingsContextProvider"
import {getLanguageFromURL} from "../utils/queryParams"
import {IPermissions} from "../types/keycloak"

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
     * Is the user in kiosk mode
     */
    isKiosk: () => boolean

    getExpiry: () => Date | undefined

    /**
     * Keycloak access token
     */
    keycloakAccessToken: string | undefined

    setTenantEvent: (tenantId: string, eventId: string, authType?: "register" | "login") => void

    /**
     * Open accountManagement from Keycloak
     * @returns
     */
    openProfileLink: () => Promise<void>

    isGoldUser: () => boolean

    reauthWithGold: (redirectUri: string) => Promise<void>
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
    getExpiry: () => undefined,
    setTenantEvent: (_tenantId: string, _eventId: string) => {},
    hasRole: () => false,
    isKiosk: () => false,
    openProfileLink: () => new Promise(() => undefined),
    isGoldUser: () => false,
    reauthWithGold: async () => {},
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
    const [authType, setAuthType] = useState<"register" | "login" | null>(null)

    useEffect(() => {
        function createKeycloak() {
            if (keycloak || !tenantId || !eventId) {
                return
            }

            /**
             * Get the Keycloak URL. If there's a param `kiosk` in the URL, it
             * appends `-kiosk` to the subdomain (if it exists).
             */
            const getKeycloakUrl: (defaultUrl: string) => string = (defaultUrl) => {
                const searchParams = new URLSearchParams(window.location.search)
                const isKiosk = searchParams.has("kiosk")

                if (!isKiosk) {
                    return defaultUrl
                }

                try {
                    const url = new URL(defaultUrl)
                    const subdomainParts = url.hostname.split(".")

                    // Only modify if there is a subdomain
                    if (subdomainParts.length > 2) {
                        subdomainParts[0] += "-kiosk"
                        url.hostname = subdomainParts.join(".")
                    }

                    return url.toString()
                } catch (error) {
                    console.error("Invalid URL provided:", defaultUrl)
                    return defaultUrl // Fallback to the original URL if an error occurs
                }
            }

            /**
             * Get the voting client. If there's a param `kiosk` in the URL, it
             * append `-kiosk` to the
             */
            const getClientId: (defaultClientId: string) => string = (defaultClientId) => {
                const searchParams = new URLSearchParams(window.location.search)
                const isKiosk = searchParams.has("kiosk")
                return isKiosk ? `${defaultClientId}-kiosk` : defaultClientId
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
                getKeycloakUrl(globalSettings.KEYCLOAK_URL),
                getClientId(globalSettings.ONLINE_VOTING_CLIENT_ID)
            )

            // Create the Keycloak client instance
            const newKeycloak = new Keycloak(keycloakConfig)

            newKeycloak.onTokenExpired = async () => {
                /*const refreshed = await newKeycloak.updateToken(0)

                if (refreshed) {
                    setKeycloakAccessToken(newKeycloak.token)
                } else {
                    newKeycloak.logout()
                }*/
                newKeycloak.logout({
                    redirectUri: `/tenant/${tenantId}/event/${eventId}/`,
                })
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
                    checkLoginIframe: false,
                    locale: getLanguageFromURL(),
                }
                const isAuthenticatedResponse = await keycloak.init(keycloakInitOptions)

                // If the authentication was not successfull the user is send back to the Keycloak login form
                if (!isAuthenticatedResponse && authType) {
                    if (authType === "register") {
                        const baseUrl = window.location.origin + window.location.pathname
                        const queryString = window.location.search

                        return await keycloak.register({
                            ...keycloakInitOptions,
                            // after successful enrollment, we should redirect to login
                            redirectUri: baseUrl.endsWith("/enroll")
                                ? baseUrl.replace(/\/enroll$/, "/login") + queryString
                                : undefined,
                        })
                    } else {
                        return await keycloak.login(keycloakInitOptions)
                    }
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
    }, [keycloak, isAuthenticated, isKeycloakInitialized, authType])

    /**
     * Returns true only if the JWT has gold permissions and the JWT
     * authentication is fresh, i.e. performed less than 60 seconds ago.
     */
    // TODO: This is duplicated from jwt.rs in sequent-core, we should just use
    // the same WASM function if possible
    const isGoldUser = () => {
        const acr = keycloak?.tokenParsed?.acr ?? null
        const isGold = acr === IPermissions.GOLD_PERMISSION

        const authTimeTimestamp = keycloak?.tokenParsed?.auth_time ?? 0
        const authTime = new Date(authTimeTimestamp * 1000)
        const freshnessLimit = new Date(Date.now().valueOf() - 60 * 1000)
        const isFresh = authTime > freshnessLimit
        return isGold && isFresh
    }

    const reauthWithGold = async (redirectUri: string): Promise<void> => {
        try {
            await keycloak?.login({
                acr: {essential: true, values: [IPermissions.GOLD_PERMISSION]},
                redirectUri: redirectUri || window.location.href, // Use the passed URL or fallback to current URL
            })
        } catch (error) {
            console.error("Re-authentication failed:", error)
        }
    }

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

    const setTenantEvent = (tenantId: string, eventId: string, authType?: "register" | "login") => {
        setTenantId(tenantId)
        setEventId(eventId)
        authType && setAuthType(authType)
    }

    const getRedirectUrl = (redirectUrl?: string) => {
        if (redirectUrl) {
            return redirectUrl
        } else {
            const currentPath = window.location.pathname
            const pathSegments = currentPath.split("/")
            while (pathSegments.length > 5) {
                pathSegments.pop() // Remove the last segment (To only keep the teanant and event params)
            }
            return pathSegments.join("/")
        }
    }

    const logout = (redirectUrl?: string) => {
        if (!keycloak) {
            // If no keycloak object initailized manually clear cookies and redirect user
            clearAllCookies()
            window.location.href = getRedirectUrl(redirectUrl)
            return
        }

        keycloak.logout({
            redirectUri: getRedirectUrl(redirectUrl),
        })
    }

    const clearAllCookies = () => {
        document.cookie.split(";").forEach((cookie) => {
            const eqPos = cookie.indexOf("=")
            const name = eqPos > -1 ? cookie.substring(0, eqPos) : cookie
            document.cookie = name + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT"
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
    /**
     * Check if the user is in kiosk mode
     * @returns whether or not the user in kiosk mode
     */
    const isKiosk = () => {
        if (!keycloak?.tokenParsed?.azp) {
            return false
        }

        return keycloak.tokenParsed.azp.endsWith("-kiosk")
    }

    const openProfileLink = async () => {
        if (!keycloak) {
            return
        }

        await keycloak.accountManagement()
    }

    const getExpiry = () => {
        let exp = keycloak?.tokenParsed?.exp
        return exp ? new Date(exp * 1000) : undefined
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
                getExpiry,
                logout,
                hasRole,
                isKiosk,
                openProfileLink,
                keycloakAccessToken,
                isGoldUser,
                reauthWithGold,
            }}
        >
            {props.children}
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
