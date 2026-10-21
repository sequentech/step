// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createElectionEventConfigLoader} from "./ElectionEventConfig"

const response = (id: string) => ({ok: true, json: async () => ({id})}) as Response
const originalFetch = globalThis.fetch
let fetchMock: jest.Mock

beforeEach(() => {
    fetchMock = jest.fn()
    globalThis.fetch = fetchMock as typeof fetch
})
afterEach(() => {
    globalThis.fetch = originalFetch
})

test("startup effects share pending and completed requests for the same URL", async () => {
    let resolve!: (value: Response) => void
    fetchMock.mockReturnValue(
        new Promise<Response>((done) => {
            resolve = done
        })
    )
    const load = createElectionEventConfigLoader()
    const first = load("/event-a/config.json")
    expect(load("/event-a/config.json")).toBe(first)
    resolve(response("a"))
    await expect(first).resolves.toEqual({id: "a"})
    await expect(load("/event-a/config.json")).resolves.toEqual({id: "a"})
    expect(fetchMock).toHaveBeenCalledTimes(1)
})

test("changing event URLs fetches their own config", async () => {
    fetchMock.mockResolvedValueOnce(response("a")).mockResolvedValueOnce(response("b"))
    const load = createElectionEventConfigLoader()
    await expect(load("/event-a/config.json")).resolves.toEqual({id: "a"})
    await expect(load("/event-b/config.json")).resolves.toEqual({id: "b"})
    expect(fetchMock).toHaveBeenCalledTimes(2)
})

test.each(["network", "http", "json"])("a %s failure can be retried", async (failure) => {
    if (failure === "network") fetchMock.mockRejectedValueOnce(new Error("offline"))
    if (failure === "http") fetchMock.mockResolvedValueOnce({ok: false, status: 503})
    if (failure === "json")
        fetchMock.mockResolvedValueOnce({
            ok: true,
            json: async () => {
                throw new Error("invalid JSON")
            },
        })
    fetchMock.mockResolvedValueOnce(response("a"))
    const load = createElectionEventConfigLoader()
    await expect(load("/event-a/config.json")).rejects.toThrow()
    await expect(load("/event-a/config.json")).resolves.toEqual({id: "a"})
    expect(fetchMock).toHaveBeenCalledTimes(2)
})

test("an old failure does not discard the newer event request", async () => {
    let reject!: (reason: Error) => void
    fetchMock.mockReturnValueOnce(
        new Promise<Response>((_, fail) => {
            reject = fail
        })
    )
    fetchMock.mockResolvedValueOnce(response("b"))
    const load = createElectionEventConfigLoader()
    const old = load("/event-a/config.json")
    const newer = load("/event-b/config.json")
    reject(new Error("offline"))
    await expect(old).rejects.toThrow("offline")
    await expect(newer).resolves.toEqual({id: "b"})
    expect(load("/event-b/config.json")).toBe(newer)
    expect(fetchMock).toHaveBeenCalledTimes(2)
})

test("a new app mount reads fresh configuration", async () => {
    fetchMock.mockResolvedValueOnce(response("old")).mockResolvedValueOnce(response("new"))
    await expect(createElectionEventConfigLoader()("/event-a/config.json")).resolves.toEqual({
        id: "old",
    })
    await expect(createElectionEventConfigLoader()("/event-a/config.json")).resolves.toEqual({
        id: "new",
    })
    expect(fetchMock).toHaveBeenCalledTimes(2)
})
