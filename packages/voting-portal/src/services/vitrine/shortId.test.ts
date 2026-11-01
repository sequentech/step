// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {BucketLookupError, lookupTrackingId} from "@vitrine/verify"
import type {BallotMatch, BucketMatches, Policy} from "@vitrine/verify"
import {
    bucketServers,
    EShortIdKind,
    lookupShortId,
    MAX_LISTED,
    shortIdResultOf,
    shortIdToLook,
} from "./shortId"

jest.mock("@vitrine/verify", () => ({
    ...jest.requireActual("@vitrine/verify"),
    lookupTrackingId: jest.fn(),
}))

const lookup = lookupTrackingId as jest.MockedFunction<typeof lookupTrackingId>

const ID = "FTBEMHRX71J75ZBD" + "0".repeat(36)
const OTHER = "FTBEMHRX0WDY4YJG" + "0".repeat(36)

const policy = (opts: {url?: string; monitors?: string[]} = {}): Policy =>
    ({
        logs: new Map([
            [
                "ballots",
                {
                    origin: "logs.vote.example.org/acme/muni/ballots",
                    url: opts.url ?? "https://logs.vote.example.org/acme/muni/ballots/",
                },
            ],
        ]),
        monitors: new Map((opts.monitors ?? []).map((url, i) => [`m${i}`, {url}])),
    }) as unknown as Policy

const ballot = (
    trackingId: string,
    shared = 2,
    stage = "cast",
    acceptedAt = 1_700_000_000
): BallotMatch => ({
    ballotId: "ab".repeat(32),
    trackingId,
    shared,
    exact: false,
    event: "event-1",
    election: "election-1",
    entries: [{index: 3, stage, acceptedAt}] as BallotMatch["entries"],
})

const answer = (matches: BallotMatch[], more = 0, truncated = false): BucketMatches => ({
    matches,
    more,
    truncated,
    size: 10,
})

const full = (chars: string) => chars.match(/.{4}/g)!.join("-")

describe("bucketServers", () => {
    it("names the log's own server first, then the monitors", () => {
        expect(bucketServers(policy({monitors: ["https://mon.example.org"]}))).toEqual([
            {server: "https://logs.vote.example.org", log: "acme/muni/ballots"},
            {server: "https://mon.example.org", log: "logs.vote.example.org/acme/muni/ballots"},
        ])
    })

    it("leaves out a log whose URL does not end with its name", () => {
        expect(bucketServers(policy({url: "https://elsewhere.example.org/x/"}))).toEqual([])
    })

    it("is empty without a ballots log", () => {
        expect(bucketServers({logs: new Map(), monitors: new Map()} as unknown as Policy)).toEqual(
            []
        )
    })
})

describe("shortIdToLook", () => {
    it("takes a short ID and the start of one, and no other input", () => {
        expect(shortIdToLook("ftbe-mhrx")).toBe("FTBEMHRX")
        expect(shortIdToLook("FTBE-MHRX-71J7")).toBe("FTBEMHRX71J7")
        expect(shortIdToLook("FTBE")).toBeUndefined()
        expect(shortIdToLook(full(ID))).toBeUndefined()
        expect(shortIdToLook("FTBE-MHRU")).toBeUndefined()
    })
})

