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
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
} from "./store/ballotStyles/ballotStylesSlice"
import {selectElectionEventById} from "./store/electionEvents/electionEventsSlice"
import WatermarkBackground from "./components/WaterMark/Watermark"
import {BallotSelectionAdapter} from "./components/BallotSelectionAdapter"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {useElectionClassName} from "./hooks/useElectionClassName"
import {
    InvalidLoginHintsError,
    parseLoginHints,
    removeLoginHintsFromSearch,
    routeAcceptsLoginHints,
} from "./utils/loginHints"
import {PREVIEW_FILE_KEY} from "./routes/PreviewFromFile"
interface ElectionEventConfigDocument {
    id: string
    tenant_id: string
    election_event_id: string
    election_event_presentation: IElectionEventPresentation
}
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
        />
    )
}

const App = () => {
    const navigate = useNavigate()
    const {globalSettings} = useContext(SettingsContext)
    const location = useLocation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {isAuthenticated, setTenantEvent} = useContext(AuthContext)
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
    const setupTenantEvent = useCallback(async () => {
        if (!tenantId || !eventId) {
            return
        }

        const isRegisterFlow = location.pathname.includes("/enroll")
        const mode = isRegisterFlow ? "register" : "login"

        try {
            const response = await fetch(electionEventConfigUrl)

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`)
            }

            const config = (await response.json()) as ElectionEventConfigDocument
            const presentation = config.election_event_presentation
            const languageConf = presentation?.language_conf

            const defaultLocale =
                languageConf?.language_detection_policy === ELanguageDetectionPolicy.FORCE_DEFAULT
                    ? languageConf.default_language_code
                    : undefined

            setTenantEvent(tenantId, eventId, mode, defaultLocale, loginHintsForCurrentRoute)
        } catch (error) {
            console.error("Error loading election event config:", error)
            setTenantEvent(tenantId, eventId, mode, undefined, loginHintsForCurrentRoute)
        }
    }, [
        tenantId,
        eventId,
        electionEventConfigUrl,
        location.pathname,
        loginHintsForCurrentRoute,
        setTenantEvent,
    ])

    useEffect(() => {
        if (isAuthenticated) {
            return
        }

        const isDemo = sessionStorage.getItem("isDemo")

        if (!globalSettings.DISABLE_AUTH && isDemo) {
            // A preview opened from a file has no bucket coordinates to go back
            // to, so it goes back to the page that holds it. Without this branch
            // it would be sent to `/preview/undefined/undefined/…` and land on a
            // blank screen.
            if (sessionStorage.getItem(PREVIEW_FILE_KEY)) {
                navigate("/preview/file")
                window.location.reload()
                return
            }

            const areaId = sessionStorage.getItem("areaId")
            const documentId = sessionStorage.getItem("documentId")
            const publicationId = sessionStorage.getItem("publicationId")

            navigate(`/preview/${tenantId}/${documentId}/${areaId}/${publicationId}`)
            window.location.reload()
            return
        }

        void setupTenantEvent()
    }, [isAuthenticated, globalSettings.DISABLE_AUTH, navigate, tenantId, setupTenantEvent])

    return (
        <StyledAppWrapper
            customCss={ballotStyle?.ballot_eml.election_event_presentation?.css ?? ""}
        >
            <StyledApp className="voting-portal app-root">
                <ScrollRestoration />
                <ApolloWrapper>
                    <HeaderWithContext />
                    <PageBanner
                        marginBottom="auto"
                        sx={{display: "flex", position: "relative", flex: 1}}
                        className="main"
                        component="main"
                        id="main-content"
                        role="main"
                    >
                        <WatermarkBackground />
                        {/* The shared ballot asks a port for the voter's marks
                            rather than reading this app's store, so that the
                            Election Architect can render the same components over
                            its own state. Supplied once, here, rather than per
                            screen: two screens draw contests today and a third
                            would otherwise be a silent failure at the first
                            click. */}
                        <BallotSelectionAdapter>
                            <Outlet />
                        </BallotSelectionAdapter>
                    </PageBanner>
                </ApolloWrapper>
                <Footer />
            </StyledApp>
        </StyledAppWrapper>
    )
}

export default App
