// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useContext} from "react"
import {Routes, Route, Navigate} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, NotFoundScreen, PageBanner} from "@sequentech/ui-essentials"
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
import {ApolloContextProvider, ApolloWrapper} from "./providers/ApolloContextProvider"
import {SettingsContext} from "./providers/SettingsContextProvider"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const HeaderWithContext: React.FC = () => {
    const authContext = useContext(AuthContext)

    return (
        <Header
            userProfile={{
                username: authContext.username,
                email: authContext.email,
                openLink: authContext.openProfileLink,
            }}
            logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
        />
    )
}

const App = () => {
    const navigate = useNavigate()
    const {globalSettings} = useContext(SettingsContext)

    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            navigate(
                `/tenant/${globalSettings.DEFAULT_TENANT_ID}/event/${globalSettings.DEFAULT_EVENT_ID}/election-chooser`
            )
        }
    }, [navigate])

    return (
        <StyledApp>
            {globalSettings.DISABLE_AUTH ? <Header /> : <HeaderWithContext />}
            <PageBanner marginBottom="auto">
                <Routes>
                    <Route path="*" element={<NotFoundScreen />} />
                    <Route
                        path="/"
                        element={
                            <Navigate
                                replace
                                to={`/tenant/${globalSettings.DEFAULT_TENANT_ID}/event/${globalSettings.DEFAULT_EVENT_ID}/login`}
                            />
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/login"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <LoginScreen />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election-chooser"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <ElectionSelectionScreen />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/start"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <StartScreen />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/vote"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <VotingScreen />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/review"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <ReviewScreen />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/confirmation"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <ConfirmationScreen />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/audit"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <AuditScreen />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
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
