// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useMemo} from "react"
import {Outlet, ScrollRestoration, useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import {
    EVotingPortalCountdownPolicy,
    IElectionEventPresentation,
    USER_LANGUAGE_COOKIE_NAME,
    setCookie,
} from "@sequentech/ui-core"
import Stack from "@mui/material/Stack"
import {AuthContext} from "@voting-portal/providers/AuthContextProvider"
import {SettingsContext} from "@voting-portal/providers/SettingsContextProvider"
import {useAppSelector} from "@voting-portal/store/hooks"
import {selectFirstBallotStyle} from "@voting-portal/store/ballotStyles/ballotStylesSlice"
import {selectElectionEventById} from "@voting-portal/store/electionEvents/electionEventsSlice"
import WatermarkBackground from "@voting-portal/components/WaterMark/Watermark"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {LOC_STUDIO_LANGUAGES} from "./i18n"
import {useLocStudio} from "./LocStudioContext"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const StyledAppWrapper = styled(Stack)<{customCss: string}>`
    ${({customCss}) => customCss}
`

const HeaderWithStudioContext: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const {eventId} = useParams<{tenantId?: string; eventId?: string}>()
    const {setLanguage} = useLocStudio()

    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))

    const presentation: IElectionEventPresentation | undefined =
        ballotStyle?.ballot_eml.election_event_presentation ??
        electionEvent?.presentation ??
        undefined

    const languagesList = presentation?.language_conf?.enabled_language_codes ?? [
        ...LOC_STUDIO_LANGUAGES,
    ]
    const showUserProfile = presentation?.show_user_profile ?? true
    const countdownPolicy = useMemo(() => presentation?.voting_portal_countdown_policy, [presentation])

    const logoImg =
        presentation?.logo_url === undefined
            ? BlankLogoImg
            : presentation?.logo_url === null
              ? SequentLogo
              : presentation.logo_url

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
            onChangeLanguage={(lang) => {
                setCookie(USER_LANGUAGE_COOKIE_NAME, lang)
                setLanguage(lang)
            }}
        />
    )
}

export const StudioShell: React.FC = () => {
    const ballotStyle = useAppSelector(selectFirstBallotStyle)

    return (
        <StyledAppWrapper
            customCss={ballotStyle?.ballot_eml.election_event_presentation?.css ?? ""}
        >
            <StyledApp className="voting-portal app-root">
                <ScrollRestoration />
                <HeaderWithStudioContext />
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
                <Footer />
            </StyledApp>
        </StyledAppWrapper>
    )
}
