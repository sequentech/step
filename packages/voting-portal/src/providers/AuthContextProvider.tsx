// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {StartupError} from "../components/StartupError"
import React, {useContext} from "react"

import Keycloak, {KeycloakConfig, KeycloakInitOptions} from "keycloak-js"
import {createContext, useEffect, useState} from "react"
import {getValueFromCookie, sleep, toBCP47, USER_LANGUAGE_COOKIE_NAME} from "@sequentech/ui-core"
import {SettingsContext} from "./SettingsContextProvider"
import {getLanguageFromURL} from "../utils/queryParams"
import {useTranslation} from "react-i18next"
import {IPermissions} from "../types/keycloak"
import {appendLoginHints, enrollmentRedirectUrl, LoginHints} from "../utils/loginHints"
import {getLogoutRedirectUrl, isKioskClientId} from "../utils/logoutRedirect"
import {getKioskAwareUrl, getKioskPortalRedirectUrl} from "../utils/kioskUrls"

/**
 * AuthContextValues defines the structure for the default values of the {@link AuthContext}.
 */
// The Keycloak required action that verifies a new channel and replaces the voter's old one.
export const CONTACT_CHANGE_ACTION = "message-channel-ra"
// It asks for a recent sign-in: older than this, Keycloak signs the voter in again first.
export const CONTACT_CHANGE_MAX_AGE_SECONDS = 300

// What Keycloak reports when the voter comes back from the contact change.
export enum ContactChangeOutcome {
    Success = "success",
    Cancelled = "cancelled",
    Error = "error",
}

/** The outcome of the contact change, or none for another required action. */
export const contactChangeOutcomeOf = (
    status: "success" | "cancelled" | "error",
    action: string | undefined
): ContactChangeOutcome | undefined => {
    if (action !== CONTACT_CHANGE_ACTION) {
        return undefined
    }
    switch (status) {
        case "success":
            return ContactChangeOutcome.Success
        case "cancelled":
            return ContactChangeOutcome.Cancelled
        case "error":
            return ContactChangeOutcome.Error
    }
}

/** Keycloak's public list of the channels the event offers for codes. */
export const messagingChannelsUrl = (authServerUrl: string, realm: string): string =>
    `${authServerUrl.replace(/\/+$/, "")}/realms/${encodeURIComponent(realm)}/messaging-channels`

/**
 * Whether the voter can change how they are contacted: only when the event offers at least one
 * channel for codes. Anything else (no channel, an older Keycloak without the endpoint, an error)
 * hides the entry, and the reason is logged once.
 *
 * With the voter's access token Keycloak decides for that voter (their Posts and whether every
 * sign-in code step sends on the voter's channel), as the contact change itself decides (R7-14);
 * without one it answers for the event only.
 */
