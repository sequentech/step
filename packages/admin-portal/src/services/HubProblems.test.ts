// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import type {Policy} from "@vitrine/verify"
import {policyText} from "@/fixtures/vitrinePolicy"
import {
    fetchHubProblems,
    hubProblems,
    MAX_PROBLEM_PAGES,
    parseProblemPage,
    problemKindKey,
    problemLink,
    problemsUrl,
    TFetch,
} from "./HubProblems"

const POLICY = `sha256:${"ab".repeat(32)}`
const evidence = (n: number) => `sha256:${n.toString(16).padStart(64, "0")}`
const row = (n: number, extra: Record<string, unknown> = {}) => ({
    id: evidence(n),
    kind: "split-view",
    subject: "logs.acme.example/e/ballots",
    policy: POLICY,
    logged_at: 1_800_000_000 + n,
    entry: n,
    ...extra,
})
const page = (problems: unknown[], more = false) => JSON.stringify({hub: "h", problems, more})
const answer = (body: string, status = 200) =>
    ({status, headers: new Headers(), text: async () => body}) as unknown as Response

beforeAll(async () => {
    await verify.init()
})

describe("parseProblemPage", () => {
    it("reads the rows of a list", () => {
        expect(parseProblemPage(page([row(2), row(1)], true), POLICY)).toEqual({
            problems: [
                {id: evidence(2), kind: "split-view", loggedAt: 1_800_000_002, entry: 2},
                {id: evidence(1), kind: "split-view", loggedAt: 1_800_000_001, entry: 1},
            ],
            more: true,
        })
    })

    it("leaves out rows that a banner could not show or that are for another policy", () => {
        const parsed = parseProblemPage(
            page([
                row(1),
                row(2, {id: "nope"}),
                row(3, {kind: "Split View"}),
                row(4, {policy: `sha256:${"cd".repeat(32)}`}),
                row(5, {logged_at: -1}),
                row(6, {entry: 1.5}),
                row(7, {logged_at: "now"}),
                "text",
                null,
            ]),
            POLICY
        )
        expect(parsed.problems.map((p) => p.entry)).toEqual([1])
        expect(parsed.more).toBe(false)
    })

    it.each(["", "{", "[]", "null", '{"problems":"x"}', "{}"])("refuses %p", (text) => {
        expect(() => parseProblemPage(text, POLICY)).toThrow(TypeError)
    })
})

describe("problemsUrl", () => {
    it("asks for the policy, a page of rows and, after the first, the older ones", () => {
        expect(problemsUrl("https://hub1.example/", POLICY)).toBe(
            `https://hub1.example/problems?policy=${encodeURIComponent(POLICY)}&limit=50`
        )
        expect(problemsUrl("https://hub1.example/h", POLICY, 7)).toBe(
            `https://hub1.example/h/problems?policy=${encodeURIComponent(POLICY)}&limit=50&before=7`
        )
    })
})

describe("fetchHubProblems", () => {
    it("reads a list without credentials, referrer or redirects", async () => {
        const fetcher = jest.fn<ReturnType<TFetch>, Parameters<TFetch>>(async () =>
            answer(page([row(1)]))
        )
        const got = await fetchHubProblems("https://hub1.example/", POLICY, {fetch: fetcher})
        expect(got.map((p) => p.id)).toEqual([evidence(1)])
        expect(fetcher).toHaveBeenCalledTimes(1)
        expect(fetcher.mock.calls[0][0]).toBe(problemsUrl("https://hub1.example/", POLICY))
        expect(fetcher.mock.calls[0][1]).toMatchObject({
            credentials: "omit",
            referrerPolicy: "no-referrer",
            redirect: "error",
        })
    })

    it("follows older pages by entry index, up to the page limit", async () => {
        const fetcher = jest.fn(async (url: string) =>
            answer(url.includes("before=") ? page([row(2)], true) : page([row(4), row(3)], true))
        )
        const got = await fetchHubProblems("https://hub1.example/", POLICY, {fetch: fetcher})
        expect(got.map((p) => p.entry)).toEqual([4, 3, 2])
        expect(fetcher).toHaveBeenCalledTimes(MAX_PROBLEM_PAGES)
        expect(fetcher.mock.calls[1][0]).toContain("before=3")
    })

    it("stops when a page does not move on", async () => {
        const fetcher = jest.fn(async () => answer(page([row(3)], true)))
        await fetchHubProblems("https://hub1.example/", POLICY, {fetch: fetcher})
        expect(fetcher).toHaveBeenCalledTimes(2)
    })

    it("throws for a hub that answers with an error, a huge body or something else", async () => {
        await expect(
            fetchHubProblems("https://hub1.example/", POLICY, {fetch: async () => answer("", 503)})
        ).rejects.toThrow()
        await expect(
            fetchHubProblems("https://hub1.example/", POLICY, {
                fetch: async () => answer(" ".repeat((1 << 20) + 1)),
            })
        ).rejects.toThrow()
        await expect(
            fetchHubProblems("https://hub1.example/", POLICY, {fetch: async () => answer("<html>")})
        ).rejects.toThrow(TypeError)
    })

    it("does not ask an address that is not https or local http", async () => {
        const fetcher = jest.fn()
        await expect(
            fetchHubProblems("http://hub1.example/", POLICY, {fetch: fetcher})
        ).rejects.toThrow()
        expect(fetcher).not.toHaveBeenCalled()
    })
})

