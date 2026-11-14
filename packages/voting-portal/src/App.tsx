// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useContext, useMemo, useCallback, useState} from "react"
import {Outlet, ScrollRestoration, useLocation, useParams} from "react-router-dom"
import {ELanguageDetectionPolicy} from "@sequentech/ui-core"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
import {SettingsContext} from "./providers/SettingsContextProvider"
import {TenantEventType} from "."
import {ApolloWrapper} from "./providers/ApolloContextProvider"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"
import {useAppDispatch} from "./store/hooks"
import {seedElectionEvent} from "./store/electionEvents/electionEventsSlice"
import {PortalChrome, PortalLoading} from "./components/PortalChrome"
import {
    InvalidLoginHintsError,
    parseLoginHints,
    removeLoginHintsFromSearch,
    routeAcceptsLoginHints,
} from "./utils/loginHints"
import {PREVIEW_FILE_KEY} from "./routes/PreviewFromFile"
import {createElectionEventConfigLoader} from "./services/ElectionEventConfig"

const App = () => {
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
                console.error("Error loading election event config:", error)
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
    const awaitingEventConfig = settledEventConfigUrl !== electionEventConfigUrl

    if (eventId && (awaitingLogin || awaitingEventConfig)) {
        return <PortalLoading />
    }

    return (
        <PortalChrome Wrapper={ApolloWrapper} before={<ScrollRestoration />}>
            <Outlet />
        </PortalChrome>
    )
}

export default App
