// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useContext, useMemo} from "react"
import {Outlet, ScrollRestoration, useLocation, useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import {EVotingPortalCountdownPolicy, IElectionEventPresentation} from "@sequentech/ui-core"
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
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"

const StyledApp = styled(Stack)<{css: string}>`
    min-height: 100vh;
    ${({css}) => css}

    /* Visually hidden until focused, then shown for keyboard users */
    .skip-link {
        position: absolute;
        top: -40px;
        left: 0;
        background: #fff;
        color: #000;
        padding: 8px 12px;
        z-index: 1000;
        text-decoration: none;
    }
    .skip-link:focus {
        top: 0;
    }
`

const StyledMain = styled(`main`)`
    margin-bottom: auto;
    display: flex;
    position: relative;
    flex: 1;
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

    const logoImg =
        presentation?.logo_url === undefined
            ? BlankLogoImg
            : presentation?.logo_url === null
            ? SequentLogo
            : presentation?.logo_url

    return (
        <Header
            appVersion={{main: globalSettings.APP_VERSION}}
            appHash={{main: globalSettings.APP_HASH}}
            userProfile={{
                firstName: authContext.firstName,
                username: authContext.username,
                email: authContext.email,
                openLink: showUserProfile ? authContext.openProfileLink : undefined,
            }}
            languagesList={languagesList}
            logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
            logoUrl={logoImg}
            expiry={{
                alertAt: countdownPolicy?.countdown_alert_anticipation_secs,
                countdown: countdownPolicy?.policy ?? EVotingPortalCountdownPolicy.NO_COUNTDOWN,
                countdownAt: countdownPolicy?.countdown_anticipation_secs,
                endTime: authContext.getExpiry(),
                duration: countdownPolicy?.countdown_anticipation_secs,
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
        if (location.pathname === "/") {
            throw new VotingPortalError(VotingPortalErrorType.NO_ELECTION_EVENT)
        }
    }, [
        globalSettings.DEFAULT_TENANT_ID,
        globalSettings.DEFAULT_EVENT_ID,
        globalSettings.DISABLE_AUTH,
        navigate,
        location.pathname,
    ])

    useEffect(() => {
        const isDemo = sessionStorage.getItem("isDemo")

        if (!isAuthenticated && !globalSettings.DISABLE_AUTH && isDemo) {
            const areaId = sessionStorage.getItem("areaId")
            const documentId = sessionStorage.getItem("documentId")
            const publicationId = sessionStorage.getItem("publicationId")
            navigate(`/preview/${tenantId}/${documentId}/${areaId}/${publicationId}`)
            window.location.reload()
        } else if (!isAuthenticated && !!tenantId && !!eventId) {
            setTenantEvent(
                tenantId,
                eventId,
                location.pathname.includes("/enroll") ? "register" : "login"
            )
        }
    }, [tenantId, eventId, isAuthenticated, setTenantEvent, globalSettings.DISABLE_AUTH])

    return (
        <StyledApp
            className="app-root"
            css={ballotStyle?.ballot_eml.election_event_presentation?.css ?? ""}
        >
            <ScrollRestoration />
            <ApolloWrapper>
                {/* Site header landmark */}
                <HeaderWithContext />
                {/* Main landmark for all page content */}
                <PageBanner
                    marginBottom="auto"
                    sx={{display: "flex", position: "relative", flex: 1}}
                    className="main"
                    component="main"
                    id="main-content"
                    role="main"
                >
                    <WatermarkBackground />
                    <Outlet />
                </PageBanner>
            </ApolloWrapper>
            {/* Footer landmark */}
            <Footer />
        </StyledApp>
    )
}

export default App
