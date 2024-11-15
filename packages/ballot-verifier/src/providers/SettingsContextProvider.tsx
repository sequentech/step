// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, CircularProgress} from "@mui/material"
import React, {createContext, useContext, useEffect, useState} from "react"
import {styled} from "@mui/material/styles"

export interface GlobalSettings {
    DISABLE_AUTH: boolean
    QUERY_POLL_INTERVAL_MS: number
    DEFAULT_TENANT_ID: string
    DEFAULT_EVENT_ID: string
    ONLINE_VOTING_CLIENT_ID: string
    KEYCLOAK_URL: string
    HASURA_URL: string
    APP_VERSION: string
    APP_HASH: string
}

interface SettingsContextValues {
    loaded: boolean
    globalSettings: GlobalSettings
}

const defaultSettingsValues: SettingsContextValues = {
    loaded: false,
    globalSettings: {
        DISABLE_AUTH: false,
        QUERY_POLL_INTERVAL_MS: 2000,
        DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        DEFAULT_EVENT_ID: "33f18502-a67c-4853-8333-a58630663559",
        ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
        KEYCLOAK_URL: "http://127.0.0.1:8090/",
        HASURA_URL: "http://localhost:8080/v1/graphql",
        APP_VERSION: "-",
        APP_HASH: "-",
    },
}

export const SettingsContext = createContext<SettingsContextValues>(defaultSettingsValues)

interface SettingsContextProviderProps {
    /**
     * The elements wrapped by the auth context.
     */
    children: JSX.Element
}

const SettingsContextProvider = (props: SettingsContextProviderProps) => {
    const [loaded, setLoaded] = useState<boolean>(false)
    const [globalSettings, setSettings] = useState<GlobalSettings>(
        defaultSettingsValues.globalSettings
    )

    const loadSettings = async () => {
        try {
            let value = await fetch("/global-settings.json")
            let json = await value.json()
            setSettings(json)
            setLoaded(true)
        } catch (e) {
            console.log(`Error loading settings: ${e}`)
        }
    }

    useEffect(() => {
        if (!loaded) {
            loadSettings()
        }
    }, [])
    // Setup the context provider
    return (
        <SettingsContext.Provider
            value={{
                loaded,
                globalSettings,
            }}
        >
            {props.children}
        </SettingsContext.Provider>
    )
}

const StyledBox = styled(Box)`
    display: flex;
    position: absolute;
    top: 0;
    bottom: 0;
    right: 0;
    left: 0;
    margin: auto;
    align-items: center;
    justify-content: center;
`

export const SettingsGate: React.FC<React.PropsWithChildren> = ({children}) => {
    const {loaded} = useContext(SettingsContext)

    return loaded ? (
        <>{children}</>
    ) : (
        <StyledBox>
            <CircularProgress />
        </StyledBox>
    )
}

export const SettingsWrapper: React.FC<React.PropsWithChildren> = ({children}) => (
    <SettingsContextProvider>
        <SettingsGate>{children}</SettingsGate>
    </SettingsContextProvider>
)
