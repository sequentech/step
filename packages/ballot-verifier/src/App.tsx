// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {Routes, Route, useNavigate, Navigate} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, NotFoundScreen, PageBanner} from "@sequentech/ui-essentials"
import {HomeScreen} from "./screens/HomeScreen"
import {ConfirmationScreen} from "./screens/ConfirmationScreen"
import Stack from "@mui/material/Stack"
import {IConfirmationBallot, provideBallotService} from "./services/BallotService"
import {AuthContext} from "./providers/AuthContextProvider"
import {RouteParameterProvider} from "."
import {ApolloContextProvider, ApolloWrapper} from "./providers/ApolloContextProvider"
import {LoginScreen} from "./screens/LoginScreen"
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
    const [confirmationBallot, setConfirmationBallot] = useState<IConfirmationBallot | null>(null)
    const [ballotId, setBallotId] = useState<string>("")
    const [fileName, setFileName] = useState("")
    const ballotService = provideBallotService()

    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            navigate(
                `/tenant/${globalSettings.DEFAULT_TENANT_ID}/event/${globalSettings.DEFAULT_EVENT_ID}/start`
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
                        path="/tenant/:tenantId/event/:eventId/start"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <HomeScreen
                                            confirmationBallot={confirmationBallot}
                                            setConfirmationBallot={setConfirmationBallot}
                                            ballotId={ballotId}
                                            setBallotId={setBallotId}
                                            fileName={fileName}
                                            setFileName={setFileName}
                                            ballotService={ballotService}
                                        />
                                    </ApolloWrapper>
                                </ApolloContextProvider>
                            </RouteParameterProvider>
                        }
                    />
                    <Route
                        path="/tenant/:tenantId/event/:eventId/confirmation"
                        element={
                            <RouteParameterProvider>
                                <ApolloContextProvider>
                                    <ApolloWrapper>
                                        <ConfirmationScreen
                                            confirmationBallot={confirmationBallot}
                                            ballotId={ballotId}
                                            ballotService={ballotService}
                                        />
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
