// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useContext} from "react"
import {Outlet} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import Stack from "@mui/material/Stack"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
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
    }, [
        globalSettings.DEFAULT_TENANT_ID,
        globalSettings.DEFAULT_EVENT_ID,
        globalSettings.DISABLE_AUTH,
        navigate,
    ])

    return (
        <StyledApp>
            {globalSettings.DISABLE_AUTH ? <Header /> : <HeaderWithContext />}
            <PageBanner marginBottom="auto">
                <Outlet />
            </PageBanner>
            <Footer />
        </StyledApp>
    )
}

export default App
