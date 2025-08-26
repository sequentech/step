// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, lazy, useContext, useEffect} from "react"
import ReactDOM from "react-dom/client"
import {Provider} from "react-redux"
import {store} from "./store/store"
import "./index.css"
import App from "./App"
import {initI18n, isI18nInitialized} from "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import AuthContextProvider from "./providers/AuthContextProvider"
import {SettingsContext, SettingsWrapper} from "./providers/SettingsContextProvider"
import {createBrowserRouter, RouterProvider, useLocation, useMatch} from "react-router-dom"
import {ErrorPage} from "./routes/ErrorPage"
import {action as votingAction} from "./routes/VotingScreen"
import {action as castBallotAction} from "./routes/ReviewScreen"
import {Loader} from "@sequentech/ui-essentials"
import TenantEvent from "./routes/TenantEvent"
import PreviewPublicationEvent from "./routes/PreviewPublicationEvent"
import ElectionSelectionScreen from "./routes/ElectionSelectionScreen"
import LoginScreen from "./routes/LoginScreen"
import RegisterScreen from "./routes/RegisterScreen"
import StartScreen from "./routes/StartScreen"
import VotingScreen from "./routes/VotingScreen"
import ReviewScreen from "./routes/ReviewScreen"
import ConfirmationScreen from "./routes/ConfirmationScreen"
import AuditScreen from "./routes/AuditScreen"
import BallotLocator from "./routes/BallotLocator"
import SupportMaterialsScreen from "./routes/SupportMaterialsScreen"
import {WasmWrapper} from "./providers/WasmWrapper"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

export type TenantEventType = {
    tenantId: string
    eventId: string
}

export type PreviewPublicationEventType = {
    tenantId: string
    documentId: string
    areaId: string
    publicationId: string
}

export interface KeycloakProviderProps extends React.PropsWithChildren {
    disable: boolean
}

const KeycloakProvider: React.FC<KeycloakProviderProps> = ({disable, children}) => {
    return disable ? (
        <>{children}</>
    ) : (
        <AuthContextProvider>
            <>{children}</>
        </AuthContextProvider>
    )
}

export const KeycloakProviderContainer: React.FC<React.PropsWithChildren> = ({children}) => {
    const {globalSettings, setDisableAuth} = useContext(SettingsContext)

    return <KeycloakProvider disable={globalSettings.DISABLE_AUTH}>{children}</KeycloakProvider>
}


// Resolve initial language with precedence:
// 1) ?lang query param
// 2) Preview JSON election default (if on /preview)
// 3) i18next detector (handled by init without explicit lang)
const getLangParam = (): string | undefined => {
    try {
        const params = new URLSearchParams(window.location.search)
        const lang = params.get("lang") || undefined
        return lang || undefined
    } catch {
        return undefined
    }
}

const isPreviewPath = (): boolean => {
    return /^\/?preview\//.test(window.location.pathname)
}

type PreviewParams = {
    tenantId: string
    documentId: string
    areaId: string
    publicationId: string
}

const parsePreviewParams = (): PreviewParams | null => {
    const parts = window.location.pathname.split("/").filter(Boolean)
    // expected: ["preview", tenantId, documentId, areaId, publicationId]
    if (parts.length >= 5 && parts[0] === "preview") {
        const [_, tenantId, documentId, areaId, publicationId] = parts
        return {tenantId, documentId, areaId, publicationId}
    }
    return null
}

const I18nGate: React.FC<React.PropsWithChildren> = ({children}) => {
    const {globalSettings} = useContext(SettingsContext)
    const [ready, setReady] = React.useState<boolean>(isI18nInitialized())

    React.useEffect(() => {
        let cancelled = False as any
        cancelled = false
        const bootstrap = async () => {
            if (isI18nInitialized()) {
                if (!cancelled) setReady(true)
                return
            }

            const urlLang = getLangParam()
            if (urlLang) {
                await initI18n(urlLang)
                if (!cancelled) setReady(true)
                return
            }

            if (isPreviewPath()) {
                try {
                    const params = parsePreviewParams()
                    if (params) {
                        const {tenantId, documentId, publicationId, areaId} = params
                        const previewUrl = `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${documentId}/${publicationId}.json`
                        const resp = await fetch(previewUrl)
                        if (resp.ok) {
                            const data = await resp.json()
                            const styles = Array.isArray(data?.ballot_styles) ? data.ballot_styles : []
                            const match = styles.find((b: any) => b?.area_id === areaId) || styles[0]
                            const defaultLang = match?.election_presentation?.language_conf?.default_language_code
                            if (defaultLang) {
                                await initI18n(defaultLang)
                                if (!cancelled) setReady(true)
                                return
                            }
                        }
                    }
                } catch (e) {
                    console.warn("I18nGate: failed to obtain preview default language", e)
                }
            }

            await initI18n(undefined)
            if (!cancelled) setReady(true)
        }
        bootstrap()
        return () => {
            cancelled = true
        }
    }, [globalSettings.PUBLIC_BUCKET_URL])

    return ready ? <>{children}</> : <Loader />
}

const router = createBrowserRouter(
    [
        {
            path: "/",
            element: <App />,
            errorElement: <ErrorPage />,
            children: [
                {
                    path: "/preview/:tenantId/:documentId/:areaId/:publicationId",
                    element: <PreviewPublicationEvent />,
                },
                {
                    path: "/tenant/:tenantId/event/:eventId",
                    element: (
                        <Suspense fallback={<Loader />}>
                            <TenantEvent />
                        </Suspense>
                    ),
                    children: [
                        {
                            path: "election-chooser",
                            element: (
                                <Suspense fallback={<Loader />}>
                                    <ElectionSelectionScreen />
                                </Suspense>
                            ),
                        },
                        {
                            path: "login",
                            element: (
                                <Suspense fallback={<Loader />}>
                                    <LoginScreen />
                                </Suspense>
                            ),
                        },
                        {
                            path: "enroll",
                            element: (
                                <Suspense fallback={<Loader />}>
                                    <RegisterScreen />
                                </Suspense>
                            ),
                        },
                        {
                            path: "election/:electionId",
                            children: [
                                {
                                    path: "start",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <StartScreen />
                                        </Suspense>
                                    ),
                                },
                                {
                                    path: "vote",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <VotingScreen />
                                        </Suspense>
                                    ),
                                    action: votingAction,
                                },
                                {
                                    path: "review",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <ReviewScreen />
                                        </Suspense>
                                    ),
                                    action: castBallotAction,
                                },
                                {
                                    path: "confirmation",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <ConfirmationScreen />
                                        </Suspense>
                                    ),
                                },
                                {
                                    path: "audit",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <AuditScreen />
                                        </Suspense>
                                    ),
                                },
                                {
                                    path: "ballot-locator/:ballotId?",
                                    element: (
                                        <Suspense fallback={<Loader />}>
                                            <BallotLocator />
                                        </Suspense>
                                    ),
                                },
                            ],
                        },
                        {
                            path: "materials",
                            element: (
                                <Suspense fallback={<Loader />}>
                                    <SupportMaterialsScreen />
                                </Suspense>
                            ),
                        },
                    ],
                },
            ],
        },
    ],
    {
        basename: "/",
    }
)

root.render(
    <React.StrictMode>
        <WasmWrapper>
            <SettingsWrapper>
                <I18nGate>
                <KeycloakProviderContainer>
                    <Provider store={store}>
                        <ThemeProvider theme={theme}>
                            <RouterProvider router={router} />
                        </ThemeProvider>
                    </Provider>
                </KeycloakProviderContainer>
                </I18nGate>
            </SettingsWrapper>
        </WasmWrapper>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
