// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useRef, useState} from "react"
import {useParams} from "react-router-dom"
import {Box} from "@mui/material"
import {Loader} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {useSettings} from "@/providers/SettingsContextProvider"
import {useCustomCss} from "@/providers/CustomCssContextProvider"
import {useResultsManifest} from "@/providers/ResultsManifestContextProvider"
import {useAuthenticatedResults} from "@/hooks/useAuthenticatedResults"
import {useResultsAuth} from "@/providers/ResultsAuthContextProvider"
import {discoverPublication, PublicationDiscoveryResult} from "@/services/publicationDiscovery"
import {resolveSqliteArtifactUrl} from "@/services/artifacts"
import {loadSqliteDatabase, readResultsDataset} from "@/services/sqliteResults"
import {parseResultsManifest, ResultsManifest, ResultsSqliteDataset} from "@/types/results"
import {publicBucketUrl} from "@/services/urls"
import {manifestCustomCss} from "@/services/customCss"
import {StateMessage} from "@/components/StateMessage"
import {ResultsPageContent} from "@/components/ResultsPageContent"
import {entityClassName} from "@/services/cssClassNames"
import {getElectionEventPresentation} from "@/services/resultsOrdering"
import {ETranslationScope, overwriteTranslations} from "@sequentech/ui-core"

interface ResultsRouteParams {
    eeId: string
    electionId?: string
}

interface RouteState {
    loading: boolean
    discovery?: PublicationDiscoveryResult | null
    manifest?: ResultsManifest
    dataset?: ResultsSqliteDataset
    error?: string
    requiresAuth: boolean
    authenticatedAccess: boolean
}

const initialState: RouteState = {
    loading: true,
    requiresAuth: false,
    authenticatedAccess: false,
}

const loadManifest = async (
    settingsPublicBucketUrl: string,
    discovery: PublicationDiscoveryResult
): Promise<ResultsManifest> => {
    if (discovery.manifest) {
        return parseResultsManifest(discovery.manifest)
    }

    if (!discovery.manifestUrl) {
        const manifestPath =
            discovery.indexEntry?.manifest_public_path ??
            discovery.resolverEntry?.manifest_public_path
        const manifestUrl = publicBucketUrl(settingsPublicBucketUrl, manifestPath ?? undefined)

        if (!manifestUrl) {
            throw new Error("Publication manifest is not available.")
        }

        const response = await fetch(manifestUrl, {cache: "no-store"})
        if (!response.ok) {
            throw new Error(`Unable to load publication manifest: HTTP ${response.status}`)
        }
        return parseResultsManifest(await response.json())
    }

    const response = await fetch(discovery.manifestUrl, {cache: "no-store"})
    if (!response.ok) {
        throw new Error(`Unable to load publication manifest: HTTP ${response.status}`)
    }

    return parseResultsManifest(await response.json())
}

