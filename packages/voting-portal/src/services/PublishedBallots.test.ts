// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ApolloClient, InMemoryCache, ApolloLink} from "@apollo/client"
import {
    capDisplayVotingClose,
    fetchPublicationJson,
    mapPublicationFiles,
    loadPublicationList,
    loadSelectedBallot,
    VoterFile,
    VoterFiles,
} from "./PublishedBallots"

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

test("publication metadata retains live channel policy over frozen status", async () => {
    const client = new ApolloClient({cache: new InMemoryCache(), link: ApolloLink.empty()})
    const status = {
        voting_status: "NOT_STARTED",
        kiosk_voting_status: "OPEN",
        early_voting_status: "OPEN",
        telephone_voting_status: "PAUSED",
    }
    const channels = {online: true, kiosk: true, telephone: false, early_voting: true}
    const refs: VoterFiles = {
        event_id: "event",
        status,
        files: [
            {
                id: "style",
                election_id: "election",
                version: "v1",
                status,
                voting_channels: channels,
                num_allowed_revotes: 3,
                urls: {
                    event_url: "event",
                    election_url: "election",
                    summary_url: "summary",
                    style_url: "style",
                },
            },
        ],
    }
    const objects: Record<string, unknown> = {
        event: {
            id: "event",
            presentation: {materials: {policy: "mandatory_for_voting"}},
            status: {voting_status: "CLOSED"},
        },
        election: {
            id: "election",
            election_event_id: "event",
            status: {voting_status: "CLOSED"},
            num_allowed_revotes: 1,
            voting_channels: {},
            presentation: {grace_period_secs: 120},
        },
        summary: {
            id: "style",
            area_presentation: {allow_early_voting: "allow_early_voting"},
            election_dates: {end_date: "2026-10-01T12:00:00Z"},
        },
    }
    global.fetch = jest.fn(async (url) => new Response(JSON.stringify(objects[String(url)])))
    const result = await loadPublicationList(client, refs)
    expect(result.sequent_backend_election[0]).toMatchObject({
        status,
        voting_channels: channels,
        num_allowed_revotes: 3,
        presentation: {grace_period_secs: 120},
    })
    expect(result.sequent_backend_election_event[0]).toMatchObject({
        status,
        presentation: {materials: {policy: "mandatory_for_voting"}},
    })
    expect(result.summaries.election).toEqual(objects.summary)
    client.stop()
})

test("selected S3 ballot reconstruction preserves the exact signed EML bytes", async () => {
    const client = new ApolloClient({cache: new InMemoryCache(), link: ApolloLink.empty()})
    const presentation = '{ "title" : "Élection", "nested": {"value":1} }'
    const prefix = '{\n "election_event_presentation": '
    const suffix = ', "contests": []\n}'
    const refs = {
        event_id: "event",
        files: [
            {id: "style", election_id: "election", urls: {style_url: "style", event_url: "event"}},
        ],
    } as VoterFiles
    const objects: Record<string, unknown> = {
        event: {ballot_eml_presentation: presentation},
        style: {
            id: "style",
            election_id: "election",
            election_event_id: "event",
            ballot_signature: "signature",
            ballot_eml_prefix: prefix,
            ballot_eml_suffix: suffix,
        },
    }
    global.fetch = jest.fn(async (url) => new Response(JSON.stringify(objects[String(url)])))
    const style = await loadSelectedBallot(client, refs, "election")
    expect(style.ballot_eml).toBe(prefix + presentation + suffix)
    expect(style.ballot_signature).toBe("signature")
    client.stop()
})
