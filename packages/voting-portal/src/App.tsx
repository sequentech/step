// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useContext, useMemo, useCallback, useState} from "react"
import {Outlet, ScrollRestoration, useLocation, useParams} from "react-router-dom"
import {ELanguageDetectionPolicy, IElectionEventPresentation} from "@sequentech/ui-core"
import {useNavigate} from "react-router-dom"
import {AuthContext} from "./providers/AuthContextProvider"
import {SettingsContext} from "./providers/SettingsContextProvider"
import {TenantEventType} from "."
import {ApolloWrapper} from "./providers/ApolloContextProvider"
import {VotingPortalError, VotingPortalErrorType} from "./services/VotingPortalError"
import {useAppDispatch} from "./store/hooks"
import {seedElectionEvent} from "./store/electionEvents/electionEventsSlice"
import {PortalChrome} from "./components/PortalChrome"
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

const App = () => {
    const navigate = useNavigate()
    const {globalSettings} = useContext(SettingsContext)
    const location = useLocation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {isAuthenticated, setTenantEvent} = useContext(AuthContext)
    const dispatch = useAppDispatch()
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

            // Seed early routes from the public config, but never downgrade a
            // full query result or frozen preview publication already stored.
            dispatch(
                seedElectionEvent({
                    id: config.election_event_id,
                    tenant_id: config.tenant_id,
                    presentation,
                })
            )

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
        dispatch,
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
        <PortalChrome Wrapper={ApolloWrapper} before={<ScrollRestoration />}>
            <Outlet />
        </PortalChrome>
    )
}

export default App
