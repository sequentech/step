// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { useEffect, useContext } from "react"
import {Routes, Route, useLocation} from "react-router-dom"
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
import { useNavigate } from "react-router"
import { AuthContext } from "./providers/AuthContextProvider"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const App = () => {
    const location = useLocation()
    const navigate = useNavigate()
    const authContext = useContext(AuthContext)

    useEffect(() => {
        console.log(location.pathname)
        if (location.pathname !== "/" && !authContext.isAuthenticated) {
            navigate("/")
        }
    }, [location.pathname, authContext.isAuthenticated])

    return (
        <StyledApp>
            <Header />
            <PageBanner marginBottom="auto">
                <Routes>
                    <Route path="/" element={<LoginScreen />} />
                    <Route path="/election-chooser" element={<ElectionSelectionScreen />} />
                    <Route path="/election/:electionId/start" element={<StartScreen />} />
                    <Route path="/election/:electionId/vote" element={<VotingScreen />} />
                    <Route path="/election/:electionId/review" element={<ReviewScreen />} />
                    <Route
                        path="/election/:electionId/confirmation"
                        element={<ConfirmationScreen />}
                    />
                    <Route path="/election/:electionId/audit" element={<AuditScreen />} />
                </Routes>
            </PageBanner>
            <Footer />
        </StyledApp>
    )
}

export default App
