// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import ReactDOM from "react-dom/client"
import {BrowserRouter, useParams} from "react-router-dom"
import "./index.css"
import App from "./App"
import "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import AuthContextProvider from "./providers/AuthContextProvider"
import {SettingsContext, SettingsWrapper} from "./providers/SettingsContextProvider"
import {Provider} from "react-redux"
import {store} from "./store/store"
import {WasmWrapper} from "./providers/WasmWrapper"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

interface TenantEventContextValues {
    tenantId: string | null
    eventId: string | null
}

export const TenantEventContext = React.createContext<TenantEventContextValues>({
    tenantId: null,
    eventId: null,
})

// This component will be used to provide tenantId and eventId to the context
export const TenantEventProvider: React.FC<{
    tenantId: string | null
    eventId: string | null
    children: React.ReactNode
}> = ({tenantId, eventId, children}) => {
    console.log(`TenantEventProvider: tenantId=${tenantId}, eventId=${eventId}`)
    return (
        <TenantEventContext.Provider value={{tenantId, eventId}}>
            {children}
        </TenantEventContext.Provider>
    )
}
interface TenantEventContextValues {
    tenantId: string | null
    eventId: string | null
}

export interface KeycloakProviderProps extends React.PropsWithChildren {
    disable: boolean
}

const KeycloakProvider: React.FC<KeycloakProviderProps> = ({disable, children}) => {
    const {tenantId, eventId} = useContext(TenantEventContext)
    console.log(`KeycloakProvider: tenantId=${tenantId}, eventId=${eventId}`)

    return disable ? (
        <>{children}</>
    ) : (
        <AuthContextProvider>
            <>{children}</>
        </AuthContextProvider>
    )
}

export const RouteParameterProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const {tenantId, eventId} = useParams<{tenantId: string; eventId: string}>()
    console.log(`RouteParameterProvider: tenantId=${tenantId}, eventId=${eventId}`)

    return (
        <TenantEventProvider
            tenantId={tenantId ? tenantId : null}
            eventId={eventId ? eventId : null}
        >
            {children}
        </TenantEventProvider>
    )
}

export const KeycloakProviderContainer: React.FC<React.PropsWithChildren> = ({children}) => {
    const {globalSettings} = useContext(SettingsContext)
    return <KeycloakProvider disable={globalSettings.DISABLE_AUTH}>{children}</KeycloakProvider>
}

root.render(
    <React.StrictMode>
        <WasmWrapper>
            <SettingsWrapper>
                <KeycloakProviderContainer>
                    <Provider store={store}>
                        <BrowserRouter>
                            <ThemeProvider theme={theme}>
                                <App />
                            </ThemeProvider>
                        </BrowserRouter>
                    </Provider>
                </KeycloakProviderContainer>
            </SettingsWrapper>
        </WasmWrapper>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
