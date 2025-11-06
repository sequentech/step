// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, PropsWithChildren, useState, useEffect} from "react"
import {ApolloClient, InMemoryCache, NormalizedCacheObject, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"
import {AuthContext} from "./AuthContextProvider"
import {ApolloProvider} from "@apollo/client"
import {SettingsContext} from "./SettingsContextProvider"
import {Box, CircularProgress} from "@mui/material"
import {useMatch} from "react-router-dom"

export const ApolloWrapper: React.FC<PropsWithChildren> = ({children}) => {
    const {globalSettings} = useContext(SettingsContext)
    const {keycloakAccessToken, isAuthContextInitialized} = useContext(AuthContext)
    const [client, setClient] = useState<ApolloClient<NormalizedCacheObject> | null>(null)

    useEffect(() => {
        if (!isAuthContextInitialized && !globalSettings.DISABLE_AUTH) {
            return
        }

        if (!keycloakAccessToken && !globalSettings.DISABLE_AUTH) {
            return
        }

        const httpLink = createHttpLink({
            uri: globalSettings.HASURA_URL,
        })

        const authLink = setContext((_, {headers}) => {
            // get the authentication token from local storage if it exists
            // return the headers to the context so httpLink can read them
            return {
                headers: {
                    ...headers,
                    authorization: keycloakAccessToken ? `Bearer ${keycloakAccessToken}` : "",
                },
            }
        })

        const apolloClient = new ApolloClient({
            link: authLink.concat(httpLink),
            cache: new InMemoryCache(),
        })

        setClient(apolloClient)
    }, [
        isAuthContextInitialized,
        keycloakAccessToken,
        globalSettings.HASURA_URL,
        globalSettings.DISABLE_AUTH,
    ])

    return client === null ? (
        <Box sx={{flex: 1, display: "flex", justifyContent: "center", alignItems: "center"}}>
            <CircularProgress />
        </Box>
    ) : (
        <ApolloProvider client={client}>{children}</ApolloProvider>
    )
}
