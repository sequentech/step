// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"

import Keycloak, {KeycloakConfig, KeycloakInitOptions} from "keycloak-js"
import {createContext, useEffect, useState} from "react"
import {isNull, isString, sleep} from "@sequentech/ui-core"
import {IPermissions} from "@/types/keycloak"
import {SettingsContext} from "./SettingsContextProvider"
import {useLocation, useNavigate} from "react-router"
import {ExecutionResult} from "graphql"
import {GetAllTenantsQuery} from "@/gql/graphql"
import SelectTenant from "@/screens/SelectTenant"
import {Dialog, IconButton, adminTheme} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"

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
     * The trustee an admin user can act as
     */
    trustee: string

    /**
     * The permission labels an admin user has
     */
    permissionLabels: string[]
    /**
     * Function to initiate the logout
     */
    logout: (redirectUri?: string) => void
    /**
     * Check if the user has the given role
     */
    hasRole: (role: string) => boolean
    /**
     * Get Access Token
     */
    getAccessToken: () => string | undefined

    updateTokenAndPermissionLabels: () => void

    initKeycloak: (tenantId: string) => Promise<boolean>

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

    isGoldUser: () => boolean

    reauthWithGold: (redirectUri: string) => Promise<void>
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
    trustee: "",
    logout: () => {},
    hasRole: () => false,
    getAccessToken: () => undefined,
    isAuthorized: () => false,
    openProfileLink: () => new Promise(() => undefined),
    permissionLabels: [],
    updateTokenAndPermissionLabels: () => {},
    isGoldUser: () => false,
    reauthWithGold: async () => {},
    initKeycloak: async (tenantId: string) => {
        return false
    },
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

const generateTokenStorage = (newKeycloak: Keycloak): string => {
    let nowInSeconds = Math.floor(Date.now() / 1000)
    let token = [newKeycloak.token ?? "", newKeycloak.refreshTokenParsed?.exp ?? nowInSeconds].join(
        ":"
    )
    return token
}

const readTokenStorage = (): string | null => {
    let token = localStorage.getItem("token")
    if (!token) {
        return null
    }
    let [tokenStr, expStr] = token.split(":")
    let exp = parseInt(expStr)
    let nowInSeconds = Math.floor(Date.now() / 1000)
    if (exp < nowInSeconds) {
        return null
    }
    return tokenStr
}

/**
 * AuthContextProvider is responsible for managing the authentication state of the current user.
 *
 * @param props
 */
