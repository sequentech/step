// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {useParams, useSearchParams} from "react-router-dom"
import {Box} from "@mui/material"
import {Loader} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {useSettings} from "@/providers/SettingsContextProvider"
import {useCustomCss} from "@/providers/CustomCssContextProvider"
import {useResultsManifest} from "@/providers/ResultsManifestContextProvider"
import {useAuthenticatedResults} from "@/hooks/useAuthenticatedResults"
import {discoverPublication, PublicationDiscoveryResult} from "@/services/publicationDiscovery"
import {resolveSqliteArtifactUrl} from "@/services/artifacts"
import {loadSqliteDatabase, readResultsDataset} from "@/services/sqliteResults"
import {ResultsManifest, ResultsSqliteDataset} from "@/types/results"
import {publicBucketUrl} from "@/services/urls"
import {manifestCustomCss} from "@/services/customCss"
import {StateMessage} from "@/components/StateMessage"
import {ResultsPageContent} from "@/components/ResultsPageContent"

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
}

const initialState: RouteState = {
    loading: true,
    requiresAuth: false,
}

const loadManifest = async (
    settingsPublicBucketUrl: string,
    discovery: PublicationDiscoveryResult
): Promise<ResultsManifest> => {
    if (discovery.manifest) {
        return discovery.manifest
    }

    if (!discovery.manifestUrl) {
        const manifestPath =
            discovery.indexEntry?.manifest_public_path ??
            discovery.resolverEntry?.manifest_public_path
        const manifestUrl = publicBucketUrl(settingsPublicBucketUrl, manifestPath)

        if (!manifestUrl) {
            throw new Error("Publication manifest is not available.")
        }

        const response = await fetch(manifestUrl, {cache: "no-store"})
        if (!response.ok) {
            throw new Error(`Unable to load publication manifest: HTTP ${response.status}`)
        }
        return (await response.json()) as ResultsManifest
    }

    const response = await fetch(discovery.manifestUrl, {cache: "no-store"})
    if (!response.ok) {
        throw new Error(`Unable to load publication manifest: HTTP ${response.status}`)
    }

    return (await response.json()) as ResultsManifest
}

export const ResultsRoute: React.FC = () => {
    const {eeId, electionId} = useParams() as unknown as ResultsRouteParams
    const [searchParams] = useSearchParams()
    const {globalSettings} = useSettings()
    const {setCustomCss} = useCustomCss()
    const {setManifestLanguageConfig, resetManifestLanguageConfig} = useResultsManifest()
    const {t} = useTranslation()
    const [state, setState] = useState<RouteState>(initialState)
    const manifestPath = searchParams.get("manifestPath") ?? undefined

    const authTenantId =
        state.discovery?.resolverEntry?.tenant_id ?? state.discovery?.index?.tenant_id
    const authEventId =
        state.discovery?.resolverEntry?.election_event_id ??
        state.discovery?.index?.election_event_id

    const auth = useAuthenticatedResults(
        globalSettings,
        authTenantId,
        authEventId,
        state.requiresAuth
    )

    const authReady = !state.requiresAuth || !!auth.token || globalSettings.DISABLE_AUTH
    const authToken = globalSettings.DISABLE_AUTH ? undefined : auth.token
    const customCss = useMemo(
        () =>
            manifestCustomCss(
                state.manifest,
                electionId ?? state.manifest?.route_election_id ?? undefined
            ),
        [state.manifest, electionId]
    )

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

        const load = async () => {
            if (!eeId) {
                setState({
                    loading: false,
                    requiresAuth: false,
                    error: "No election event id was provided.",
                })
                return
            }

            if (state.requiresAuth && !authReady) {
                return
            }

            try {
                setState((current) => ({
                    ...current,
                    loading: true,
                    error: undefined,
                    manifest: undefined,
                    dataset: undefined,
                }))
                const discovery = await discoverPublication(
                    globalSettings,
                    eeId,
                    electionId,
                    authToken,
                    {manifestPath}
                )

                if (!discovery) {
                    if (mounted) {
                        setState({
                            loading: false,
                            requiresAuth: false,
                            discovery: null,
                        })
                    }
                    return
                }

                const access =
                    discovery.resolverEntry?.access ?? discovery.indexEntry?.access ?? "public"

                if (access === "authenticated" && !authToken && !globalSettings.DISABLE_AUTH) {
                    if (mounted) {
                        setState({
                            loading: false,
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
                    authToken,
                    electionId
                )
                const db = await loadSqliteDatabase(artifactUrl)
                const dataset = readResultsDataset(db)
                db.close()

                if (mounted) {
                    setState({
                        loading: false,
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
                        requiresAuth: false,
                        error: error instanceof Error ? error.message : "unexpected_error",
                    })
                }
            }
        }

        void load()

        return () => {
            mounted = false
        }
    }, [eeId, electionId, manifestPath, globalSettings, authReady, authToken, state.requiresAuth])

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
                    title={t("resultsPortal.state.unexpectedErrorTitle")}
                    message={t("resultsPortal.state.signInErrorMessage")}
                />
            )
        }

        if (state.error) {
            return (
                <StateMessage
                    title={t("resultsPortal.state.unexpectedErrorTitle")}
                    message={t("resultsPortal.state.loadErrorMessage")}
                />
            )
        }

        if (state.requiresAuth) {
            return (
                <StateMessage
                    title={t("resultsPortal.state.signInRequiredTitle")}
                    message={t("resultsPortal.state.signInRequiredMessage")}
                />
            )
        }

        if (!state.discovery) {
            return (
                <StateMessage
                    title={t("resultsPortal.state.notPublishedTitle")}
                    message={t("resultsPortal.state.notPublishedMessage")}
                />
            )
        }

        if (!state.manifest || !state.dataset) {
            return (
                <StateMessage
                    title={t("resultsPortal.state.notPublishedTitle")}
                    message={t("resultsPortal.state.notPublishedMessage")}
                />
            )
        }

        if (state.manifest.contests.length === 0) {
            return (
                <StateMessage
                    title={t("resultsPortal.state.notPublishedTitle")}
                    message={t("resultsPortal.state.notPublishedMessage")}
                />
            )
        }

        return <ResultsPageContent manifest={state.manifest} dataset={state.dataset} />
    }, [auth.error, auth.loading, state, t])

    return <Box className="seq-results-route">{content}</Box>
}
