// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useContext, useMemo, useCallback, useState} from "react"
import {Outlet, ScrollRestoration, useLocation, useParams} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import {
    ELanguageDetectionPolicy,
    EVotingPortalCountdownPolicy,
    IElectionEventPresentation,
    USER_LANGUAGE_COOKIE_NAME,
    setCookie,
    getValueFromCookie,
} from "@sequentech/ui-core"
import Stack from "@mui/material/Stack"
import {useTranslation} from "react-i18next"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
import {SettingsContext} from "./providers/SettingsContextProvider"
import {TenantEventType} from "."
import {ApolloWrapper} from "./providers/ApolloContextProvider"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"
import {useAppDispatch, useAppSelector} from "./store/hooks"
import {selectElectionIds} from "./store/elections/electionsSlice"
import {
    selectBallotStyleByElectionId,
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
} from "./store/ballotStyles/ballotStylesSlice"
import {
    seedElectionEvent,
    selectElectionEventById,
} from "./store/electionEvents/electionEventsSlice"
import WatermarkBackground from "./components/WaterMark/Watermark"
import {useElectionClassName} from "./hooks/useElectionClassName"
import {
    InvalidLoginHintsError,
    parseLoginHints,
    removeLoginHintsFromSearch,
    routeAcceptsLoginHints,
} from "./utils/loginHints"
import {createElectionEventConfigLoader} from "./services/ElectionEventConfig"
import NeutralLoader from "./components/NeutralLoader/NeutralLoader"
import {
    EventStyles,
    eventLogo,
    eventStylesClassName,
    useEventCss,
} from "./components/EventBranding/EventBranding"
import {hidesPlatformBranding, usePageBranding} from "@sequentech/ui-core"
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
            logoUrl={eventLogo(presentation)}
            expiry={{
                alertAt: countdownPolicy?.countdown_alert_anticipation_secs,
                countdown: countdownPolicy?.policy ?? EVotingPortalCountdownPolicy.NO_COUNTDOWN,
                countdownAt: countdownPolicy?.countdown_anticipation_secs,
                endTime: authContext.getExpiry(),
                duration: countdownPolicy?.countdown_anticipation_secs,
            }}
            onChangeLanguage={onChangeLanguage}
        />
    )
}

