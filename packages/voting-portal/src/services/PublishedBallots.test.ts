// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ApolloClient, InMemoryCache, ApolloLink} from "@apollo/client"
import {fetchPublicationJson, mapPublicationFiles, VoterFile} from "./PublishedBallots"

test("200 elections have at most four simultaneous metadata workers", async () => {
    let active = 0
    let peak = 0
    const files = Array.from({length: 200}, (_, i) => ({id: String(i)}) as VoterFile)
    const result = await mapPublicationFiles(files, async (file) => {
        active++
        peak = Math.max(peak, active)
        await new Promise((resolve) => setTimeout(resolve, 1))
        active--
        return file.id
    })
    expect(peak).toBe(4)
    expect(result).toEqual(files.map((file) => file.id))
})

test("in-flight downloads are shared only within an authenticated client", async () => {
    const first = new ApolloClient({cache: new InMemoryCache(), link: ApolloLink.empty()})
    const second = new ApolloClient({cache: new InMemoryCache(), link: ApolloLink.empty()})
    global.fetch = jest.fn(async () => new Response("{}"))
    await Promise.all([
        fetchPublicationJson(first, "https://private/object"),
        fetchPublicationJson(first, "https://private/object"),
    ])
    expect(global.fetch).toHaveBeenCalledTimes(1)
    await fetchPublicationJson(second, "https://private/object")
    expect(global.fetch).toHaveBeenCalledTimes(2)
    first.stop()
    second.stop()
})

test("failed downloads are evicted and errors never include signed URLs", async () => {
    const client = new ApolloClient({cache: new InMemoryCache(), link: ApolloLink.empty()})
    global.fetch = jest.fn(async () => new Response("denied", {status: 403}))
    await expect(
        fetchPublicationJson(client, "https://private/object?signature=secret")
    ).rejects.toThrow("Unable to download published ballot data")
    global.fetch = jest.fn(async () => new Response("{}"))
    await expect(
        fetchPublicationJson(client, "https://private/object?signature=secret")
    ).resolves.toEqual({})
    expect(global.fetch).toHaveBeenCalledTimes(1)
    client.stop()
})

test("current signed close caps the chooser dates without changing signed ballot bytes", async () => {
    const {loadPublicationList, loadSelectedBallot} = await import("./PublishedBallots")
    const client = new ApolloClient({cache: new InMemoryCache(), link: ApolloLink.empty()})
    const ballot =
        '{ "id":"style", "election_dates": {"scheduled_event_dates":{"END_VOTING_PERIOD":{"scheduled_at":"2030-01-02T12:00:00Z"}}} }'
    const urls = {
        event_url: "event",
        election_url: "election",
        summary_url: "summary",
        style_url: "style",
    }
    const originalDates = {
        scheduled_event_dates: {
            END_VOTING_PERIOD: {scheduled_at: "2030-01-02T12:00:00Z", timezone: "Europe/Madrid"},
        },
    }
    const objects: Record<string, unknown> = {
        event: {id: "event", presentation: {timezones: {primary: "UTC"}}},
        election: {id: "election", election_event_id: "event"},
        summary: {id: "style", election_dates: originalDates},
        style: {
            id: "style",
            election_id: "election",
            election_event_id: "event",
            ballot_eml: ballot,
        },
    }
    global.fetch = jest.fn(async (url) => new Response(JSON.stringify(objects[String(url)])))
    const refs = {
        event_id: "event",
        status: {},
        files: [
            {
                id: "style",
                election_id: "election",
                version: "version",
                urls,
                status: {},
                num_allowed_revotes: 1,
                voting_channels: {online: true},
                signed_close: {scheduled_at: "2030-01-02T10:00:00Z", timezone: "Asia/Manila"},
            },
        ],
    }
    const list = await loadPublicationList(client, refs)
    expect(list.summaries.election.election_dates?.authoritative_close).toEqual({
        scheduled_at: "2030-01-02T10:00:00Z",
        timezone: "Asia/Manila",
    })
    expect((await loadSelectedBallot(client, refs, "election")).ballot_eml).toBe(ballot)
    expect(originalDates.scheduled_event_dates.END_VOTING_PERIOD.scheduled_at).toBe(
        "2030-01-02T12:00:00Z"
    )
    // Metadata refresh reuses immutable downloads but applies the new cap afresh.
    const refreshed = {
        ...refs,
        files: [{...refs.files[0], signed_close: {scheduled_at: "2030-01-02T09:00:00Z"}}],
    }
    const updated = await loadPublicationList(client, refreshed)
    expect(updated.summaries.election.election_dates?.authoritative_close).toEqual({
        scheduled_at: "2030-01-02T09:00:00Z",
    })
    client.stop()
})

test("current authority replaces an unauthorized earlier advertised date and can remove its deadline", async () => {
    const {capDisplayVotingClose} = await import("./PublishedBallots")
    const early = {
        id: "style",
        area_presentation: undefined,
        election_dates: {
            scheduled_event_dates: {
                END_VOTING_PERIOD: {scheduled_at: "2030-01-02T09:00:00Z", timezone: "UTC"},
            },
        },
    }
    expect(
        capDisplayVotingClose(early, {scheduled_at: "2030-01-02T10:00:00Z"}).election_dates
            ?.authoritative_close
    ).toEqual({scheduled_at: "2030-01-02T10:00:00Z"})
    expect(
        capDisplayVotingClose(early, {scheduled_at: null}).election_dates?.authoritative_close
    ).toEqual({scheduled_at: null})
    expect(capDisplayVotingClose(early, undefined)).toBe(early)
})
