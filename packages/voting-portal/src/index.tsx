// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import ReactDOM from "react-dom/client"
import {Provider} from "react-redux"
import {store} from "./store/store"
import "./index.css"
import App from "./App"
import "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import {NotFoundScreen, theme} from "@sequentech/ui-essentials"
import SequentCoreLibInit, {set_hooks} from "sequent-core"
import AuthContextProvider from "./providers/AuthContextProvider"
import {SettingsContext, SettingsWrapper} from "./providers/SettingsContextProvider"
import {ApolloWrapper} from "./providers/ApolloContextProvider"
import {createBrowserRouter, RouterProvider} from "react-router-dom"
import {LoginScreen} from "./screens/LoginScreen"
import {StartScreen} from "./screens/StartScreen"
import {VotingScreen} from "./screens/VotingScreen"
import {ReviewScreen} from "./screens/ReviewScreen"
import {ConfirmationScreen} from "./screens/ConfirmationScreen"
import {AuditScreen} from "./screens/AuditScreen"
import {ElectionSelectionScreen} from "./screens/ElectionSelectionScreen"
import {BallotLocator} from "./screens/BallotLocator"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

SequentCoreLibInit().then(set_hooks)

export type TenantEvent = {
    tenantId: string
    eventId: string
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
    const {globalSettings} = useContext(SettingsContext)
    return <KeycloakProvider disable={globalSettings.DISABLE_AUTH}>{children}</KeycloakProvider>
}

const router = createBrowserRouter([
    {
        path: "*",
        // TODO: improve this screen
        element: <NotFoundScreen />,
    },
    {
        path: "/",
        element: <App />,
        children: [
            {
                path: "/tenant/:tenantId/event/:eventId/login",
                element: <LoginScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election-chooser",
                element: <ElectionSelectionScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/start",
                element: <StartScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/vote",
                element: <VotingScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/review",
                element: <ReviewScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/confirmation",
                element: <ConfirmationScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/audit",
                element: <AuditScreen />,
            },
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/ballot-locator/:ballotId?",
                element: <BallotLocator />,
            },
        ],
    },
])

root.render(
    <React.StrictMode>
        <SettingsWrapper>
            <KeycloakProviderContainer>
                <Provider store={store}>
                    <ApolloWrapper>
                        <ThemeProvider theme={theme}>
                            <RouterProvider router={router} />
                        </ThemeProvider>
                    </ApolloWrapper>
                </Provider>
            </KeycloakProviderContainer>
        </SettingsWrapper>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
