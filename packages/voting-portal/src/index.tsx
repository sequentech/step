// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, lazy, useContext} from "react"
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
import {ErrorPage} from "./routes/ErrorPage"
import {action as votingAction} from "./routes/VotingScreen"
import {action as castBallotAction} from "./routes/ReviewScreen"
import Loader from "./components/Loader"
// import useDemo from "./hooks/useDemo"

const TenantEvent = lazy(() => import("./routes/TenantEvent"))
const ElectionSelectionScreen = lazy(() => import("./routes/ElectionSelectionScreen"))
const LoginScreen = lazy(() => import("./routes/LoginScreen"))
const StartScreen = lazy(() => import("./routes/StartScreen"))
const VotingScreen = lazy(() => import("./routes/VotingScreen"))
const ReviewScreen = lazy(() => import("./routes/ReviewScreen"))
const ConfirmationScreen = lazy(() => import("./routes/ConfirmationScreen"))
const AuditScreen = lazy(() => import("./routes/AuditScreen"))
const BallotLocator = lazy(() => import("./routes/BallotLocator"))
const SupportMaterialsScreen = lazy(() => import("./routes/SupportMaterialsScreen"))

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
    // const isDemo = useDemo()  // TODO: maybe move to redux
    const isDemo = true; //TODO: delete
    const {globalSettings} = useContext(SettingsContext)
    // 1) TODO: check location - if demo: put disable=true
    return (
        <KeycloakProvider disable={isDemo || globalSettings.DISABLE_AUTH}>
            {children}
        </KeycloakProvider>
    )
}

const router = createBrowserRouter(
    [
        {
            path: "/",
            element: <App />,
            errorElement: <ErrorPage />,
            children: [
                // {
                //     path: "/demo",
                //     element: (
                //         <Suspense fallback={<Loader />}>
                //             <DemoEvent />
                //             {/* inside demo event -
                //                 do mock authentication and demo logic,
                //                 save demo mode in context
                //                 navigate to  path: "/tenant/:tenantId/event/:eventId"
                //                  add tenant event*/}
                //         </Suspense>
                //     ),
                // },
                {
                    path: "/tenant/:tenantId/event/:eventId",
                    element: (
                        <Suspense fallback={<Loader />}>
                            <TenantEvent />
                        </Suspense>
                    ),
                    children: [
                        {
                            path: "election-chooser",
                            element: (
                                <Suspense fallback={<Loader />}>
                                    <ElectionSelectionScreen />
                                </Suspense>
                            ),
                        },
                        {
                            path: "login",
                            element: (
                                <Suspense fallback={<Loader />}>
                                    <LoginScreen />
                                </Suspense>
                            ),
                        },
                        {
                            path: "election/:electionId",
                            children: [
                                {
                                    path: "start",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <StartScreen />
                                        </Suspense>
                                    ),
                                },
                                {
                                    path: "vote",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <VotingScreen />
                                        </Suspense>
                                    ),
                                    action: votingAction,
                                },
                                {
                                    path: "review",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <ReviewScreen />
                                        </Suspense>
                                    ),
                                    action: castBallotAction,
                                },
                                {
                                    path: "confirmation",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <ConfirmationScreen />
                                        </Suspense>
                                    ),
                                },
                                {
                                    path: "audit",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <AuditScreen />
                                        </Suspense>
                                    ),
                                },
                                {
                                    path: "ballot-locator/:ballotId?",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <BallotLocator />
                                        </Suspense>
                                    ),
                                },
                            ],
                        },
                        {
                            path: "materials",
                            element: (
                                <Suspense fallback={<Loader />}>
                                    <SupportMaterialsScreen />
                                </Suspense>
                            ),
                        },
                    ],
                },
            ],
        },
    ],
    {
        basename: "/",
    }
)

root.render(
    <React.StrictMode>
        <SettingsWrapper>
            <Provider store={store}>
                <KeycloakProviderContainer>
                    <ThemeProvider theme={theme}>
                        <RouterProvider router={router} />
                    </ThemeProvider>
                </KeycloakProviderContainer>
            </Provider>
        </SettingsWrapper>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
