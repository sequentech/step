// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GlobalSettings} from "@/providers/SettingsContextProvider"
import {FETCH_RESULTS_ARTIFACT, FetchResultsArtifactVariables} from "@/queries/resultsPublication"
import {ResultsManifest} from "@/types/results"
import {graphqlFetch} from "./graphql"
import {publicBucketUrl} from "./urls"

interface FetchArtifactData {
    fetchResultsArtifact: {
        urls: string[]
    } | null
}

export const resolveSqliteArtifactUrl = async (
    settings: GlobalSettings,
    manifest: ResultsManifest,
    token?: string,
    electionId?: string
): Promise<string> => {
    const fullSqlite = manifest.artifacts.full_sqlite
    const directUrl =
        fullSqlite?.url ?? publicBucketUrl(settings.PUBLIC_BUCKET_URL, fullSqlite?.public_path)

    if (directUrl) {
        return directUrl
    }

    if (!token) {
        throw new Error("This publication requires sign-in before results can be loaded.")
    }

    const data = await graphqlFetch<FetchArtifactData, FetchResultsArtifactVariables>(
        settings.HASURA_URL,
        FETCH_RESULTS_ARTIFACT,
        {
            electionEventId: manifest.election_event_id,
            electionId,
            publicationId: manifest.publication_id,
        },
        token
    )

    const output = data.fetchResultsArtifact
    const signedUrl = output?.urls[0]

    if (!signedUrl) {
        throw new Error("No accessible results artifact was returned for this account.")
    }

    return signedUrl
}
