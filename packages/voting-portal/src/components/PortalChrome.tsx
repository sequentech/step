// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useMemo} from "react"
import {useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import Stack from "@mui/material/Stack"
import {useTranslation} from "react-i18next"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import {
    EVotingPortalCountdownPolicy,
    IElectionEventPresentation,
    USER_LANGUAGE_COOKIE_NAME,
    setCookie,
    getValueFromCookie,
} from "@sequentech/ui-core"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {AuthContext} from "../providers/AuthContextProvider"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {TenantEventType} from ".."
import {useAppSelector} from "../store/hooks"
import {selectElectionIds} from "../store/elections/electionsSlice"
import {
    selectBallotStyleByElectionId,
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import WatermarkBackground from "./WaterMark/Watermark"
import {BallotSelectionAdapter} from "./BallotSelectionAdapter"
import {ScreenAudioInstructions} from "./ScreenAudioInstructions/ScreenAudioInstructions"
import {useElectionClassName} from "../hooks/useElectionClassName"

const StyledApp = styled(Stack)`
    min-height: 100vh;

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

const StyledAppWrapper = styled(Stack)<{customCss: string}>`
    ${({customCss}) => customCss}
`

const HeaderWithContext: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const {eventId} = useParams<TenantEventType>()

    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))

    let presentation: IElectionEventPresentation | undefined =
        ballotStyle?.ballot_eml.election_event_presentation ??
        electionEvent?.presentation ??
        undefined

    let languagesList = presentation?.language_conf?.enabled_language_codes ?? ["en"]
    let showUserProfile = presentation?.show_user_profile ?? true
    const countdownPolicy = useMemo(() => {
        return presentation?.voting_portal_countdown_policy
    }, [presentation])

    const logoImg =
        presentation?.logo_url === undefined
            ? BlankLogoImg
            : presentation?.logo_url === null
              ? SequentLogo
              : presentation?.logo_url

    const onChangeLanguage = (lang: string) => {
        if (getValueFromCookie(USER_LANGUAGE_COOKIE_NAME) !== lang) {
            setCookie(USER_LANGUAGE_COOKIE_NAME, lang)
        }
    }

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
            onChangeLanguage={onChangeLanguage}
            accessibilitySettingsPolicy={presentation?.voter_accessibility_settings_policy}
        />
    )
}

export interface PortalChromeProps extends React.PropsWithChildren {
    /** Wraps the header and the main content, e.g. the portal's authenticated Apollo client. */
    Wrapper?: React.ComponentType<React.PropsWithChildren>
    /** Rendered first, before the skip link, e.g. the router's scroll restoration. */
    before?: React.ReactNode
}

const Unwrapped: React.FC<React.PropsWithChildren> = ({children}) => <>{children}</>

/**
 * What surrounds every voter screen: the event's stylesheet, the header with its logo and
 * languages, the demo watermark and the footer. The portal's application route and the
 * embedded voter preview both render the screens inside it.
 */
export const PortalChrome: React.FC<PortalChromeProps> = ({
    Wrapper = Unwrapped,
    before,
    children,
}) => {
    const {t} = useTranslation()
    const {eventId} = useParams<TenantEventType>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const electionIds = useAppSelector(selectElectionIds)
    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)

    const ballotStyle = useAppSelector((state) => {
        const electionId = electionIds[0] ?? ballotStyleElectionIds[0]

        return electionId ? selectBallotStyleByElectionId(String(electionId))(state) : undefined
    })

    useElectionClassName()

    return (
        <StyledAppWrapper
            className="voting-portal-wrapper"
            customCss={
                ballotStyle?.ballot_eml.election_event_presentation?.css ??
                electionEvent?.presentation?.css ??
                ""
            }
        >
            <StyledApp className="voting-portal app-root">
                {before}
                <a className="skip-link" href="#main-content">
                    {t("a11y.skipToContent")}
                </a>
                <Wrapper>
                    <HeaderWithContext />
                    <PageBanner
                        marginBottom="auto"
                        sx={{
                            display: "flex",
                            position: "relative",
                            flex: 1,
                            justifyContent: "flex-start",
                        }}
                        className="main"
                        component="main"
                        id="main-content"
                        tabIndex={-1}
                    >
                        <WatermarkBackground />
                        <ScreenAudioInstructions />
                        {/* The shared ballot asks a port for the voter's marks
                            rather than reading this app's store, so that the
                            Election Architect can render the same components over
                            its own state. Supplied once, here, rather than per
                            screen: two screens draw contests today and a third
                            would otherwise be a silent failure at the first
                            click. */}
                        <BallotSelectionAdapter>{children}</BallotSelectionAdapter>
                    </PageBanner>
                </Wrapper>
                <Footer />
            </StyledApp>
        </StyledAppWrapper>
    )
}
