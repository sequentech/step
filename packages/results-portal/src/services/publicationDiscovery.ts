// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ResultsManifest,
    ResultsPublicationIndex,
    ResultsPublicationIndexEntry,
    ResultsResolverResponse,
} from "@/types/results"
import {GlobalSettings} from "@/providers/SettingsContextProvider"
import {graphqlFetch} from "./graphql"
import {
    RESOLVE_RESULTS_PUBLICATION,
    ResolveResultsPublicationVariables,
} from "@/queries/resultsPublication"
import {publicBucketUrl} from "./urls"

interface ResolveData {
    resolveResultsPublication: ResultsResolverResponse | null
}

export interface PublicationDiscoveryResult {
    source: "public-index" | "resolver"
    index?: ResultsPublicationIndex
    indexEntry?: ResultsPublicationIndexEntry
    resolverEntry?: ResultsResolverResponse
    manifest?: ResultsManifest
    manifestUrl?: string
}

export interface PublicationDiscoveryOptions {
    manifestPath?: string
}

const isAbsoluteUrl = (value: string) => /^https?:\/\//i.test(value)

const resolveManifestOverride = (settings: GlobalSettings, manifestPath?: string) => {
    if (!manifestPath) {
        return undefined
    }

    return isAbsoluteUrl(manifestPath)
        ? manifestPath
        : publicBucketUrl(settings.PUBLIC_BUCKET_URL, manifestPath)
}

const isRouteMatch = (
    publication: ResultsPublicationIndexEntry,
    eeId: string,
    electionId?: string
): boolean => {
    if (electionId) {
        if (publication.route_scope === "election") {
            return (
                publication.route_election_id === electionId ||
                publication.route === `/${eeId}/elections/${electionId}`
            )
        }

        return (
            publication.route_scope === "event" &&
            (publication.election_ids?.includes(electionId) || publication.route === `/${eeId}`)
        )
    }

    return publication.route_scope === "event" || publication.route === `/${eeId}`
}

const normalizeIndex = (value: unknown): ResultsPublicationIndex => {
    const index = value as ResultsPublicationIndex | ResultsPublicationIndexEntry

    if ("publication_id" in index) {
        return {
            schema_version: 1,
            publications: [index as ResultsPublicationIndexEntry],
        }
    }

    return index as ResultsPublicationIndex
}

export const fetchPublicIndex = async (
    settings: GlobalSettings,
    eeId: string
): Promise<ResultsPublicationIndex | null> => {
    const url = publicBucketUrl(settings.PUBLIC_BUCKET_URL, `results-index/${eeId}.json`)
    if (!url) {
        return null
    }

    const response = await fetch(url, {cache: "no-store"})
    if (response.status === 404) {
        return null
    }
    if (!response.ok) {
        throw new Error(`Unable to load results index: HTTP ${response.status}`)
    }

    return normalizeIndex(await response.json())
}

export const findIndexPublication = (
    index: ResultsPublicationIndex | null,
    eeId: string,
    electionId?: string
): ResultsPublicationIndexEntry | null => {
    return (
        index?.publications?.find((publication) => isRouteMatch(publication, eeId, electionId)) ??
        null
    )
}

export const resolveManifestUrl = (
    settings: GlobalSettings,
    publication: Pick<ResultsPublicationIndexEntry, "manifest_public_path" | "manifest_url">
) =>
    publication.manifest_url ??
    publicBucketUrl(settings.PUBLIC_BUCKET_URL, publication.manifest_public_path)

export const discoverPublication = async (
    settings: GlobalSettings,
    eeId: string,
    electionId?: string,
    token?: string,
    options: PublicationDiscoveryOptions = {}
): Promise<PublicationDiscoveryResult | null> => {
    const index = await fetchPublicIndex(settings, eeId)
    const indexEntry = findIndexPublication(index, eeId, electionId)

    if (indexEntry?.access === "public") {
        const activeManifestUrl = resolveManifestUrl(settings, indexEntry)
        const requestedManifestUrl = resolveManifestOverride(settings, options.manifestPath)

        return {
            source: "public-index",
            index: index ?? undefined,
            indexEntry,
            manifestUrl:
                requestedManifestUrl === activeManifestUrl
                    ? requestedManifestUrl
                    : activeManifestUrl,
        }
    }

    if (indexEntry?.access === "authenticated" && !token) {
        return {
            source: "public-index",
            index: index ?? undefined,
            indexEntry,
        }
    }

    if (token) {
        const data = await graphqlFetch<ResolveData, ResolveResultsPublicationVariables>(
            settings.HASURA_URL,
            RESOLVE_RESULTS_PUBLICATION,
            {eeId, electionId},
            token
        )
        const resolverEntry = data.resolveResultsPublication

        if (!resolverEntry) {
            return null
        }

        return {
            source: "resolver",
            index: index ?? undefined,
            resolverEntry,
            manifest: resolverEntry.manifest,
            manifestUrl:
                resolverEntry.manifest_url ??
                publicBucketUrl(settings.PUBLIC_BUCKET_URL, resolverEntry.manifest_public_path),
        }
    }

    return indexEntry
        ? {
              source: "public-index",
              index: index ?? undefined,
              indexEntry,
          }
        : null
}
