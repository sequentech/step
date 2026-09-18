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
