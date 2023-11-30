// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useContext} from "react"
import {Routes, Route, useLocation, Navigate} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import Stack from "@mui/material/Stack"
import {StartScreen} from "./screens/StartScreen"
import {VotingScreen} from "./screens/VotingScreen"
import {ReviewScreen} from "./screens/ReviewScreen"
import {ConfirmationScreen} from "./screens/ConfirmationScreen"
import {AuditScreen} from "./screens/AuditScreen"
import {ElectionSelectionScreen} from "./screens/ElectionSelectionScreen"
import {LoginScreen} from "./screens/LoginScreen"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
import {RouteParameterProvider} from "."
import {DISABLE_AUTH, DEFAULT_TENANT_ID, DEFAULT_EVENT_ID} from "./Config"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const HeaderWithContext: React.FC = () => {
    const authContext = useContext(AuthContext)

    return <Header logoutFn={authContext.isAuthenticated ? authContext.logout : undefined} />
}

const App = () => {
    const navigate = useNavigate()

    useEffect(() => {
        if (DISABLE_AUTH) {
            navigate(`/tenant/${DEFAULT_TENANT_ID}/event/${DEFAULT_EVENT_ID}/election-chooser`)
        }
    }, [navigate])

    return (
        <StyledApp>
            {DISABLE_AUTH ? <Header /> : <HeaderWithContext />}
            <PageBanner marginBottom="auto">
                <Routes>
                    <Route
                        path="/"
                        element={
                            <Navigate
                                replace
                                to={`/tenant/${DEFAULT_TENANT_ID}/event/${DEFAULT_EVENT_ID}/login`}
                            />
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/login"
                        element={
                            <RouteParameterProvider>
                                <LoginScreen />
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election-chooser"
                        element={
                            <RouteParameterProvider>
                                <ElectionSelectionScreen />
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/start"
                        element={
                            <RouteParameterProvider>
                                <StartScreen />
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/vote"
                        element={
                            <RouteParameterProvider>
                                <VotingScreen />
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/review"
                        element={
                            <RouteParameterProvider>
                                <ReviewScreen />
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/confirmation"
                        element={
                            <RouteParameterProvider>
                                <ConfirmationScreen />
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/audit"
                        element={
                            <RouteParameterProvider>
                                <AuditScreen />
                            </RouteParameterProvider>
                        }
                    />
                </Routes>
            </PageBanner>
            <Footer />
        </StyledApp>
    )
}

export default App
