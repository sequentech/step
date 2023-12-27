// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, PropsWithChildren, useCallback} from "react"
import {ApolloClient, InMemoryCache, NormalizedCacheObject, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"
import {AuthContext} from "./AuthContextProvider"
import {ApolloProvider} from "@apollo/client"
import {SettingsContext} from "./SettingsContextProvider"

export const ApolloWrapper: React.FC<PropsWithChildren> = ({children}) => {
    const {getAccessToken} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    const createApolloClient = useCallback((): ApolloClient<NormalizedCacheObject> => {
        const httpLink = createHttpLink({
            uri: globalSettings.HASURA_URL,
        })

        const token = getAccessToken()
        const authLink = token
            ? setContext((_, {headers}) => {
                  // get the authentication token from local storage if it exists
                  // return the headers to the context so httpLink can read them
                  return {
                      headers: {
                          ...headers,
                          authorization: token ? `Bearer ${token}` : "",
                      },
                  }
              })
            : null

        const apolloClient = new ApolloClient({
            link: authLink ? authLink.concat(httpLink) : httpLink,
            cache: new InMemoryCache(),
        })
        return apolloClient
    }, [getAccessToken, globalSettings.HASURA_URL])

    let apolloClient = createApolloClient()

    return (
        <>
            {null === apolloClient ? (
                <>{children}</>
            ) : (
                <ApolloProvider client={apolloClient}>{children}</ApolloProvider>
            )}
        </>
    )
}