const AuthContextProvider = (props: AuthContextProviderProps) => {
    const {loaded: loadedGlobalSettings, globalSettings} = useContext(SettingsContext)
    const [keycloak, setKeycloak] = useState<Keycloak | null>(null)
    const [isKeycloakInitialized, setIsKeycloakInitialized] = useState<boolean>(false)

    // Create the local state in which we will keep track if a user is authenticated
    const [isAuthenticated, setAuthenticated] = useState<boolean>(false)
    // Local state that will contain the users name once it is loaded
    const [userId, setUserId] = useState<string>("")
    const [username, setUsername] = useState<string>("")
    const [email, setEmail] = useState<string>("")
    const [firstName, setFirstName] = useState<string>("")
    const [tenantId, setTenantId] = useState<string>("")
    const [trustee, setTrustee] = useState<string>("")
    const [permissionLabels, setPermissionLabels] = useState<string[]>([])
    const [selectedTenantId, setSelectTenantId] = useState<string | null>(
        localStorage.getItem("selected-tenant-id")
    )

    const modifySelectedTenantId = (val: string | null) => {
        if (null === val) {
            localStorage.removeItem("selected-tenant-id")
        } else {
            localStorage.setItem("selected-tenant-id", val)
        }
        setSelectTenantId(val)
    }

    const [openModal, setOpenModal] = React.useState(false)
    const {t, i18n} = useTranslation()

    const sleepSecs = 50
    const bufferSecs = 10

    const navigate = useNavigate()
    const location = useLocation()

    const fetchGraphQL = async (
        operationsDoc: string,
        operationName: string,
        variables: Record<string, any>
    ): Promise<ExecutionResult<GetAllTenantsQuery>> => {
        let result = await fetch(globalSettings.HASURA_URL, {
            method: "POST",
            body: JSON.stringify({
                query: operationsDoc,
                variables,
            }),
        })
        return result.json()
    }

    const operation = `
        query GetAllTenants {
            sequent_backend_tenant {
                id
                slug
            }
        }
    `

    useEffect(() => {
        if (location.pathname.endsWith("/tenant")) {
            if (readTokenStorage()) {
                setOpenModal(true)
            } else {
                setOpenModal(false)
                modifySelectedTenantId(null)
            }
        }
    }, [location.pathname.endsWith("/tenant"), localStorage.getItem("token"), selectedTenantId])

    /**
     * Initializes the Keycloak instance for the specified tenant and handles authentication.
     *
     * @param {string} tenantId - The ID of the tenant to initialize Keycloak for.
     * @returns {Promise<boolean>} - A promise that resolves to `true` if the user is authenticated, `false` otherwise.
     *
     * @throws {Error} - Throws an error if the initialization process fails.
     *
     * @remarks
     * This function performs the following steps:
     * 1. Creates a Keycloak instance with the specified tenant configuration.
     * 2. Stores the tenant ID in local storage.
     * 3. Attempts to initialize Keycloak with the `login-required` option to force login if not authenticated.
     * 4. If the initial initialization fails, it retries with the `check-sso` option to check for an existing session.
     * 5. Updates the state with the new Keycloak instance and authentication status.
     * 6. Sets a timeout to periodically update the token.
     * 7. Redirects to the home page on failure.
     */
    const initKeycloak = async (tenantId: string) => {
        try {
            // Create the Keycloak instance with the specified tenant
            const keycloakConfig: KeycloakConfig = {
                realm: `tenant-${tenantId}`,
                clientId: globalSettings.ONLINE_VOTING_CLIENT_ID,
                url: globalSettings.KEYCLOAK_URL,
            }
            const newKeycloak = new Keycloak(keycloakConfig)

            // Store the tenant ID for initialization
            modifySelectedTenantId(tenantId)
            navigate("/")

            // Initialize Keycloak with login-required to force login if not authenticated
            const keycloakInitOptions: KeycloakInitOptions = {
                onLoad: "login-required", // Force login if not authenticated
                checkLoginIframe: false,
                // redirectUri: window.location.origin // + "/?tenant=" + tenantId
            }

            try {
                // Initialize and get authentication status
                const isAuthenticatedResponse = await newKeycloak.init(keycloakInitOptions)

                // Update state with the new Keycloak instance
                setKeycloak(newKeycloak)

                // User should be authenticated now due to login-required
                localStorage.setItem("token", generateTokenStorage(newKeycloak))
                setAuthenticated(true)
                setIsKeycloakInitialized(true)
                setTimeout(updateTokenPeriodically, 4e3)
                navigate("/")
                return true
            } catch (initError) {
                // If initialization fails, try with check-sso instead
                const fallbackOptions: KeycloakInitOptions = {
                    onLoad: "check-sso",
                    checkLoginIframe: false,
                }

                const fallbackResponse = await newKeycloak.init(fallbackOptions)

                // Update state with the new Keycloak instance
                setKeycloak(newKeycloak)

                if (fallbackResponse) {
                    // User is authenticated
                    localStorage.setItem("token", generateTokenStorage(newKeycloak))
                    setAuthenticated(true)
                    setIsKeycloakInitialized(true)
                    setTimeout(updateTokenPeriodically, 4e3)
                    return true
                } else {
                    // User is not authenticated, but we have a Keycloak instance
                    setAuthenticated(false)
                    setIsKeycloakInitialized(true)
                    return false
                }
            }
        } catch (error) {
            setAuthenticated(false)
            navigate("/tenant") // Redirect back on failure
            return false
        }
    }

    const fetchGetTenant = async (): Promise<ExecutionResult<GetAllTenantsQuery>> => {
        return fetchGraphQL(operation, "GetTenant", {})
    }

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

    const createKeycloak = (tenantId?: string) => {
        if (keycloak) {
            return
        }
        /**
         * KeycloakConfig configures the connection to the Keycloak server.
         */
        const storedTenantId = tenantId || selectedTenantId || globalSettings.DEFAULT_TENANT_ID

        if (location.pathname.endsWith("/tenant") && !selectedTenantId) {
            return
        }

        modifySelectedTenantId(storedTenantId)

        const keycloakConfig: KeycloakConfig = {
            realm: `tenant-${storedTenantId}`,
            clientId: globalSettings.ONLINE_VOTING_CLIENT_ID,
            url: globalSettings.KEYCLOAK_URL,
        }
        const newKeycloak = new Keycloak(keycloakConfig)
        setKeycloak(newKeycloak)
    }

    const initializeKeycloak = async () => {
        if (!keycloak) {
            return
        }
        try {
            /**
             * KeycloakInitOptions configures the Keycloak client.
             */
            const keycloakInitOptions: KeycloakInitOptions = {
                // Configure that Keycloak will check if a user is already authenticated (when
                // opening the app or reloading the page). If not authenticated, we'll handle
                // this in the App component by showing the SelectTenant screen.
                onLoad: location.pathname.endsWith("/tenant") ? "check-sso" : "login-required",
                checkLoginIframe: false,
                flow: "standard", // Use standard flow instead of implicit
                responseMode: "fragment", // Use fragment response mode
            }
            const isAuthenticatedResponse = await keycloak.init(keycloakInitOptions)

            // If the authentication was not successful, we'll let the App component handle it
            // by showing the SelectTenant screen
            if (!isAuthenticatedResponse) {
                setAuthenticated(false)
                setIsKeycloakInitialized(true) // Still mark as initialized so we can use it for login
                localStorage.removeItem("token")
                if (location.pathname.endsWith("/tenant") && selectedTenantId) {
                    modifySelectedTenantId(null)
                }
                return
            }
            if (!keycloak.token) {
                setAuthenticated(false)
                setIsKeycloakInitialized(true) // Still mark as initialized so we can use it for login
                return
            }
            // If we get here the user is authenticated and we can update the state accordingly
            localStorage.setItem("token", generateTokenStorage(keycloak))
            setAuthenticated(true)
            setTimeout(updateTokenPeriodically, 4e3)
            setIsKeycloakInitialized(true)
        } catch (error) {
            setAuthenticated(false)
            setIsKeycloakInitialized(true) // Still mark as initialized so we can use it for login
            localStorage.removeItem("token")
            if (location.pathname.endsWith("/tenant") && selectedTenantId) {
                modifySelectedTenantId(null)
            }
        }
    }

    useEffect(() => {
        if (loadedGlobalSettings && !keycloak) {
            createKeycloak()
        }
    }, [loadedGlobalSettings, keycloak])

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
                return
            }
            if (refreshed) {
                localStorage.setItem("token", generateTokenStorage(keycloak))
            }
        }
        await sleep(sleepSecs * 1e3)
        updateTokenPeriodically()
    }

    const extractPermissionLabels = (input: string): string[] => {
        const regex = /"(.*?)"/g
        const matches = []
        let match
        while ((match = regex.exec(input)) !== null) {
            matches.push(match[1])
        }
        return matches
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

                if (keycloak.tokenParsed?.trustee) {
                    setTrustee(keycloak.tokenParsed?.trustee)
                }
                const tokenPermissionLabels =
                    keycloak.tokenParsed?.["https://hasura.io/jwt/claims"]?.[
                        "x-hasura-permission-labels"
                    ]
                if (tokenPermissionLabels) {
                    const permissionLabelsArray = extractPermissionLabels(tokenPermissionLabels)
                    setPermissionLabels(permissionLabelsArray)
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
    const logout = (redirectUri?: string) => {
        if (!keycloak) {
            return
        }
        localStorage.removeItem("token")
        sessionStorage.removeItem("selected-election-event-tally-id")

        let redirect = redirectUri ? redirectUri : window.location.origin

        // Redirect to the main route after logout
        keycloak.logout({
            redirectUri: redirect,
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
        return !!keycloak.tokenParsed?.["https://hasura.io/jwt/claims"]?.[
            "x-hasura-allowed-roles"
        ].includes(role)
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

    const updateTokenAndPermissionLabels = () => {
        if (keycloak) {
            const refresh = keycloak.updateToken(keycloak.tokenParsed?.exp || 30)
            refresh
                .then(() => {
                    const tokenPermissionLabels =
                        keycloak.tokenParsed?.["https://hasura.io/jwt/claims"]?.[
                            "x-hasura-permission-labels"
                        ]
                    if (tokenPermissionLabels) {
                        const permissionLabelsArray = extractPermissionLabels(tokenPermissionLabels)
                        setPermissionLabels(permissionLabelsArray)
                    }
                })
                .catch((error) => {
                    console.log("error updating token", error)
                })
        }
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
                trustee,
                logout,
                hasRole,
                getAccessToken,
                isAuthorized,
                openProfileLink,
                permissionLabels,
                updateTokenAndPermissionLabels,
                isGoldUser,
                reauthWithGold,
                initKeycloak,
            }}
        >
            {selectedTenantId ? props.children : <SelectTenant />}
            <Dialog
                variant="info"
                hasCloseButton={false}
                open={openModal}
                ok={String(t("common.label.logout"))}
                cancel={String(t("common.label.continue"))}
                title={String(t("common.label.warning"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        modifySelectedTenantId(null)
                        logout(window.location.origin + "/tenant")
                    } else {
                        setOpenModal(false)
                    }
                }}
            >
                {t("common.message.continueOrLogout")}
            </Dialog>
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