export const ResultsRoute: React.FC = () => {
    const {eeId, electionId} = useParams<keyof ResultsRouteParams>()
    const {globalSettings} = useSettings()
    const {setCustomCss} = useCustomCss()
    const {setManifestLanguageConfig, resetManifestLanguageConfig} = useResultsManifest()
    const {setAuthenticatedSession, clearAuthenticatedSession} = useResultsAuth()
    const {t} = useTranslation()
    const [state, setState] = useState<RouteState>(initialState)
    const [selectedElectionId, setSelectedElectionId] = useState<string>()

    const authTenantId =
        state.discovery?.resolverEntry?.tenant_id ?? state.discovery?.index?.tenant_id
    const authEventId =
        state.discovery?.resolverEntry?.election_event_id ??
        state.discovery?.index?.election_event_id

    const auth = useAuthenticatedResults(
        globalSettings,
        authTenantId,
        authEventId,
        state.requiresAuth || state.authenticatedAccess
    )

    const authToken = auth.token
    const authTokenRef = useRef(authToken)
    authTokenRef.current = authToken
    const hasAuthToken = Boolean(authToken)
    const customCss = useMemo(
        () =>
            manifestCustomCss(
                state.manifest,
                electionId ?? selectedElectionId ?? state.manifest?.route_election_id ?? undefined
            ),
        [state.manifest, electionId, selectedElectionId]
    )

    useEffect(() => {
        clearAuthenticatedSession()
        setSelectedElectionId(undefined)

        return () => clearAuthenticatedSession()
    }, [clearAuthenticatedSession, eeId, electionId])

    useEffect(() => {
        if (auth.token && auth.userProfile && auth.logout) {
            setAuthenticatedSession({
                userProfile: auth.userProfile,
                logout: auth.logout,
            })
            return
        }

        if (!state.requiresAuth && !state.authenticatedAccess) {
            clearAuthenticatedSession()
        }
    }, [
        auth.logout,
        auth.token,
        auth.userProfile,
        clearAuthenticatedSession,
        setAuthenticatedSession,
        state.authenticatedAccess,
        state.requiresAuth,
    ])

    useEffect(() => {
        setCustomCss(customCss)

        return () => setCustomCss("")
    }, [customCss, setCustomCss])

    useEffect(() => {
        if (state.manifest) {
            setManifestLanguageConfig(state.manifest)
        } else {
            resetManifestLanguageConfig()
        }

        return () => resetManifestLanguageConfig()
    }, [resetManifestLanguageConfig, setManifestLanguageConfig, state.manifest])

    useEffect(() => {
        let mounted = true

        // Do not display wording from the previous publication while a new
        // route is loading or after it fails to load.
        overwriteTranslations(undefined, {
            scope: ETranslationScope.RESULTS_PORTAL,
            changeDefaultLanguage: false,
        })

        const load = async () => {
            if (!eeId) {
                setState({
                    loading: false,
                    authenticatedAccess: false,
                    requiresAuth: false,
                    error: "No election event id was provided.",
                })
                return
            }

            try {
                const token = authTokenRef.current
                setState((current) => ({
                    ...current,
                    loading: true,
                    error: undefined,
                    manifest: undefined,
                    dataset: undefined,
                }))
                const discovery = await discoverPublication(globalSettings, eeId, electionId, token)

                if (!discovery) {
                    if (mounted) {
                        setState({
                            loading: false,
                            authenticatedAccess: false,
                            requiresAuth: false,
                            discovery: null,
                        })
                    }
                    return
                }

                const access =
                    discovery.resolverEntry?.access ?? discovery.indexEntry?.access ?? "public"

                if (access === "authenticated" && !token) {
                    if (mounted) {
                        setState({
                            loading: false,
                            authenticatedAccess: true,
                            requiresAuth: true,
                            discovery,
                        })
                    }
                    return
                }

                const manifest = await loadManifest(globalSettings.PUBLIC_BUCKET_URL, discovery)
                const artifactUrl = await resolveSqliteArtifactUrl(
                    globalSettings,
                    manifest,
                    token,
                    electionId
                )
                const db = await loadSqliteDatabase(artifactUrl)
                const dataset = readResultsDataset(db)
                db.close()

                if (mounted) {
                    overwriteTranslations(getElectionEventPresentation(manifest, dataset), {
                        scope: ETranslationScope.RESULTS_PORTAL,
                        changeDefaultLanguage: false,
                    })
                    setState({
                        loading: false,
                        authenticatedAccess: access === "authenticated",
                        requiresAuth: false,
                        discovery,
                        manifest,
                        dataset,
                    })
                }
            } catch (error) {
                if (mounted) {
                    setState({
                        loading: false,
                        authenticatedAccess: false,
                        requiresAuth: false,
                        error: error instanceof Error ? error.message : "unexpected_error",
                    })
                }
            }
        }

        void load()

        return () => {
            mounted = false
            overwriteTranslations(undefined, {
                scope: ETranslationScope.RESULTS_PORTAL,
                changeDefaultLanguage: false,
            })
        }
    }, [eeId, electionId, globalSettings, hasAuthToken])

    const content = useMemo(() => {
        if (state.loading || auth.loading) {
            return (
                <Box
                    className="seq-results-route__loader"
                    sx={{display: "flex", justifyContent: "center", py: 10}}
                >
                    <Loader />
                </Box>
            )
        }

        if (auth.error) {
            return (
                <StateMessage
                    className="seq-results-state-message--sign-in-error"
                    title={t("resultsPortal.state.unexpectedErrorTitle")}
                    message={t("resultsPortal.state.signInErrorMessage")}
                />
            )
        }

        if (state.error) {
            return (
                <StateMessage
                    className="seq-results-state-message--load-error"
                    title={t("resultsPortal.state.unexpectedErrorTitle")}
                    message={t("resultsPortal.state.loadErrorMessage")}
                />
            )
        }

        if (state.requiresAuth) {
            return (
                <StateMessage
                    className="seq-results-state-message--sign-in-required"
                    title={t("resultsPortal.state.signInRequiredTitle")}
                    message={t("resultsPortal.state.signInRequiredMessage")}
                />
            )
        }

        if (!state.discovery) {
            return (
                <StateMessage
                    className="seq-results-state-message--not-published"
                    title={t("resultsPortal.state.notPublishedTitle")}
                    message={t("resultsPortal.state.notPublishedMessage")}
                />
            )
        }

        if (!state.manifest || !state.dataset) {
            return (
                <StateMessage
                    className="seq-results-state-message--missing-artifacts"
                    title={t("resultsPortal.state.notPublishedTitle")}
                    message={t("resultsPortal.state.notPublishedMessage")}
                />
            )
        }

        if (state.manifest.contests.length === 0) {
            return (
                <StateMessage
                    className="seq-results-state-message--empty-publication"
                    title={t("resultsPortal.state.notPublishedTitle")}
                    message={t("resultsPortal.state.notPublishedMessage")}
                />
            )
        }

        return (
            <ResultsPageContent
                manifest={state.manifest}
                dataset={state.dataset}
                initialElectionId={electionId ?? state.manifest.route_election_id ?? undefined}
                onElectionChange={setSelectedElectionId}
            />
        )
    }, [auth.error, auth.loading, state, t])

    return (
        <Box
            className={[
                "seq-results-route",
                entityClassName("route-event", eeId),
                electionId ? entityClassName("route-election", electionId) : null,
            ]
                .filter(Boolean)
                .join(" ")}
        >
            {content}
        </Box>
    )
}
