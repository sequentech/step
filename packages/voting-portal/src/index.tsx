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
import {theme} from "@sequentech/ui-essentials"
import SequentCoreLibInit, {set_hooks} from "sequent-core"
import AuthContextProvider from "./providers/AuthContextProvider"
import {SettingsContext, SettingsWrapper} from "./providers/SettingsContextProvider"
import {createBrowserRouter, RouterProvider} from "react-router-dom"
import {LoginScreen} from "./routes/LoginScreen"
import {StartScreen} from "./routes/StartScreen"
import VotingScreen, {action as votingAction} from "./routes/VotingScreen"
import {ReviewScreen} from "./routes/ReviewScreen"
import {ConfirmationScreen} from "./routes/ConfirmationScreen"
import {AuditScreen} from "./routes/AuditScreen"
import {ElectionSelectionScreen} from "./routes/ElectionSelectionScreen"
import {BallotLocator} from "./routes/BallotLocator"
import {ErrorPage} from "./routes/ErrorPage"
import {SupportMaterialsScreen} from "./routes/SupportMaterialsScreen"
import TenantEvent from "./routes/TenantEvent"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

SequentCoreLibInit().then(set_hooks)

export type TenantEventType = {
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
        path: "/",
        element: <App />,
        errorElement: <ErrorPage />,
        children: [
            {
                path: "/tenant/:tenantId/event/:eventId",
                element: <TenantEvent />,
                children: [
                    {
                        path: "election-chooser",
                        element: <ElectionSelectionScreen />,
                    },
                    {
                        path: "login",
                        element: <LoginScreen />,
                    },
                    {
                        path: "election/:electionId",
                        children: [
                            {
                                path: "start",
                                element: <StartScreen />,
                            },
                            {
                                path: "vote",
                                element: <VotingScreen />,
                                action: votingAction,
                            },
                            {
                                path: "review",
                                element: <ReviewScreen />,
                            },
                            {
                                path: "confirmation",
                                element: <ConfirmationScreen />,
                            },
                            {
                                path: "audit",
                                element: <AuditScreen />,
                            },
                            {
                                path: "ballot-locator/:ballotId?",
                                element: <BallotLocator />,
                            },
                        ],
                    },
                    {
                        path: "materials",
                        element: <SupportMaterialsScreen />,
                    },
                ],
            },
        ],
    },
])

root.render(
    <React.StrictMode>
        <SettingsWrapper>
            <KeycloakProviderContainer>
                <Provider store={store}>
                    <ThemeProvider theme={theme}>
                        <RouterProvider router={router} />
                    </ThemeProvider>
                </Provider>
            </KeycloakProviderContainer>
        </SettingsWrapper>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
