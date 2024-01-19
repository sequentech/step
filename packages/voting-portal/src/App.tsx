// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useContext} from "react"
import {Outlet, useLocation, useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner, i18n} from "@sequentech/ui-essentials"
import Stack from "@mui/material/Stack"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
import {SettingsContext} from "./providers/SettingsContextProvider"
import {TenantEventType} from "."
import {ApolloWrapper} from "./providers/ApolloContextProvider"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`
//

const HeaderWithContext: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    return (
        <Header
            appVersion={{main: globalSettings.APP_VERSION}}
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
    const location = useLocation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {isAuthenticated, setTenantEvent} = useContext(AuthContext)

    useEffect(() => {
        const dir = i18n.dir(i18n.language)
        document.documentElement.dir = dir
    }, [])

    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            navigate(
                `/tenant/${globalSettings.DEFAULT_TENANT_ID}/event/${globalSettings.DEFAULT_EVENT_ID}/election-chooser`
            )
        } else {
            if (location.pathname === "/") {
                throw new VotingPortalError(VotingPortalErrorType.NO_ELECTION_EVENT)
            }
        }
    }, [
        globalSettings.DEFAULT_TENANT_ID,
        globalSettings.DEFAULT_EVENT_ID,
        globalSettings.DISABLE_AUTH,
        navigate,
        location.pathname,
    ])

    useEffect(() => {
        if (!isAuthenticated && !!tenantId && !!eventId) {
            setTenantEvent(tenantId, eventId)
        }
    }, [tenantId, eventId, isAuthenticated, setTenantEvent])

    return (
        <StyledApp>
            {globalSettings.DISABLE_AUTH ? <Header /> : <HeaderWithContext />}
            <PageBanner marginBottom="auto">
                <ApolloWrapper>
                    <Outlet />
                </ApolloWrapper>
            </PageBanner>
            <Footer />
        </StyledApp>
    )
}

export default App