describe("hubProblems", () => {
    const policy: Policy = verify.pinPolicy(policyText({hubExplorers: true}))

    it("asks every hub of the policy, lists a problem once with the hubs that list it, newest first", async () => {
        const fetcher = jest.fn(async (url: string) => {
            const forPolicy = (r: ReturnType<typeof row>) => ({...r, policy: policy.id})
            if (url.startsWith("https://hub1.example/"))
                return answer(page([forPolicy(row(1)), forPolicy(row(5, {kind: "bad-entry"}))]))
            if (url.startsWith("https://hub2.example/")) return answer(page([forPolicy(row(1))]))
            return answer("", 503)
        })
        const found = await hubProblems(policy, {fetch: fetcher})
        expect(found.problems.map((p) => [p.entry, p.hubs])).toEqual([
            [5, ["h1"]],
            [1, ["h1", "h2"]],
        ])
        expect(found.unreachable).toEqual([])
    })

    it("leaves a hub that stays silent out, without calling that a problem", async () => {
        const found = await hubProblems(policy, {
            fetch: async (url: string) =>
                url.startsWith("https://hub1.example/")
                    ? answer(page([{...row(1), policy: policy.id}]))
                    : answer("", 503),
        })
        expect(found.problems.map((p) => p.hubs)).toEqual([["h1"]])
        expect(found.unreachable).toEqual(["h2"])
    })

    it("finds nothing, and throws nothing, when no hub answers", async () => {
        const found = await hubProblems(policy, {
            fetch: async () => {
                throw new Error("offline")
            },
        })
        expect(found.problems).toEqual([])
        expect(found.unreachable.length).toBe(policy.hubs.size)
    })
})

describe("words and links", () => {
    it.each([
        ["split-view", "splitView"],
        ["broken-promise", "brokenPromise"],
        ["bad-entry", "breaksRules"],
        ["hub-entry", "breaksRules"],
        ["time-regression", "breaksRules"],
        ["bad-checkpoint", "breaksRules"],
        ["policy-equivocation", "twoPolicies"],
        ["unlogged-code", "unloggedCode"],
        ["something-new", "other"],
    ])("words %s in plain words, never as its code", (kind, key) => {
        expect(problemKindKey(kind)).toBe(`transparencyTab.hubProblems.kind.${key}`)
    })

    it("links to the problem on the hub's site", () => {
        expect(problemLink("https://explorer1.example/", POLICY, evidence(1))).toBe(
            `https://explorer1.example/#policy=${encodeURIComponent(POLICY)}&problem=${"0".repeat(63)}1`
        )
        expect(problemLink("https://explorer1.example/#old", POLICY, evidence(1))).toContain(
            "/#policy="
        )
    })

    it("does not link to a site that is not an address to open", () => {
        expect(problemLink(undefined, POLICY, evidence(1))).toBeUndefined()
        expect(problemLink("javascript:alert(1)", POLICY, evidence(1))).toBeUndefined()
        expect(problemLink("http://explorer1.example/", POLICY, evidence(1))).toBeUndefined()
    })
})
