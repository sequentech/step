// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, CircularProgress} from "@mui/material"
import React, {createContext, useContext, useEffect, useState} from "react"
import {styled} from "@mui/material/styles"

export interface GlobalSettings {
    QUERY_POLL_INTERVAL_MS: number
    QUERY_FAST_POLL_INTERVAL_MS: number
    DEFAULT_TENANT_ID: string
    ONLINE_VOTING_CLIENT_ID: string
    KEYCLOAK_URL: string
    HASURA_URL: string
    APP_VERSION: string
    APP_HASH: string
    DEFAULT_EMAIL_SUBJECT: {[langCode: string]: string}
    DEFAULT_EMAIL_HTML_BODY: {[langCode: string]: string}
    DEFAULT_EMAIL_PLAINTEXT_BODY: {[langCode: string]: string}
    DEFAULT_SMS_MESSAGE: {[langCode: string]: string}
    DEFAULT_DOCUMENT: {[langCode: string]: string}
    PUBLIC_BUCKET_URL: string
    VOTING_PORTAL_URL: string
    ACTIVATE_MIRU_EXPORT: boolean
    CUSTOM_URLS_DOMAIN_NAME: string
}

interface SettingsContextValues {
    loaded: boolean
    globalSettings: GlobalSettings
}

const defaultSettingsValues: SettingsContextValues = {
    loaded: false,
    globalSettings: {
        QUERY_POLL_INTERVAL_MS: 3000,
        QUERY_FAST_POLL_INTERVAL_MS: 5000,
        DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        ONLINE_VOTING_CLIENT_ID: "admin-portal",
        KEYCLOAK_URL: "http://127.0.0.1:8090/",
        HASURA_URL: "http://localhost:8080/v1/graphql",
        APP_VERSION: "-",
        APP_HASH: "-",
        DEFAULT_EMAIL_SUBJECT: {en: "Participate in {{election_event.name}}"},
        DEFAULT_EMAIL_HTML_BODY: {
            en: "<p>Hello {{user.first_name}},<br><br>Enter in {{vote_url}} to vote</p>",
        },
        DEFAULT_EMAIL_PLAINTEXT_BODY: {
            en: "Hello {{user.first_name}},\n\nEnter in {{vote_url}} to vote",
        },
        DEFAULT_SMS_MESSAGE: {
            en: "Enter in {{vote_url}} to vote",
        },
        DEFAULT_DOCUMENT: {
            en: `<div>
  {{{data.logo}}}
</div>
<div>
  <h2>Your vote has been cast</h2>
  <p>
    The confirmation code bellow verifies that your ballot has been cast
    successfully. You can use this code to verify that your ballot has been
    counted.
  </p>
  <div class="info">
    <p>
      Your Ballot ID:
      <span class="id-content">{{data.ballot_id}}</span>
    </p>
    <p>
      Ballot tracker link:
      <a href="{{data.ballot_tracker_url}}">Click here</a>
    </p>
  </div>
</div>

<div>
  <h2>Verify that your ballot has been cast</h2>
  <p>
    You can verify your ballot has been cast correctly at any moment using the
    following QR code:
  </p>
  {{{data.qrcode}}}
</div>`,
        },
        PUBLIC_BUCKET_URL: "http://127.0.0.1:9002/public/",
        VOTING_PORTAL_URL: "http://localhost:3000",
        ACTIVATE_MIRU_EXPORT: false,
        CUSTOM_URLS_DOMAIN_NAME: "google.com",
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

            setSettings(Object.assign({}, defaultSettingsValues.globalSettings, json))
            setLoaded(true)
        } catch (e) {
            console.log(`Error loading settings: ${e}`)
        }
    }

    useEffect(() => {
        if (!loaded) {
            loadSettings()
        }
    }, [loaded])
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
