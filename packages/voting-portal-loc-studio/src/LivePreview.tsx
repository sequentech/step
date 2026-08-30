// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useRef} from "react"
import {Provider} from "react-redux"
import {ApolloProvider} from "@apollo/client/react"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {AuthContext, AuthContextValues} from "@voting-portal/providers/AuthContextProvider"
import {
    GlobalSettings,
    SettingsContext,
} from "@voting-portal/providers/SettingsContextProvider"
import ElectionSelectionScreen from "@voting-portal/routes/ElectionSelectionScreen"
import StartScreen from "@voting-portal/routes/StartScreen"
import VotingScreen from "@voting-portal/routes/VotingScreen"
import ReviewScreen from "@voting-portal/routes/ReviewScreen"
import ConfirmationScreen from "@voting-portal/routes/ConfirmationScreen"
import AuditScreen from "@voting-portal/routes/AuditScreen"
import BallotLocator from "@voting-portal/routes/BallotLocator"
import SupportMaterialsScreen from "@voting-portal/routes/SupportMaterialsScreen"
import {ErrorPage} from "@voting-portal/routes/ErrorPage"
import {VotingPortalError, VotingPortalErrorType} from "@voting-portal/services/VotingPortalError"
import {StudioShell} from "./StudioShell"
import {StudioRouteError} from "./StudioRouteError"
import {createStudioApollo} from "./studioApollo"
import {PreviewErrorBoundary} from "./PreviewErrorBoundary"
import {useCurrentScene, useLocStudio} from "./LocStudioContext"
import {
    createStudioStore,
    createStudioStoreFromUpload,
    disableAuthFor,
    electionIdForScene,
    STUDIO_BALLOT_ID,
    STUDIO_EVENT_ID,
    STUDIO_TENANT_ID,
} from "./studioStore"
import {UploadedElectionEvent} from "./uploadedElection"

const defaultSettings: GlobalSettings = {
    DISABLE_AUTH: true,
    QUERY_POLL_INTERVAL_MS: 2000,
    DEFAULT_TENANT_ID: STUDIO_TENANT_ID,
    DEFAULT_EVENT_ID: STUDIO_EVENT_ID,
    ONLINE_VOTING_CLIENT_ID: "voting-portal",
    KEYCLOAK_URL: "http://127.0.0.1:8090/",
    HASURA_URL: "http://localhost:8080/v1/graphql",
    APP_VERSION: "loc-studio",
    APP_HASH: "dev",
    BALLOT_VERIFIER_URL: "http://127.0.0.1:3001/",
    PUBLIC_BUCKET_URL: "http://127.0.0.1:9002/public/",
    KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS: 900,
    POLLING_DURATION_TIMEOUT: 12000,
}

const ThrowCertAuthError = (): React.ReactElement => {
    throw new VotingPortalError(VotingPortalErrorType.CERT_AUTH_FAILED)
}

const ThrowGenericError = (): React.ReactElement => {
    throw new Error("Unexpected error")
}

const routeError = {errorElement: <StudioRouteError />}

const studioRoutes = [
    {
        path: "/cert-auth-error",
        element: <ThrowCertAuthError />,
        errorElement: <ErrorPage />,
    },
    {
        path: "/error",
        element: <ThrowGenericError />,
        errorElement: <ErrorPage />,
    },
    {
        path: "/tenant/:tenantId/event/:eventId",
        element: <StudioShell />,
        errorElement: <StudioRouteError />,
        children: [
            {path: "election-chooser", element: <ElectionSelectionScreen />, ...routeError},
            {path: "election/:electionId/start", element: <StartScreen />, ...routeError},
            {path: "election/:electionId/vote", element: <VotingScreen />, ...routeError},
            {path: "election/:electionId/review", element: <ReviewScreen />, ...routeError},
            {path: "election/:electionId/confirmation", element: <ConfirmationScreen />, ...routeError},
            {path: "election/:electionId/audit", element: <AuditScreen />, ...routeError},
            {
                path: "election/:electionId/ballot-locator/:ballotId?",
                element: <BallotLocator />,
                ...routeError,
            },
            {path: "materials", element: <SupportMaterialsScreen />, ...routeError},
        ],
    },
]

const clickSelector = (selector: string): boolean => {
    const node = document.querySelector(selector)
    if (!node) {
        return false
    }
    const clickable =
        (node instanceof Element
            ? node.closest("button, [role='button'], a, .MuiButtonBase-root")
            : null) || (node instanceof HTMLElement ? node : node.parentElement)
    if (!(clickable instanceof HTMLElement) || typeof clickable.click !== "function") {
        return false
    }
    clickable.click()
    return true
}

const retryClick = (selector: string, extra?: () => void): void => {
    let attempts = 0
    const tick = () => {
        attempts += 1
        if (clickSelector(selector) || attempts >= 12) {
            extra?.()
            return
        }
        window.setTimeout(tick, 120)
    }
    window.setTimeout(tick, 200)
}

