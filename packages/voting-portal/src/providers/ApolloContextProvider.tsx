// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, PropsWithChildren, useState, useEffect, useRef} from "react"
import {ApolloClient, InMemoryCache, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"
import {AuthContext} from "./AuthContextProvider"
import {ApolloProvider} from "@apollo/client/react"
import {SettingsContext} from "./SettingsContextProvider"
import {Box, CircularProgress} from "@mui/material"
import {clearVoterSession, store} from "../store/store"
import {voterSessionScope} from "../utils/voterSessionScope"

export const ApolloWrapper: React.FC<PropsWithChildren> = ({children}) => {
    const {globalSettings} = useContext(SettingsContext)
    const {keycloakAccessToken, isAuthContextInitialized} = useContext(AuthContext)
    const scope = globalSettings.DISABLE_AUTH ? "demo" : voterSessionScope(keycloakAccessToken)
    const previousScope = useRef<string | undefined>(undefined)
    const [loadedScope, setLoadedScope] = useState<string | undefined>(undefined)
    const [client, setClient] = useState<ApolloClient | null>(null)

    useEffect(() => {
        if (scope !== previousScope.current) {
            previousScope.current = scope
            store.dispatch(clearVoterSession())
            setLoadedScope(scope)
        }
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
        return () => apolloClient.stop()
    }, [
        isAuthContextInitialized,
        keycloakAccessToken,
        scope,
        globalSettings.HASURA_URL,
        globalSettings.DISABLE_AUTH,
    ])

    return client === null ||
        scope !== loadedScope ||
        (!globalSettings.DISABLE_AUTH && (!isAuthContextInitialized || !keycloakAccessToken)) ? (
        <Box sx={{flex: 1, display: "flex", justifyContent: "center", alignItems: "center"}}>
            <CircularProgress />
        </Box>
    ) : (
        <ApolloProvider client={client}>{children}</ApolloProvider>
    )
}
