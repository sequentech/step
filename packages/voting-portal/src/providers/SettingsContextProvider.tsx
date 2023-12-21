// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, CircularProgress} from "@mui/material"
import React, {createContext, useContext, useEffect, useState} from "react"

export interface GlobalSettings {
    QUERY_POLL_INTERVAL_MS: number
    DEFAULT_TENANT_ID: string
    ONLINE_VOTING_CLIENT_ID: string
    BALLOT_VERIFIER_URL: string
    KEYCLOAK_URL: string
    APP_VERSION: string
}

interface SettingsContextValues {
    loaded: boolean
    globalSettings: GlobalSettings
}

const defaultSettingsValues: SettingsContextValues = {
    loaded: false,
    globalSettings: {
        QUERY_POLL_INTERVAL_MS: 2000,
        DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        ONLINE_VOTING_CLIENT_ID: "admin-portal",
        KEYCLOAK_URL: "http://127.0.0.1:8090/",
        APP_VERSION: "10.0.0",
        BALLOT_VERIFIER_URL: "http://127.0.0.1:3001/",
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

export const SettingsGate: React.FC<React.PropsWithChildren> = ({children}) => {
    const {loaded} = useContext(SettingsContext)

    return loaded ? (
        <>{children}</>
    ) : (
        <Box>
            <CircularProgress />
        </Box>
    )
}

export const SettingsWrapper: React.FC<React.PropsWithChildren> = ({children}) => (
    <SettingsContextProvider>
        <SettingsGate>{children}</SettingsGate>
    </SettingsContextProvider>
)
