// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useEffect, useState} from "react"
import {Loader} from "@sequentech/ui-essentials"

export interface GlobalSettings {
    DISABLE_AUTH: boolean
    QUERY_POLL_INTERVAL_MS: number
    DEFAULT_TENANT_ID: string
    DEFAULT_EVENT_ID: string
    ONLINE_VOTING_CLIENT_ID: string
    BALLOT_VERIFIER_URL: string
    KEYCLOAK_URL: string
    HASURA_URL: string
    APP_VERSION: string
    APP_HASH: string
    PUBLIC_BUCKET_URL: string
    KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS: number
    POLLING_DURATION_TIMEOUT: number
}

interface SettingsContextValues {
    loaded: boolean
    globalSettings: GlobalSettings
    setDisableAuth: (disable: boolean) => void
}

const defaultSettingsValues: SettingsContextValues = {
    loaded: false,
    globalSettings: {
        DISABLE_AUTH: false,
        QUERY_POLL_INTERVAL_MS: 2000,
        DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        DEFAULT_EVENT_ID: "33f18502-a67c-4853-8333-a58630663559",
        ONLINE_VOTING_CLIENT_ID: "voting-portal",
        KEYCLOAK_URL: "http://127.0.0.1:8090/",
        HASURA_URL: "http://localhost:8080/v1/graphql",
        APP_VERSION: "-",
        APP_HASH: "-",
        BALLOT_VERIFIER_URL: "http://127.0.0.1:3001/",
        PUBLIC_BUCKET_URL: "http://127.0.0.1:9002/public/",
        KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS: 900,
        POLLING_DURATION_TIMEOUT: 12000,
    },
    setDisableAuth: () => {},
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
    const isPreviewMatch =
        window.location.pathname.includes("preview/") &&
        !window.location.pathname.includes("tenant/")

    useEffect(() => {
        if (!loaded) {
            loadSettings()
        }
    }, [loaded])

    const loadSettings = async () => {
        try {
            let value = await fetch("/global-settings.json")
            let json = (await value.json()) as GlobalSettings
            if (isPreviewMatch) {
                json.DISABLE_AUTH = true
            }
            setSettings(json)
            setLoaded(true)
        } catch (e) {
            console.log(`Error loading settings: ${e}`)
        }
    }

    const setDisableAuth = (disable: boolean) => {
        setSettings({
            ...globalSettings,
            DISABLE_AUTH: disable,
        })
    }

    // Setup the context provider
    return (
        <SettingsContext.Provider
            value={{
                loaded,
                globalSettings,
                setDisableAuth,
            }}
        >
            {props.children}
        </SettingsContext.Provider>
    )
}

export const SettingsGate: React.FC<React.PropsWithChildren> = ({children}) => {
    const {loaded} = useContext(SettingsContext)

    return loaded ? <>{children}</> : <Loader />
}

export const SettingsWrapper: React.FC<React.PropsWithChildren> = ({children}) => (
    <SettingsContextProvider>
        <SettingsGate>{children}</SettingsGate>
    </SettingsContextProvider>
)