const App = () => {
    const {t} = useTranslation()
    const navigate = useNavigate()
    const {globalSettings} = useContext(SettingsContext)
    const location = useLocation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {isAuthenticated, setTenantEvent} = useContext(AuthContext)
    const dispatch = useAppDispatch()
    const [loadElectionEventConfig] = useState(createElectionEventConfigLoader)
    const [settledEventConfigUrl, setSettledEventConfigUrl] = useState<string>()
    const [loginHintRequest] = useState(() => {
        const acceptsLoginHints = routeAcceptsLoginHints(location.pathname)

        try {
            const parsed = acceptsLoginHints
                ? parseLoginHints(location.search)
                : {hints: {}, remainingSearch: location.search}
            return {...parsed, pathname: location.pathname, hash: location.hash}
        } catch (error) {
            if (error instanceof InvalidLoginHintsError) {
                const remainingSearch = removeLoginHintsFromSearch(location.search)
                window.history.replaceState(
                    window.history.state,
                    "",
                    `${location.pathname}${remainingSearch}${location.hash}`
                )
                throw new VotingPortalError(VotingPortalErrorType.INVALID_LOGIN_HINT_PARAMETERS)
            }
            throw error
        }
    })
    const loginHintsForCurrentRoute = useMemo(
        () => (loginHintRequest.pathname === location.pathname ? loginHintRequest.hints : {}),
        [location.pathname, loginHintRequest]
    )

    const electionIds = useAppSelector(selectElectionIds)
    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)

    const ballotStyle = useAppSelector((state) => {
        const electionId = electionIds[0] ?? ballotStyleElectionIds[0]

        return electionId ? selectBallotStyleByElectionId(String(electionId))(state) : undefined
    })
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const presentation =
        ballotStyle?.ballot_eml.election_event_presentation ?? electionEvent?.presentation
    const customCss = useEventCss(presentation)
    usePageBranding(presentation)

    useElectionClassName()

    useEffect(() => {
        if (Object.keys(loginHintRequest.hints).length === 0) {
            return
        }

        // Keep validated hints in memory while removing PII from browser history and redirect URIs.
        navigate(
            {
                pathname: loginHintRequest.pathname,
                search: loginHintRequest.remainingSearch,
                hash: loginHintRequest.hash,
            },
            {replace: true}
        )
    }, [loginHintRequest, navigate])

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

    const electionEventConfigUrl = `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/event-${eventId}/election_event_config.json`

    // Set up tenant and event in AuthContext on initial load.
    // It is needed to fetch the election event config file from S3
    // and apply the language policy before loading any other data.
    const setupTenantEvent = useCallback(
        async (isCurrent: () => boolean) => {
            if (!tenantId || !eventId) {
                return
            }

            const isRegisterFlow = location.pathname.includes("/enroll")
            const mode = isRegisterFlow ? "register" : "login"

            try {
                const config = await loadElectionEventConfig(electionEventConfigUrl)
                if (!isCurrent()) {
                    return
                }
                const languageConf = config.election_event_presentation?.language_conf

                const defaultLocale =
                    languageConf?.language_detection_policy ===
                    ELanguageDetectionPolicy.FORCE_DEFAULT
                        ? languageConf.default_language_code
                        : undefined

                setTenantEvent(tenantId, eventId, mode, defaultLocale, loginHintsForCurrentRoute)
            } catch (error) {
                if (!isCurrent()) {
                    return
                }
                setTenantEvent(tenantId, eventId, mode, undefined, loginHintsForCurrentRoute)
            }
        },
        [
            tenantId,
            eventId,
            electionEventConfigUrl,
            loadElectionEventConfig,
            location.pathname,
            loginHintsForCurrentRoute,
            setTenantEvent,
        ]
    )

    // The public config is read on every load of an event route, also when the
    // voter comes back already logged in, so the event's presentation is known
    // before the first full page is shown.
    useEffect(() => {
        if (!tenantId || !eventId) {
            return
        }

        let active = true
        const seedFromConfig = async () => {
            try {
                const config = await loadElectionEventConfig(electionEventConfigUrl)
                if (!active) {
                    return
                }
                // Seed early routes from the public config, but never downgrade a
                // full query result or frozen preview publication already stored.
                dispatch(
                    seedElectionEvent({
                        id: config.election_event_id,
                        tenant_id: config.tenant_id,
                        presentation: config.election_event_presentation,
                    })
                )
            } catch (error) {
                if (!active) {
                    return
                }
                console.error("Error loading election event config:", error)
            }
            setSettledEventConfigUrl(electionEventConfigUrl)
        }
        void seedFromConfig()
        return () => {
            active = false
        }
    }, [tenantId, eventId, electionEventConfigUrl, loadElectionEventConfig, dispatch])

    useEffect(() => {
        if (isAuthenticated) {
            return
        }

        const isDemo = sessionStorage.getItem("isDemo")

        if (!globalSettings.DISABLE_AUTH && isDemo) {
            const areaId = sessionStorage.getItem("areaId")
            const documentId = sessionStorage.getItem("documentId")
            const publicationId = sessionStorage.getItem("publicationId")

            navigate(`/preview/${tenantId}/${documentId}/${areaId}/${publicationId}`)
            window.location.reload()
            return
        }

        let active = true
        void setupTenantEvent(() => active)
        return () => {
            active = false
        }
    }, [isAuthenticated, globalSettings.DISABLE_AUTH, navigate, tenantId, setupTenantEvent])

    // Event routes always end in a login. Until it completes, the voter may be
    // bounced through the identity provider several times, so show one page
    // without the default header, footer or colours across those loads, and
    // keep it until the event's own presentation has been looked up.
    const awaitingLogin = !globalSettings.DISABLE_AUTH && !isAuthenticated
    // The platform's footer waits until the event is known to show it.
    const showFooter = !hidesPlatformBranding(presentation) && (!eventId || !!presentation)
    const awaitingEventConfig = settledEventConfigUrl !== electionEventConfigUrl

    if (eventId && (awaitingLogin || awaitingEventConfig)) {
        return (
            <EventStyles className={eventStylesClassName(presentation)} customCss={customCss}>
                <StyledApp className="voting-portal app-root app-awaiting-login">
                    <NeutralLoader />
                </StyledApp>
            </EventStyles>
        )
    }

    return (
        <EventStyles className={eventStylesClassName(presentation)} customCss={customCss}>
            <StyledApp className="voting-portal app-root">
                <ScrollRestoration />
                <nav className="skip-navigation" aria-label={t("a11y.skipToContent")}>
                    <a className="skip-link" href="#main-content">
                        {t("a11y.skipToContent")}
                    </a>
                </nav>
                <ApolloWrapper>
                    <HeaderWithContext />
                    <PageBanner
                        marginBottom="auto"
                        sx={{display: "flex", position: "relative", flex: 1}}
                        className="main"
                        component="main"
                        id="main-content"
                        tabIndex={-1}
                    >
                        <WatermarkBackground />
                        <Outlet />
                    </PageBanner>
                </ApolloWrapper>
                {showFooter && <Footer />}
            </StyledApp>
        </EventStyles>
    )
}

export default App
