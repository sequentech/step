// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, useContext, PropsWithChildren, createContext} from "react"
import {ApolloClient, InMemoryCache, NormalizedCacheObject, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"
import {AuthContext} from "./AuthContextProvider"
import {Box, CircularProgress} from "@mui/material"
import {ApolloProvider} from "@apollo/client"
import {SettingsContext} from "./SettingsContextProvider"
import {getOperationRole} from "@/services/Permissions"
import {IPermissions} from "@/types/keycloak"

interface ApolloContextValues {
    apolloClient: ApolloClient<NormalizedCacheObject> | null
    role: string
}

export const defaultApolloContextValues: ApolloContextValues = {
    apolloClient: null,
    role: "admin-user",
}
/**
 * Create the AuthContext using the default values.
 */
export const ApolloContext = createContext<ApolloContextValues>(defaultApolloContextValues)

interface ApolloContextProviderProps {
    /**
     * The elements wrapped by the auth context.
     */
    children: JSX.Element
    role: string
}

export const ApolloContextProvider = ({children, role}: ApolloContextProviderProps) => {
    const [apolloClient, setApolloClient] = useState<ApolloClient<NormalizedCacheObject> | null>(
        null
    )
    const {isAuthenticated, getAccessToken, hasRole} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    const createApolloClient = (): ApolloClient<NormalizedCacheObject> => {
        const httpLink = createHttpLink({
            uri: globalSettings.HASURA_URL,
        })

        const authLink = setContext((operation, {headers}) => {
            // get the authentication token from local storage if it exists
            const token = getAccessToken()
            if (!token) {
                console.error("No access token available")
                return {}
            }
            // return the headers to the context so httpLink can read them
            const operationRole = getOperationRole(
                operation,
                hasRole(IPermissions.TRUSTEE_CEREMONY)
            )

            return {
                headers: {
                    "authorization": token ? `Bearer ${token}` : "",
                    "x-hasura-role": operationRole,
                    ...headers,
                },
            }
        })

        const apolloClient = new ApolloClient({
            link: authLink.concat(httpLink),
            cache: new InMemoryCache(),
        })
        return apolloClient
    }

    useEffect(() => {
        // Clear client if not authenticated (logout case)
        if (!isAuthenticated) {
            if (apolloClient) {
                setApolloClient(null)
            }
            return
        }

        // Don't recreate if we already have a client
        if (apolloClient) {
            return
        }

        const token = getAccessToken()
        if (!token) {
            return
        }

        let newClient = createApolloClient()
        setApolloClient(newClient)
    }, [isAuthenticated, apolloClient, getAccessToken])

    // Setup the context provider
    return (
        <ApolloContext.Provider
            value={{
                apolloClient,
                role,
            }}
        >
            {children}
        </ApolloContext.Provider>
    )
}

export const ApolloWrapper: React.FC<PropsWithChildren> = ({children}) => {
    const {apolloClient} = useContext(ApolloContext)

    // Show loading spinner while waiting for client
    if (null === apolloClient) {
        return (
            <Box className="apollo-wrapper">
                <CircularProgress />
            </Box>
        )
    }

    // Show app content when authenticated and client is ready
    return <ApolloProvider client={apolloClient}>{children}</ApolloProvider>
}

export const CustomApolloContextProvider: React.FC<ApolloContextProviderProps> = ({
    children,
    role,
}) => (
    <ApolloContextProvider role={role}>
        <ApolloWrapper>{children}</ApolloWrapper>
    </ApolloContextProvider>
)