export async function fetchContactChangeAvailable(
    authServerUrl: string | undefined,
    realm: string | undefined,
    token: string | undefined,
    fetchImpl: typeof fetch = fetch
): Promise<boolean> {
    const hidden = (reason: string) => {
        console.info(`"Change how we contact you" is not offered: ${reason}`)
        return false
    }
    if (!authServerUrl || !realm) {
        return hidden("Keycloak's URL or realm is unknown")
    }
    const url = messagingChannelsUrl(authServerUrl, realm)
    try {
        const headers: Record<string, string> = {Accept: "application/json"}
        if (token) {
            headers.Authorization = `Bearer ${token}`
        }
        const response = await fetchImpl(url, {headers})
        if (response.status !== 200) {
            return hidden(`${url} answered ${response.status}`)
        }
        const body: unknown = await response.json()
        const channels = (body as {code_channels?: unknown} | null)?.code_channels
        if (!Array.isArray(channels)) {
            return hidden(`${url} answered without a code_channels list`)
        }
        if (!channels.some((channel) => typeof channel === "string" && channel !== "")) {
            return hidden("the event offers no channel for codes")
        }
        return true
    } catch (error) {
        return hidden(`${url} could not be read (${String(error)})`)
    }
}

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

    setTenantEvent: (
        tenantId: string,
        eventId: string,
        authType?: "register" | "login",
        defaultLocale?: string,
        loginHints?: LoginHints
    ) => void

    /**
     * Open accountManagement from Keycloak
     * @returns
     */
    openProfileLink: () => Promise<void>

    isGoldUser: () => boolean

    reauthWithGold: (redirectUri: string) => Promise<void>

    /** Starts Keycloak's contact change ({@link CONTACT_CHANGE_ACTION}) after a recent sign-in. */
    changeContactChannel?: () => Promise<void>

    /**
     * Whether the event offers a channel for codes, so the voter can change theirs. Absent
     * (an offline context) means no.
     */
    contactChangeAvailable?: boolean

    /** How the last contact change ended, until the voter dismisses it. */
    contactChangeOutcome?: ContactChangeOutcome

    /** Dismisses {@link contactChangeOutcome}. */
    clearContactChangeOutcome?: () => void
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
    setTenantEvent: (
        _tenantId: string,
        _eventId: string,
        _authType,
        _defaultLocale,
        _loginHints
    ) => {},
    hasRole: () => false,
    isKiosk: () => false,
    openProfileLink: () => new Promise(() => undefined),
    isGoldUser: () => false,
    reauthWithGold: async () => {},
    contactChangeAvailable: false,
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
    children: React.ReactNode
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
    const [initializationFailed, setInitializationFailed] = useState(false)
    const [isAuthenticated, setAuthenticated] = useState<boolean>(false)

    // Local state that will contain the users name once it is loaded
    const [userProfile, setUserProfile] = useState<UserProfile | null>(null)

    const [tenantId, setTenantId] = useState<string | null>(null)
    const [eventId, setEventId] = useState<string | null>(null)
    const [authType, setAuthType] = useState<"register" | "login" | null>(null)
    const [defaultLocale, setDefaultLocale] = useState<string | undefined>(undefined)
    const [loginHints, setLoginHints] = useState<LoginHints>({})
    const [contactChangeAvailable, setContactChangeAvailable] = useState<boolean>(false)
    const [contactChangeOutcome, setContactChangeOutcome] = useState<
        ContactChangeOutcome | undefined
    >(undefined)

    const {i18n} = useTranslation()

    useEffect(() => {
        function createKeycloak() {
            if (keycloak || !tenantId || !eventId) {
                return
            }

            const isKiosk = new URLSearchParams(window.location.search).has("kiosk")
            const kioskPortalRedirectUrl = getKioskPortalRedirectUrl(
                window.location.href,
                globalSettings.KIOSK_VOTING_PORTAL_URL,
                isKiosk
            )
            if (kioskPortalRedirectUrl) {
                window.location.replace(kioskPortalRedirectUrl)
                return
            }

            /**
             * Get the voting client. If there's a param `kiosk` in the URL, it
             * append `-kiosk` to the
             */
            const getClientId: (defaultClientId: string) => string = (defaultClientId) => {
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
                getKioskAwareUrl(
                    globalSettings.KEYCLOAK_URL,
                    globalSettings.KIOSK_KEYCLOAK_URL,
                    isKiosk
                ),
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
                    redirectUri: getLogoutRedirectUrl({
                        origin: window.location.origin,
                        pathname: window.location.pathname,
                        tenantId,
                        eventId,
                        clientId: newKeycloak.clientId,
                        defaultClientId: globalSettings.ONLINE_VOTING_CLIENT_ID,
                        hasKioskQuery: new URLSearchParams(window.location.search).has("kiosk"),
                    }),
                })
            }

            // Keycloak reports a required action's outcome when it redirects back, during init.
            newKeycloak.onActionUpdate = (status, action) => {
                const outcome = contactChangeOutcomeOf(status, action)
                if (outcome !== undefined) {
                    console.info(`The contact change ended: ${outcome}`)
                    setContactChangeOutcome(outcome)
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
        globalSettings.KIOSK_KEYCLOAK_URL,
        globalSettings.KIOSK_VOTING_PORTAL_URL,
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
                const rawLocale =
                    getLanguageFromURL() ||
                    getValueFromCookie(USER_LANGUAGE_COOKIE_NAME) ||
                    defaultLocale
                const keycloakInitOptions: KeycloakInitOptions = {
                    // Configure that Keycloak will check if a user is already authenticated (when
                    // opening the app or reloading the page). If not authenticated the user will
                    // be send to the login form. If already authenticated the webapp will open.
                    checkLoginIframe: false,
                    // Keycloak expects BCP 47 locale codes (e.g. "ca" for Catalan, not "cat")
                    locale: rawLocale ? toBCP47(rawLocale) : undefined,
                }
                const isAuthenticatedResponse = await keycloak.init(keycloakInitOptions)

                // If the authentication was not successfull the user is send back to the Keycloak login form
                if (!isAuthenticatedResponse && authType) {
                    if (authType === "register") {
                        const baseUrl = window.location.origin + window.location.pathname
                        const queryString = window.location.search
                        const registerOptions = {
                            ...keycloakInitOptions,
                            // after successful enrollment, we should redirect to login
                            redirectUri: enrollmentRedirectUrl(baseUrl, queryString),
                            loginHint: loginHints.username,
                        }

                        if (Object.keys(loginHints).length > 0) {
                            const registerUrl = await keycloak.createRegisterUrl(registerOptions)
                            window.location.assign(appendLoginHints(registerUrl, loginHints))
                            return
                        }

                        return await keycloak.register(registerOptions)
                    } else {
                        if (Object.keys(loginHints).length > 0) {
                            const loginUrl = await keycloak.createLoginUrl({
                                ...keycloakInitOptions,
                                // Stock username forms only understand the standard OIDC hint.
                                loginHint: loginHints.username,
                            })
                            window.location.assign(appendLoginHints(loginUrl, loginHints))
                            return
                        }

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
                setInitializationFailed(true)
                console.log("error initializing Keycloak")
                console.log(error)
                setAuthenticated(false)
            }
        }

        if (keycloak && !isAuthenticated && !isKeycloakInitialized) {
            initializeKeycloak()
        }
    }, [keycloak, isAuthenticated, isKeycloakInitialized, authType, defaultLocale, loginHints])

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
                locale: toBCP47(i18n.language),
            })
        } catch (error) {
            console.error("Re-authentication failed:", error)
        }
    }

    const changeContactChannel = async (): Promise<void> => {
        try {
            await keycloak?.login({
                action: CONTACT_CHANGE_ACTION,
                maxAge: CONTACT_CHANGE_MAX_AGE_SECONDS,
                redirectUri: window.location.href,
                locale: toBCP47(i18n.language),
            })
        } catch (error) {
            console.error("The contact change could not start:", error)
        }
    }

    // The contact change is offered only when the event has a channel for codes.
    useEffect(() => {
        if (!keycloak || !isAuthenticated || !isKeycloakInitialized) {
            return
        }
        let current = true
        fetchContactChangeAvailable(keycloak.authServerUrl, keycloak.realm, keycloak.token).then(
            (available) => {
                if (current) {
                    setContactChangeAvailable(available)
                }
            }
        )
        return () => {
            current = false
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

    const setTenantEvent = (
        tenantId: string,
        eventId: string,
        authType?: "register" | "login",
        defaultLocale?: string,
        initialLoginHints?: LoginHints
    ) => {
        setTenantId(tenantId)
        setEventId(eventId)
        setDefaultLocale(defaultLocale)
        authType && setAuthType(authType)
        setLoginHints(initialLoginHints ?? {})
    }

    const getRedirectUrl = (redirectUrl?: string) => {
        return getLogoutRedirectUrl({
            redirectUrl,
            origin: window.location.origin,
            pathname: window.location.pathname,
            tenantId,
            eventId,
            clientId: keycloak?.clientId,
            defaultClientId: globalSettings.ONLINE_VOTING_CLIENT_ID,
            hasKioskQuery: new URLSearchParams(window.location.search).has("kiosk"),
        })
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
        const clientId = keycloak?.clientId ?? keycloak?.tokenParsed?.azp
        return isKioskClientId(clientId, globalSettings.ONLINE_VOTING_CLIENT_ID)
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
                changeContactChannel,
                contactChangeAvailable,
                contactChangeOutcome,
                clearContactChangeOutcome: () => setContactChangeOutcome(undefined),
            }}
        >
            {initializationFailed ? <StartupError /> : props.children}
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
