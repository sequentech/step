// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import ReactDOM from "react-dom/client"
import {BrowserRouter, useParams} from "react-router-dom"
import "./index.css"
import App from "./App"
import {initI18n, isI18nInitialized} from "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import AuthContextProvider from "./providers/AuthContextProvider"
import {SettingsContext, SettingsWrapper} from "./providers/SettingsContextProvider"
import {Provider} from "react-redux"
import {store} from "./store/store"
import {WasmWrapper} from "./providers/WasmWrapper"
import {Loader} from "@sequentech/ui-essentials"

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

const getLangParam = (): string | undefined => {
    try {
        const params = new URLSearchParams(window.location.search)
        return params.get("lang") || undefined
    } catch {
        return undefined
    }
}

const I18nGate: React.FC<React.PropsWithChildren> = ({children}) => {
    const [ready, setReady] = React.useState<boolean>(isI18nInitialized())
    React.useEffect(() => {
        let cancelled = false
        const run = async () => {
            if (isI18nInitialized()) {
                if (!cancelled) setReady(true)
                return
            }
            const urlLang = getLangParam()
            await initI18n(urlLang)
            if (!cancelled) setReady(true)
        }
        run()
        return () => {
            cancelled = true
        }
    }, [])
    return ready ? <>{children}</> : <Loader />
}

root.render(
    <React.StrictMode>
        <WasmWrapper>
            <SettingsWrapper>
                <I18nGate>
                    <KeycloakProviderContainer>
                        <Provider store={store}>
                            <BrowserRouter>
                                <ThemeProvider theme={theme}>
                                    <App />
                                </ThemeProvider>
                            </BrowserRouter>
                        </Provider>
                    </KeycloakProviderContainer>
                </I18nGate>
            </SettingsWrapper>
        </WasmWrapper>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
