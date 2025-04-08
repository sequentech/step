// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
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
    logout: () => void
    /**
     * Check if the user has the given role
     */
    hasRole: (role: string) => boolean
    /**
     * Get Access Token
     */
    getAccessToken: () => string | undefined

    updateTokenAndPermissionLabels: () => void

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
    const [isGetTenantChecked, setIsGetTenantChecked] = useState<boolean>(false)

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

    useEffect(() => {
        const getTenant = async (slug: string) => {
            try {
                const {data, errors} = await fetchGetTenant()

                if (errors) {
                    console.error(errors)
                    return
                }
                const tenants = data?.sequent_backend_tenant
                const tenantIdFromParam = slug

                if (tenants && tenantIdFromParam) {
                    const matchedTenant = tenants.find(
                        (tenant: {id: string; slug: string}) => tenant.slug === tenantIdFromParam
                    )

                    if (matchedTenant) {
                        const currentTenantId = localStorage.getItem("selected-tenant-id")

                        if (currentTenantId !== matchedTenant.id) {
                            localStorage.setItem("selected-tenant-id", matchedTenant.id)
                            createKeycloak()
                            navigate(`/`)
                        } else {
                            navigate(`/`)
                        }
                    }
                } else {
                    console.error("Tenant not found")
                }
            } catch (error) {
                console.error(error)
            }
            setIsGetTenantChecked(true)
        }

        if (location.pathname.includes("/admin/login")) {
            const slug = location.pathname.split("/").pop()
            if (slug) {
                getTenant(slug || "")
            }
        } else {
            setIsGetTenantChecked(true)
            createKeycloak()
        }
    }, [])

    const createKeycloak = () => {
        if (keycloak) {
            return
        }
        /**
         * KeycloakConfig configures the connection to the Keycloak server.
         */
        let localStoredTenant = localStorage.getItem("selected-tenant-id")
        let newTenant = localStoredTenant ? localStoredTenant : globalSettings.DEFAULT_TENANT_ID

        const keycloakConfig: KeycloakConfig = {
            realm: `tenant-${newTenant}`,
            clientId: globalSettings.ONLINE_VOTING_CLIENT_ID,
            url: globalSettings.KEYCLOAK_URL,
        }

        // Create the Keycloak client instance
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
                // opening the app or reloading the page). If not authenticated the user will
                // be send to the login form. If already authenticated the webapp will open.
                onLoad: "login-required",
                checkLoginIframe: false,
            }
            const isAuthenticatedResponse = await keycloak.init(keycloakInitOptions)

            // If the authentication was not successfull the user is send back to the Keycloak login form
            if (!isAuthenticatedResponse) {
                await keycloak.login()
            }
            if (!keycloak.token) {
                setAuthenticated(false)
                return
            }
            // If we get here the user is authenticated and we can update the state accordingly
            localStorage.setItem("token", keycloak.token)
            setAuthenticated(true)
            setTimeout(updateTokenPeriodically, 4e3)
            setIsKeycloakInitialized(true)
        } catch (error) {
            setAuthenticated(false)
        }
    }

    useEffect(() => {
        if (keycloak || !loaded || !isGetTenantChecked) {
            return
        }
        createKeycloak()
    }, [loaded, keycloak, isGetTenantChecked])

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
                localStorage.setItem("token", keycloak.token)
            }
        }
        await sleep(sleepSecs * 1e3)
        updateTokenPeriodically()
    }

    const extractPermissionLabels = (input: string): string[] => {
        const regex = /\"(.*?)\"/g
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
    const logout = () => {
        if (!keycloak) {
            return
        }
        localStorage.removeItem("token")
        sessionStorage.removeItem("selected-election-event-tally-id")

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
            }}
        >
            {props.children}
        </AuthContext.Provider>
    )
}

export default AuthContextProvider
