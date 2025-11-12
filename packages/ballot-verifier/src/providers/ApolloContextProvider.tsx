// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, useContext, PropsWithChildren, createContext} from "react"
import {ApolloClient, InMemoryCache, NormalizedCacheObject, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"
import {AuthContext} from "./AuthContextProvider"
import {Box, CircularProgress} from "@mui/material"
import {ApolloProvider} from "@apollo/client"
import {useParams} from "react-router-dom"
import {SettingsContext} from "./SettingsContextProvider"

interface ApolloContextValues {
    apolloClient: ApolloClient<NormalizedCacheObject> | null
}

const defaultApolloContextValues: ApolloContextValues = {
    apolloClient: null,
}
/**
 * Create the AuthContext using the default values.
 */
export const ApolloContext = createContext<ApolloContextValues>(defaultApolloContextValues)

interface ApolloContextProviderProps {
    /**
     * The elements wrapped by the auth context.
     */
    children: React.ReactNode
}

export const ApolloContextProvider = ({children}: ApolloContextProviderProps) => {
    const [apolloClient, setApolloClient] = useState<ApolloClient<NormalizedCacheObject> | null>(
        null
    )
    const {isAuthenticated, getAccessToken, login} = useContext(AuthContext)
    let {tenantId, eventId} = useParams()
    const {globalSettings} = useContext(SettingsContext)

    useEffect(() => {
        if (!isAuthenticated && tenantId && eventId) {
            login(tenantId, eventId)
        }
    }, [isAuthenticated, tenantId, eventId])

    const createApolloClient = (): ApolloClient<NormalizedCacheObject> => {
        const httpLink = createHttpLink({
            uri: globalSettings.HASURA_URL,
        })

        const authLink = setContext((_, {headers}) => {
            // get the authentication token from local storage if it exists
            const token = getAccessToken()
            // return the headers to the context so httpLink can read them
            return {
                headers: {
                    ...headers,
                    authorization: token ? `Bearer ${token}` : "",
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
        if (apolloClient || !isAuthenticated) {
            return
        }
        let token = getAccessToken()
        if (!token) {
            return
        }
        let newClient = createApolloClient()
        setApolloClient(newClient)
    }, [isAuthenticated, apolloClient])

    // Setup the context provider
    return (
        <ApolloContext.Provider
            value={{
                apolloClient,
            }}
        >
            {children}
        </ApolloContext.Provider>
    )
}

export const ApolloWrapper: React.FC<PropsWithChildren> = ({children}) => {
    const {apolloClient} = useContext(ApolloContext)
    return (
        <>
            {null === apolloClient ? (
                <Box>
                    <CircularProgress />
                </Box>
            ) : (
                <ApolloProvider client={apolloClient}>{children}</ApolloProvider>
            )}
        </>
    )
}