describe("shortIdResultOf", () => {
    it("lists a few ballots, the third group ready to compare", () => {
        const result = shortIdResultOf(
            "FTBEMHRX",
            answer([ballot(ID), ballot(OTHER, 2, "audited")])
        )
        expect(result.kind).toBe(EShortIdKind.LISTED)
        expect(result.total).toBe(2)
        expect(result.matches.map((m) => [m.third, m.fourth, m.audited])).toEqual([
            ["71J7", "5ZBD", false],
            ["0WDY", "4YJG", true],
        ])
        expect(result.matches[0].at).toBe(1_700_000_000)
        expect(result.matches.some((m) => m.onDevice)).toBe(false)
    })

    it("lists nothing above five, and asks for the third group", () => {
        const matches = Array.from({length: MAX_LISTED}, () => ballot(ID))
        const result = shortIdResultOf("FTBEMHRX", answer(matches, 23))
        expect(result.kind).toBe(EShortIdKind.MANY)
        expect(result.matches).toEqual([])
        expect(result.total).toBe(28)
    })

    it("is none for an empty bucket", () => {
        expect(shortIdResultOf("FTBEMHRX", answer([])).kind).toBe(EShortIdKind.NONE)
    })

    it("puts the ballot of this device first and keeps the look-alikes listed", () => {
        const result = shortIdResultOf("FTBEMHRX", answer([ballot(OTHER), ballot(ID)]), [full(ID)])
        expect(result.kind).toBe(EShortIdKind.LISTED)
        expect(result.mine).toBe(ID)
        expect(result.matches.map((m) => m.onDevice)).toEqual([true, false])
        expect(result.matches[1].trackingId).toBe(OTHER)
    })

    it("lists the ballot of this device even when many share the short ID", () => {
        const matches = Array.from({length: MAX_LISTED}, () => ballot(OTHER))
        const result = shortIdResultOf("FTBEMHRX", answer(matches, 40), [full(ID)])
        expect(result.kind).toBe(EShortIdKind.LISTED)
        expect(result.matches).toHaveLength(MAX_LISTED)
        expect(result.matches[0]).toMatchObject({trackingId: ID, onDevice: true})
    })

    it("does not mark a receipt of another ballot", () => {
        const result = shortIdResultOf("FTBEMHRX", answer([ballot(ID)]), [
            full("ZZZZ" + ID.slice(4)),
        ])
        expect(result.mine).toBeUndefined()
        expect(result.matches[0].onDevice).toBe(false)
    })

    it("narrows to the ballots that share the third group", () => {
        const result = shortIdResultOf("FTBEMHRX71J7", answer([ballot(ID, 3), ballot(OTHER, 2)]))
        expect(result.matches.map((m) => m.trackingId)).toEqual([ID])
        expect(result.total).toBe(1)
    })
})

describe("lookupShortId", () => {
    beforeEach(() => lookup.mockReset())

    it("asks the log's own server first and stops at a whole answer", async () => {
        lookup.mockResolvedValueOnce(answer([ballot(ID)]))
        const result = await lookupShortId(
            policy({monitors: ["https://mon.example.org"]}),
            "FTBE-MHRX"
        )
        expect(result.kind).toBe(EShortIdKind.LISTED)
        expect(lookup).toHaveBeenCalledTimes(1)
        expect(lookup.mock.calls[0].slice(0, 3)).toEqual([
            "https://logs.vote.example.org",
            "acme/muni/ballots",
            "FTBE-MHRX",
        ])
    })

    it("tries the monitor when the log server has no bucket route", async () => {
        lookup
            .mockRejectedValueOnce(new BucketLookupError("no", 404))
            .mockResolvedValueOnce(answer([ballot(ID)]))
        const result = await lookupShortId(
            policy({monitors: ["https://mon.example.org"]}),
            "FTBE-MHRX"
        )
        expect(result.kind).toBe(EShortIdKind.LISTED)
        expect(lookup.mock.calls[1][0]).toBe("https://mon.example.org")
    })

    it("is unsupported when every server answers 404", async () => {
        lookup.mockRejectedValue(new BucketLookupError("no", 404))
        const result = await lookupShortId(
            policy({monitors: ["https://mon.example.org"]}),
            "FTBE-MHRX"
        )
        expect(result.kind).toBe(EShortIdKind.UNSUPPORTED)
    })

    it("is unsupported when the policy names no server", async () => {
        const result = await lookupShortId(
            {logs: new Map(), monitors: new Map()} as unknown as Policy,
            "FTBE-MHRX"
        )
        expect(result.kind).toBe(EShortIdKind.UNSUPPORTED)
        expect(lookup).not.toHaveBeenCalled()
    })

    it("is unavailable when a server fails and none answers", async () => {
        lookup.mockRejectedValueOnce(new BucketLookupError("busy", 503, 5))
        const result = await lookupShortId(policy(), "FTBE-MHRX")
        expect(result.kind).toBe(EShortIdKind.UNAVAILABLE)
    })

    it("falls back to a truncated answer when no server has the whole bucket", async () => {
        lookup.mockResolvedValue(answer([ballot(ID)], 0, true))
        const result = await lookupShortId(
            policy({monitors: ["https://mon.example.org"]}),
            "FTBE-MHRX"
        )
        expect(result.kind).toBe(EShortIdKind.LISTED)
        expect(result.truncated).toBe(true)
        expect(lookup).toHaveBeenCalledTimes(2)
    })

    it("refuses what is not a short ID before asking", async () => {
        await expect(lookupShortId(policy(), full(ID))).rejects.toThrow(TypeError)
        expect(lookup).not.toHaveBeenCalled()
    })
})
