// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ApolloClient} from "@apollo/client"
import {GetBallotStylesQuery, GetElectionEventQuery, GetElectionsQuery} from "../gql/graphql"
import {IBallotStyle as BallotDefinition} from "@sequentech/ui-core"

type Election = GetElectionsQuery["sequent_backend_election"][number]
export type BallotRecord = GetBallotStylesQuery["sequent_backend_ballot_style"][number]
export interface BallotSummary {
    id: string
    area_presentation: BallotDefinition["area_presentation"]
    election_dates: BallotDefinition["election_dates"]
}
export interface VoterFile {
    id: string
    election_id: string
    version: string
    urls: {event_url: string; election_url: string; summary_url: string; style_url: string}
    status: Election["status"]
    num_allowed_revotes: Election["num_allowed_revotes"]
    voting_channels: Election["voting_channels"]
}
export interface VoterFiles {
    event_id: string
    status: GetElectionEventQuery["sequent_backend_election_event"][number]["status"]
    files: VoterFile[]
}

export class PublicationDownloadError extends Error {
    constructor(public status: number) {
        super("Unable to download published ballot data")
    }
}

// Weak client ownership partitions bytes and in-flight requests across authenticated sessions.
const downloads = new WeakMap<ApolloClient, Map<string, Promise<unknown>>>()
export function fetchPublicationJson<T>(client: ApolloClient, url: string): Promise<T> {
    let cache = downloads.get(client)
    if (!cache) {
        cache = new Map()
        downloads.set(client, cache)
    }
    const existing = cache.get(url)
    if (existing) return existing as Promise<T>
    const request = (async () => {
        let response: Response
        try {
            response = await fetch(url, {credentials: "omit", referrerPolicy: "no-referrer"})
        } catch {
            throw new PublicationDownloadError(0)
        }
        if (!response.ok) throw new PublicationDownloadError(response.status)
        try {
            return (await response.json()) as T
        } catch {
            throw new PublicationDownloadError(response.status)
        }
    })()
    cache.set(url, request)
    void request.catch(() => cache?.delete(url))
    return request
}

export async function mapPublicationFiles<T>(
    files: VoterFile[],
    load: (file: VoterFile) => Promise<T>
): Promise<T[]> {
    const results: T[] = new Array(files.length)
    let next = 0
    await Promise.all(
        Array.from({length: Math.min(4, files.length)}, async () => {
            while (next < files.length) {
                const index = next++
                results[index] = await load(files[index])
            }
        })
    )
    return results
}

export async function loadPublicationList(
    client: ApolloClient,
    refs: VoterFiles,
    selectedElectionId?: string
) {
    const files = selectedElectionId
        ? refs.files.filter((file) => file.election_id === selectedElectionId)
        : refs.files
    const entries = await mapPublicationFiles(files, async (file) => {
        const event = await fetchPublicationJson<
            GetElectionEventQuery["sequent_backend_election_event"][number]
        >(client, file.urls.event_url)
        const election = await fetchPublicationJson<Election>(client, file.urls.election_url)
        const summary = await fetchPublicationJson<BallotSummary>(client, file.urls.summary_url)
        if (
            event.id !== refs.event_id ||
            election.election_event_id !== refs.event_id ||
            election.id !== file.election_id ||
            summary.id !== file.id
        ) {
            throw new Error("Published ballot scope mismatch")
        }
        return {
            event: {...event, status: refs.status},
            election: {
                ...election,
                status: file.status,
                num_allowed_revotes: file.num_allowed_revotes,
                voting_channels: file.voting_channels,
            },
            summary,
        }
    })
    const selectedEvent =
        entries.find((entry) => entry.election.id === selectedElectionId)?.event ??
        entries[0]?.event
    return {
        sequent_backend_election: entries.map((entry) => entry.election),
        sequent_backend_election_event: selectedEvent
            ? [selectedEvent]
            : [{id: refs.event_id, status: refs.status, presentation: null, description: null}],
        summaries: Object.fromEntries(entries.map((entry) => [entry.election.id, entry.summary])),
    }
}

export async function loadSelectedBallot(
    client: ApolloClient,
    refs: VoterFiles,
    electionId: string
): Promise<BallotRecord> {
    const file = refs.files.find((file) => file.election_id === electionId)
    if (!file) throw new Error("Election is not authorized or published")
    const wire = await fetchPublicationJson<
        BallotRecord & {ballot_eml_prefix?: string; ballot_eml_suffix?: string}
    >(client, file.urls.style_url)
    const event = await fetchPublicationJson<{ballot_eml_presentation?: string}>(
        client,
        file.urls.event_url
    )
    const style: BallotRecord = {...wire}
    if (
        typeof wire.ballot_eml_prefix === "string" &&
        typeof wire.ballot_eml_suffix === "string" &&
        typeof event.ballot_eml_presentation === "string"
    ) {
        style.ballot_eml =
            wire.ballot_eml_prefix + event.ballot_eml_presentation + wire.ballot_eml_suffix
    }
    if (
        style.id !== file.id ||
        style.election_id !== electionId ||
        style.election_event_id !== refs.event_id ||
        typeof style.ballot_eml !== "string"
    ) {
        throw new Error("Published ballot scope mismatch")
    }
    return style
}
