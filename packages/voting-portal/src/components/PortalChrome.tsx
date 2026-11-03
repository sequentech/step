// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useMemo} from "react"
import {useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import Stack from "@mui/material/Stack"
import Snackbar from "@mui/material/Snackbar"
import Alert, {type AlertColor} from "@mui/material/Alert"
import ContactPhoneOutlinedIcon from "@mui/icons-material/ContactPhoneOutlined"
import {useTranslation} from "react-i18next"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import {
    EVotingPortalCountdownPolicy,
    IElectionEventPresentation,
    USER_LANGUAGE_COOKIE_NAME,
    setCookie,
    getValueFromCookie,
    sanitizePresentationCss,
} from "@sequentech/ui-core"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {
    AuthContext,
    AuthContextValues,
    ContactChangeOutcome,
} from "../providers/AuthContextProvider"
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

const OUTCOME_NOTICES: Record<ContactChangeOutcome, {key: string; severity: AlertColor}> = {
    [ContactChangeOutcome.Success]: {key: "contactChannel.updated", severity: "success"},
    [ContactChangeOutcome.Cancelled]: {key: "contactChannel.cancelled", severity: "info"},
    [ContactChangeOutcome.Error]: {key: "contactChannel.failed", severity: "error"},
}

/**
 * How the contact change ended, when the voter comes back from Keycloak. It stays until the voter
 * closes it, so it can be read at any pace.
 */
const ContactChangeNotice: React.FC = () => {
    const {t} = useTranslation()
    const {contactChangeOutcome, clearContactChangeOutcome} = useContext(AuthContext)
    if (contactChangeOutcome === undefined) {
        return null
    }
    const notice = OUTCOME_NOTICES[contactChangeOutcome]
    return (
        <Snackbar
            open
            className="contact-change-notice"
            anchorOrigin={{vertical: "top", horizontal: "center"}}
            onClose={(_event, reason) => {
                // Only the voter's own close dismisses it, never a click elsewhere.
                if (reason !== "clickaway") {
                    clearContactChangeOutcome?.()
                }
            }}
        >
            <Alert
                className="contact-change-alert"
                severity={notice.severity}
                onClose={() => clearContactChangeOutcome?.()}
                closeText={t("contactChannel.close")}
            >
                {t(notice.key)}
            </Alert>
        </Snackbar>
    )
}

const HeaderWithContext: React.FC = () => {
    const {t} = useTranslation()
    // Offered only to a signed-in voter, when the event has a channel for codes. A context
    // without these (one that can't reach Keycloak, such as the embedded preview) offers none.
    const authContext: AuthContextValues = useContext(AuthContext)
    const {changeContactChannel} = authContext
    const showContactChange =
        authContext.isAuthenticated &&
        authContext.contactChangeAvailable === true &&
        changeContactChannel !== undefined
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
                menuActions: showContactChange
                    ? [
                          {
                              id: "contact-method",
                              label: t("contactChannel.change"),
                              icon: (
                                  <ContactPhoneOutlinedIcon
                                      className="contact-method-icon"
                                      aria-hidden
                                  />
                              ),
                              onSelect: () => void changeContactChannel(),
                          },
                      ]
                    : undefined,
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
    const {globalSettings} = useContext(SettingsContext)
    const {eventId} = useParams<TenantEventType>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const electionIds = useAppSelector(selectElectionIds)
    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)

    const ballotStyle = useAppSelector((state) => {
        const electionId = electionIds[0] ?? ballotStyleElectionIds[0]

        return electionId ? selectBallotStyleByElectionId(String(electionId))(state) : undefined
    })

    const presentation =
        ballotStyle?.ballot_eml.election_event_presentation ?? electionEvent?.presentation
    const css = presentation?.css ?? ""
    const customCss = useMemo(
        () =>
            sanitizePresentationCss(css, {
                baseUrl: document.baseURI,
                publicBucketUrl: globalSettings.PUBLIC_BUCKET_URL,
            }),
        [css, globalSettings.PUBLIC_BUCKET_URL]
    )

    useElectionClassName()

    return (
        <StyledAppWrapper className="voting-portal-wrapper" customCss={customCss}>
            <StyledApp className="voting-portal app-root">
                {before}
                <nav className="skip-navigation" aria-label={t("a11y.skipToContent")}>
                    <a className="skip-link" href="#main-content">
                        {t("a11y.skipToContent")}
                    </a>
                </nav>
                <Wrapper>
                    <HeaderWithContext />
                    <ContactChangeNotice />
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