const pathFor = (
    sceneId: string,
    variantId: string,
    uploadedEvent: UploadedElectionEvent | null
): string => {
    const electionId =
        uploadedEvent?.ballotStyles[0]?.election_id ?? electionIdForScene(sceneId, variantId)
    const tenantId = uploadedEvent?.tenantId ?? STUDIO_TENANT_ID
    const eventId = uploadedEvent?.electionEventId ?? STUDIO_EVENT_ID
    const base = `/tenant/${tenantId}/event/${eventId}`
    switch (sceneId) {
        case "start":
            return `${base}/election/${electionId}/start`
        case "write-in":
        case "overvote":
        case "undervote":
        case "blank":
        case "invalid":
        case "voting":
            return `${base}/election/${electionId}/vote`
        case "review":
            return `${base}/election/${electionId}/review`
        case "confirmation":
            return `${base}/election/${electionId}/confirmation`
        case "audit":
            return `${base}/election/${electionId}/audit`
        case "ballot-locator":
            return variantId === "lookup"
                ? `${base}/election/${electionId}/ballot-locator`
                : `${base}/election/${electionId}/ballot-locator/${STUDIO_BALLOT_ID}`
        case "materials":
            return `${base}/materials`
        case "error":
            return variantId === "cert" ? "/cert-auth-error" : "/error"
        default:
            return `${base}/election-chooser`
    }
}

const VariantActions: React.FC<{sceneId: string; variantId: string}> = ({sceneId, variantId}) => {
    useEffect(() => {
        const run = () => {
            const key = `${sceneId}:${variantId}`
            switch (key) {
                case "election-list:help":
                    retryClick(".election-selection-screen h1 button")
                    break
                case "voting:help":
                    retryClick(".voting-screen .title-question")
                    break
                case "write-in:default":
                    retryClick(".voting-screen .next-button")
                    break
                case "overvote:default":
                case "undervote:default":
                case "blank:default":
                    retryClick(".voting-screen .next-button")
                    break
                case "review:help":
                    retryClick(".review-screen h4 button")
                    break
                case "review:confirm":
                    retryClick(".cast-ballot-button")
                    break
                case "review:audit-help":
                    retryClick(".audit-button")
                    break
                case "review:error":
                    retryClick(".cast-ballot-button")
                    break
                case "confirmation:help":
                    retryClick(".confirmation-screen h4 button")
                    break
                case "confirmation:demo":
                    retryClick(".confirmation-screen button")
                    break
                case "audit:help":
                    retryClick(".audit-screen h4 button")
                    break
                case "session:logout":
                    retryClick(".header-class .logout-button", () => {
                        window.setTimeout(() => clickSelector(".MuiMenuItem-root.logout-button"), 80)
                    })
                    break
                default:
                    break
            }
        }
        run()
    }, [sceneId, variantId])

    return null
}

const createAuth = (sceneId: string, variantId: string): AuthContextValues => {
    const timeout = sceneId === "session" && variantId === "timeout"
    return {
        isAuthContextInitialized: true,
        isAuthenticated: true,
        userId: "voter-1",
        username: "alex.voter",
        email: "alex@example.com",
        firstName: "Alex",
        keycloakAccessToken: "loc-studio",
        logout: () => undefined,
        getExpiry: () => new Date(Date.now() + (timeout ? 25_000 : 15 * 60 * 1000)),
        setTenantEvent: () => undefined,
        hasRole: () => false,
        isKiosk: () => false,
        openProfileLink: () => Promise.resolve(),
        isGoldUser: () => false,
        reauthWithGold: async () => undefined,
    }
}

export const LivePreview: React.FC = () => {
    const {previewRevision, uploadedEvent, language} = useLocStudio()
    const {scene, variant} = useCurrentScene()
    const path = pathFor(scene.id, variant.id, uploadedEvent)
    const disableAuth = disableAuthFor(scene.id, variant.id)

    const store = useMemo(
        () =>
            uploadedEvent
                ? createStudioStoreFromUpload(uploadedEvent, scene.id, variant.id, language)
                : createStudioStore(scene.id, variant.id),
        [previewRevision, scene.id, variant.id, uploadedEvent, language]
    )
    const apollo = useMemo(
        () => createStudioApollo(scene.id, variant.id, uploadedEvent),
        [previewRevision, scene.id, variant.id, uploadedEvent]
    )
    const auth = useMemo(() => createAuth(scene.id, variant.id), [scene.id, variant.id])
    const settings = useMemo(
        () => ({
            loaded: true,
            globalSettings: {
                ...defaultSettings,
                DISABLE_AUTH: disableAuth,
                DEFAULT_TENANT_ID: uploadedEvent?.tenantId ?? STUDIO_TENANT_ID,
                DEFAULT_EVENT_ID: uploadedEvent?.electionEventId ?? STUDIO_EVENT_ID,
            },
            defaultLanguageTouched: true,
            setDefaultLanguageTouched: () => undefined,
            setDisableAuth: () => undefined,
        }),
        [disableAuth, uploadedEvent]
    )

    const routerRef = useRef<ReturnType<typeof createMemoryRouter> | null>(null)
    if (!routerRef.current) {
        routerRef.current = createMemoryRouter(studioRoutes, {
            initialEntries: [path],
            initialIndex: 0,
        })
    }
    const router = routerRef.current

    useEffect(() => {
        void router.navigate(path, {replace: true})
    }, [path, router])

    return (
        <Provider store={store}>
            <SettingsContext.Provider value={settings}>
                <AuthContext.Provider value={auth}>
                    <ApolloProvider client={apollo}>
                        <PreviewErrorBoundary resetKey={`${previewRevision}:${scene.id}:${variant.id}`}>
                            <VariantActions sceneId={scene.id} variantId={variant.id} />
                            <RouterProvider router={router} />
                        </PreviewErrorBoundary>
                    </ApolloProvider>
                </AuthContext.Provider>
            </SettingsContext.Provider>
        </Provider>
    )
}
