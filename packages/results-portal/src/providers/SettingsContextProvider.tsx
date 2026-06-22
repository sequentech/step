// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useContext, useEffect, useState} from "react"
import {Loader} from "@sequentech/ui-essentials"

export interface GlobalSettings {
    DISABLE_AUTH: boolean
    ONLINE_VOTING_CLIENT_ID: string
    KEYCLOAK_URL: string
    HASURA_URL: string
    APP_VERSION: string
    APP_HASH: string
    PUBLIC_BUCKET_URL: string
    RESULTS_PORTAL_URL: string
}

interface SettingsContextValues {
    loaded: boolean
    globalSettings: GlobalSettings
}

const defaultSettings: GlobalSettings = {
    DISABLE_AUTH: false,
    ONLINE_VOTING_CLIENT_ID: "voting-portal",
    KEYCLOAK_URL: "http://127.0.0.1:8090/",
    HASURA_URL: "http://localhost:8080/v1/graphql",
    APP_VERSION: "dev",
    APP_HASH: "dev",
    PUBLIC_BUCKET_URL: "http://127.0.0.1:9002/public/",
    RESULTS_PORTAL_URL: "http://127.0.0.1:3004",
}

export const SettingsContext = createContext<SettingsContextValues>({
    loaded: false,
    globalSettings: defaultSettings,
})

export const SettingsWrapper: React.FC<React.PropsWithChildren> = ({children}) => {
    const [loaded, setLoaded] = useState(false)
    const [globalSettings, setGlobalSettings] = useState<GlobalSettings>(defaultSettings)

    useEffect(() => {
        let mounted = true

        const loadSettings = async () => {
            try {
                const response = await fetch("/global-settings.json")
                const json = (await response.json()) as GlobalSettings
                if (mounted) {
                    setGlobalSettings({...defaultSettings, ...json})
                    setLoaded(true)
                }
            } catch (error) {
                console.error("Error loading settings", error)
                if (mounted) {
                    setLoaded(true)
                }
            }
        }

        void loadSettings()

        return () => {
            mounted = false
        }
    }, [])

    return (
        <SettingsContext.Provider value={{loaded, globalSettings}}>
            {loaded ? children : <Loader />}
        </SettingsContext.Provider>
    )
}

export const useSettings = () => useContext(SettingsContext)
