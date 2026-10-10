// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import ReactDOM from "react-dom/client"
import {Provider} from "react-redux"
import {store} from "./store/store"
import "./index.css"
import "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import AuthContextProvider from "./providers/AuthContextProvider"
import {SettingsContext, SettingsWrapper} from "./providers/SettingsContextProvider"
import {createBrowserRouter, RouterProvider} from "react-router-dom"
import {appRoutes} from "./appRoutes"
import {WasmWrapper} from "./providers/WasmWrapper"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

export type TenantEventType = {
    tenantId: string
    eventId: string
}

export type PreviewPublicationEventType = {
    tenantId: string
    documentId: string
    areaId: string
    publicationId: string
}

export interface KeycloakProviderProps extends React.PropsWithChildren {
    disable: boolean
}

const KeycloakProvider: React.FC<KeycloakProviderProps> = ({disable, children}) => {
    return disable ? (
        <>{children}</>
    ) : (
        <AuthContextProvider>
            <>{children}</>
        </AuthContextProvider>
    )
}

export const KeycloakProviderContainer: React.FC<React.PropsWithChildren> = ({children}) => {
    const {globalSettings, setDisableAuth} = useContext(SettingsContext)

    return <KeycloakProvider disable={globalSettings.DISABLE_AUTH}>{children}</KeycloakProvider>
}

const router = createBrowserRouter(appRoutes, {
    basename: "/",
})

root.render(
    <React.StrictMode>
        <WasmWrapper>
            <SettingsWrapper>
                <KeycloakProviderContainer>
                    <Provider store={store}>
                        <ThemeProvider theme={theme}>
                            <RouterProvider router={router} />
                        </ThemeProvider>
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
