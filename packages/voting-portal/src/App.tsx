// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useContext, useMemo} from "react"
import {Outlet, ScrollRestoration, useLocation, useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {
    EVotingPortalCountdownPolicy,
    Footer,
    Header,
    IElectionEventPresentation,
    PageBanner,
} from "@sequentech/ui-essentials"
import Stack from "@mui/material/Stack"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
import {SettingsContext} from "./providers/SettingsContextProvider"
import {TenantEventType} from "."
import {ApolloWrapper} from "./providers/ApolloContextProvider"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"
import {useAppSelector} from "./store/hooks"
import {selectElectionIds} from "./store/elections/electionsSlice"
import {
    selectBallotStyleByElectionId,
    selectFirstBallotStyle,
} from "./store/ballotStyles/ballotStylesSlice"
import WatermarkBackground from "./components/WaterMark/Watermark"

const StyledApp = styled(Stack)<{css: string}>`
    min-height: 100vh;
    ${({css}) => css}
`

const HeaderWithContext: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const {eventId} = useParams<TenantEventType>()

    const ballotStyle = useAppSelector(selectFirstBallotStyle)

    let presentation: IElectionEventPresentation | undefined =
        ballotStyle?.ballot_eml.election_event_presentation

    let languagesList = presentation?.language_conf?.enabled_language_codes ?? ["en"]
    let showUserProfile = presentation?.show_user_profile ?? true
    const countdownPolicy = useMemo(() => {
        return ballotStyle?.ballot_eml.election_event_presentation?.voting_portal_countdown_policy
    }, [ballotStyle])

    return (
        <Header
            appVersion={{main: globalSettings.APP_VERSION}}
            userProfile={{
                username: authContext.username,
                email: authContext.email,
                openLink: showUserProfile ? authContext.openProfileLink : undefined,
            }}
            languagesList={languagesList}
            logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
            logoUrl={presentation?.logo_url}
            expiry={{
                alertAt: countdownPolicy?.countdown_alert_anticipation_secs,
                countdown: countdownPolicy?.policy ?? EVotingPortalCountdownPolicy.NO_COUNTDOWN,
                countdownAt: countdownPolicy?.countdown_anticipation_secs,
                endTime: authContext.getExpiry(),
            }}
        />
    )
}

const App = () => {
    const navigate = useNavigate()
    const {globalSettings} = useContext(SettingsContext)
    const location = useLocation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {isAuthenticated, setTenantEvent} = useContext(AuthContext)

    const electionIds = useAppSelector(selectElectionIds)
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionIds[0])))
    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            navigate(
                `/tenant/${globalSettings.DEFAULT_TENANT_ID}/event/${globalSettings.DEFAULT_EVENT_ID}/election-chooser${location.search}`
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
        <StyledApp
            className="app-root"
            css={ballotStyle?.ballot_eml.election_event_presentation?.css ?? ""}
        >
            <ScrollRestoration />
            <ApolloWrapper>
                {globalSettings.DISABLE_AUTH ? <Header /> : <HeaderWithContext />}
                <PageBanner
                    marginBottom="auto"
                    sx={{display: "flex", position: "relative", flex: 1}}
                >
                    <WatermarkBackground />
                    <Outlet />
                </PageBanner>
            </ApolloWrapper>
            <Footer />
        </StyledApp>
    )
}

export default App
