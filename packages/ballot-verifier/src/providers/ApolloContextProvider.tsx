// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {
    useEffect,
    useState,
    useContext,
    PropsWithChildren,
    createContext,
    useCallback,
} from "react"
import {ApolloClient, InMemoryCache, NormalizedCacheObject, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"
import {AuthContext} from "./AuthContextProvider"
import {Box, CircularProgress} from "@mui/material"
import {ApolloProvider} from "@apollo/client"
import {useParams} from "react-router-dom"
import {SettingsContext} from "./SettingsContextProvider"
import {IElectionEventPresentation} from "@sequentech/ui-core"
import {ELanguageDetectionPolicy} from "@sequentech/ui-core"

interface ApolloContextValues {
    apolloClient: ApolloClient<NormalizedCacheObject> | null
}

interface ElectionEventConfigDocument {
    id: string
    tenant_id: string
    election_event_id: string
    election_event_presentation: IElectionEventPresentation
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

    const electionEventConfigUrl = `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/event-${eventId}/election_event_config.json`
    // Set up tenant and event in AuthContext on initial load.
    // It is needed to fetch the election event config file from S3
    // and apply the language policy before loading any other data.
    const setupLogin = useCallback(async () => {
        if (!tenantId || !eventId) {
            return
        }

        try {
            const response = await fetch(electionEventConfigUrl)

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`)
            }

            const config = (await response.json()) as ElectionEventConfigDocument
            const presentation = config.election_event_presentation
            const languageConf = presentation?.language_conf

            const defaultLocale =
                languageConf?.language_detection_policy === ELanguageDetectionPolicy.FORCE_DEFAULT
                    ? languageConf.default_language_code
                    : undefined

            login(tenantId, eventId, defaultLocale)
        } catch (error) {
            console.error("Error loading election event config:", error)
            login(tenantId, eventId, undefined)
        }
    }, [tenantId, eventId, electionEventConfigUrl, location.pathname, login])

    useEffect(() => {
        if (!isAuthenticated && tenantId && eventId) {
            void setupLogin()
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
