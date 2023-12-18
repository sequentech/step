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
import { ApolloWrapper } from "./providers/ApolloContextProvider"

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
                            <ApolloWrapper>
                                <LoginScreen />
                            </ApolloWrapper>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election-chooser"
                        element={
                            <ApolloWrapper>
                                <ElectionSelectionScreen />
                            </ApolloWrapper>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/start"
                        element={
                            <ApolloWrapper>
                                <StartScreen />
                            </ApolloWrapper>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/vote"
                        element={
                            <ApolloWrapper>
                                <VotingScreen />
                            </ApolloWrapper>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/review"
                        element={
                            <ApolloWrapper>
                                <ReviewScreen />
                            </ApolloWrapper>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/confirmation"
                        element={
                            <ApolloWrapper>
                                <ConfirmationScreen />
                            </ApolloWrapper>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/election/:electionId/audit"
                        element={
                            <ApolloWrapper>
                                <AuditScreen />
                            </ApolloWrapper>
                        }
                    />
                </Routes>
            </PageBanner>
            <Footer />
        </StyledApp>
    )
}

export default App
