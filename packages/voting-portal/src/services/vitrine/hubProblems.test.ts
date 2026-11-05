// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {Policy} from "@vitrine/verify"
import {
    fetchHubProblems,
    hubProblems,
    parseProblemPage,
    problemKindKey,
    problemLink,
    problemsUrl,
} from "./hubProblems"

const POLICY = `sha256:${"a".repeat(64)}`
const EVIDENCE = (n: number) => `sha256:${n.toString(16).padStart(64, "0")}`

const row = (n: number, extra: Record<string, unknown> = {}) => ({
    id: EVIDENCE(n),
    kind: "split-view",
    subject: "ballots",
    policy: POLICY,
    logged_at: 1_793_270_000 + n,
    entry: 100 + n,
    ...extra,
})

const answer = (problems: unknown[], more = false) => JSON.stringify({hub: "hub-a", problems, more})

const policyOf = (hubs: Record<string, string>) =>
    ({
        id: POLICY,
        hubs: new Map(Object.entries(hubs).map(([name, url]) => [name, {name, url}])),
    }) as unknown as Policy

/** A fetch that answers by URL prefix. */
const hubFetch =
    (answers: Record<string, () => Response | Promise<Response>>) => async (input: string) => {
        const key = Object.keys(answers).find((prefix) => input.startsWith(prefix))
        if (!key) throw new Error("no route")
        return answers[key]()
    }

const reply = (status: number, body: string) =>
    ({
        status,
        headers: {get: () => null},
        arrayBuffer: async () => new TextEncoder().encode(body).buffer,
    }) as unknown as Response
const ok = (body: string) => reply(200, body)

describe("the problems a hub lists", () => {
    it("asks for the policy's problems and nothing else", () => {
        expect(problemsUrl("https://hub.example.org", POLICY)).toBe(
            `https://hub.example.org/problems?policy=sha256%3A${"a".repeat(64)}&limit=50`
        )
        expect(problemsUrl("https://hub.example.org/", POLICY, 7)).toContain("&before=7")
    })

    it("reads the list, leaving out rows that are not whole", () => {
        const page = parseProblemPage(
            answer(
                [
                    row(1),
                    row(2, {kind: "Not A Kind"}),
                    row(3, {id: "sha256:zz"}),
                    row(4, {policy: `sha256:${"b".repeat(64)}`}),
                    row(5, {logged_at: -1}),
                    row(6, {entry: "9"}),
                    "text",
                ],
                true
            ),
            POLICY
        )
        expect(page.problems.map((problem) => problem.id)).toEqual([EVIDENCE(1)])
        expect(page.more).toBe(true)
    })

    it("refuses what is not a list", () => {
        expect(() => parseProblemPage("[]", POLICY)).toThrow(TypeError)
        expect(() => parseProblemPage("nope", POLICY)).toThrow(TypeError)
        expect(() => parseProblemPage('{"problems":1}', POLICY)).toThrow(TypeError)
    })

    it("follows more up to two pages, by the entry index", async () => {
        const urls: string[] = []
        const pages = [answer([row(9)], true), answer([row(3)], true), answer([row(1)])]
        const fetch = async (input: string) => {
            urls.push(input)
            return ok(pages[urls.length - 1])
        }
        const got = await fetchHubProblems("https://hub.example.org", POLICY, {fetch})
        expect(got.map((problem) => problem.id)).toEqual([EVIDENCE(9), EVIDENCE(3)])
        expect(urls[1]).toContain("before=109")
    })

    it("throws when the hub does not answer", async () => {
        await expect(
            fetchHubProblems("https://hub.example.org", POLICY, {
                fetch: async () => reply(503, ""),
            })
        ).rejects.toThrow()
    })
})

describe("the problems the hubs of a policy list", () => {
    it("lists each once, with the hubs that list it, newest first", async () => {
        const fetch = hubFetch({
            "https://hub-a.example.org": () => ok(answer([row(2), row(1)])),
            "https://hub-b.example.org": () =>
                ok(answer([row(3, {kind: "broken-promise"}), row(2)])),
        })
        const found = await hubProblems(
            policyOf({"hub-a": "https://hub-a.example.org", "hub-b": "https://hub-b.example.org"}),
            {fetch}
        )
        expect(found.problems.map((problem) => [problem.id, problem.hubs])).toEqual([
            [EVIDENCE(3), ["hub-b"]],
            [EVIDENCE(2), ["hub-a", "hub-b"]],
            [EVIDENCE(1), ["hub-a"]],
        ])
        expect(found.unreachable).toEqual([])
    })

    it("leaves a hub that does not answer out, and never takes silence for a problem", async () => {
        const fetch = hubFetch({
            "https://hub-a.example.org": () => ok(answer([])),
            "https://hub-b.example.org": () => {
                throw new Error("down")
            },
        })
        const found = await hubProblems(
            policyOf({"hub-a": "https://hub-a.example.org", "hub-b": "https://hub-b.example.org"}),
            {fetch}
        )
        expect(found.problems).toEqual([])
        expect(found.unreachable).toEqual(["hub-b"])
    })
})

describe("the words for a problem", () => {
    it("are D4's plain words, and never the code of an unknown kind", () => {
        expect(problemKindKey("split-view")).toBe("vitrine.hubProblems.kind.splitView")
        expect(problemKindKey("broken-promise")).toBe("vitrine.hubProblems.kind.brokenPromise")
        for (const kind of ["bad-entry", "hub-entry", "time-regression", "bad-checkpoint"]) {
            expect(problemKindKey(kind)).toBe("vitrine.hubProblems.kind.breaksRules")
        }
        expect(problemKindKey("policy-equivocation")).toBe("vitrine.hubProblems.kind.twoPolicies")
        expect(problemKindKey("unlogged-code")).toBe("vitrine.hubProblems.kind.unloggedCode")
        for (const unknown of ["promise-late", "unreachable", "anything"]) {
            expect(problemKindKey(unknown)).toBe("vitrine.hubProblems.kind.other")
        }
    })

    it("link to the problem's page on the hub's own site", () => {
        expect(problemLink("https://hub-a.example.org/#old", POLICY, EVIDENCE(2))).toBe(
            `https://hub-a.example.org/#policy=sha256%3A${"a".repeat(64)}&problem=${"0".repeat(63)}2`
        )
    })
})
