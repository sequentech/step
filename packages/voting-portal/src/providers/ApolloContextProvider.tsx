// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, PropsWithChildren, useCallback} from "react"
import {ApolloClient, InMemoryCache, NormalizedCacheObject, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"
import {AuthContext} from "./AuthContextProvider"
import {ApolloProvider} from "@apollo/client"
import {SettingsContext} from "./SettingsContextProvider"
import {Box, CircularProgress} from "@mui/material"

export const ApolloWrapper: React.FC<PropsWithChildren> = ({children}) => {
    const {globalSettings} = useContext(SettingsContext)

    const {getAccessToken} = useContext(AuthContext)

    const createApolloClient = useCallback((): ApolloClient<NormalizedCacheObject> | null => {
        const token = getAccessToken()
        console.log("LS -> src/providers/ApolloContextProvider.tsx:19 -> token: ", token)

        if (!token) {
            return null
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
                    authorization: token ? `Bearer ${token}` : "",
                },
            }
        })

        const apolloClient = new ApolloClient({
            link: authLink.concat(httpLink),
            cache: new InMemoryCache(),
        })

        return apolloClient
    }, [getAccessToken, globalSettings.HASURA_URL])

    let apolloClient = createApolloClient()

    return apolloClient === null ? (
        <Box sx={{marginTop: "25px"}}>
            <CircularProgress />
        </Box>
    ) : (
        <ApolloProvider client={apolloClient}>{children}</ApolloProvider>
    )
}
