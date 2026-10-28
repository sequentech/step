// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ApolloClient} from "@apollo/client"
import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {
    ELECTION_ID,
    EVENT_ID,
    MONITOR_URL,
    NOW,
    PUBLIC_KEY,
    RELEASE,
    STYLE_TEXT,
    scenario,
} from "../../fixtures/vitrine"
import {IRunBoothCheck, runBoothCheck} from "./booth"
import {EBoothCheckReason, EBoothRefusalState} from "./boothCheck"
import {electionKeyBytes} from "./digests"
import type {THttpInit} from "./http"
import {MAX_REPORT_DELAY_MS, OBSERVATION_HEADER, OBSERVATION_TYPE} from "./observation"
import type {IVitrineSdk} from "./sdk"

const sdk: IVitrineSdk = {verify, pure}

beforeAll(async () => {
    await verify.init()
})

const fakeClient = (query: jest.Mock) => ({query}) as unknown as ApolloClient

const answering = (context: unknown) =>
    jest.fn(async () => ({data: {vitrine_voter_context: context}}))

const okResponse = () =>
    ({status: 202, arrayBuffer: async () => new ArrayBuffer(0)}) as unknown as Response

const setup = (overrides: Partial<IRunBoothCheck> = {}) => {
    const s = scenario()
    const query = answering(s.context)
    const fetch = jest.fn(async (_url: string, _init?: THttpInit) => okResponse())
    const schedule = jest.fn((send: () => Promise<void>, _delayMs: number) => {
        void send()
    })
    const loadSdk = jest.fn(async () => sdk)
    const options: IRunBoothCheck = {
        client: fakeClient(query),
        style: {
            electionEventId: EVENT_ID,
            electionId: ELECTION_ID,
            publicKey: PUBLIC_KEY,
            styleText: STYLE_TEXT,
        },
        eventPresentation: s.presentation,
        release: RELEASE,
        skip: false,
        now: () => NOW * 1000,
        report: {fetch, schedule, random: () => 0.5},
        loadSdk,
        ...overrides,
    }
    return {s, query, fetch, schedule, loadSdk, options}
}

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))

describe("runBoothCheck", () => {
    it("passes events without transparency without loading anything", async () => {
        const {query, loadSdk, options} = setup({eventPresentation: {}})
        await expect(runBoothCheck(options)).resolves.toEqual({ok: true})
        expect(loadSdk).not.toHaveBeenCalled()
        expect(query).not.toHaveBeenCalled()
    })

    it("passes previews and sessions without a voter without asking Harvest", async () => {
        const {query, loadSdk, options} = setup({skip: true})
        await expect(runBoothCheck(options)).resolves.toEqual({ok: true})
        expect(loadSdk).not.toHaveBeenCalled()
        expect(query).not.toHaveBeenCalled()
    })

    it("reads the transparency mode from the ballot style's copy of the presentation", async () => {
        const {s, query, options} = setup({eventPresentation: undefined})
        const style = {...options.style, presentation: s.presentation}
        await expect(runBoothCheck({...options, style})).resolves.toEqual({ok: true})
        expect(query).toHaveBeenCalledTimes(1)
    })

    it("asks Harvest for the context, never the cache, and passes", async () => {
        const {query, fetch, options} = setup()
        await expect(runBoothCheck(options)).resolves.toEqual({ok: true})
        expect(query).toHaveBeenCalledWith(
            expect.objectContaining({
                variables: {electionEventId: EVENT_ID, electionId: ELECTION_ID},
                fetchPolicy: "network-only",
            })
        )
        await flush()
        expect(fetch).not.toHaveBeenCalled()
    })

    it("refuses calmly when the verifier can't be loaded", async () => {
        const {query, options} = setup({loadSdk: jest.fn(async () => Promise.reject(new Error()))})
        await expect(runBoothCheck(options)).resolves.toEqual({
            ok: false,
            reason: EBoothCheckReason.VERIFIER_UNAVAILABLE,
            state: EBoothRefusalState.PENDING,
        })
        expect(query).not.toHaveBeenCalled()
    })

    it("reports unavailable context to monitors of a separately pinned public policy", async () => {
        const query = jest.fn(async () => Promise.reject(new Error("offline")))
        const {fetch, options} = setup()
        await expect(runBoothCheck({...options, client: fakeClient(query)})).resolves.toEqual({
            ok: false,
            reason: EBoothCheckReason.CONTEXT_MISSING,
            state: EBoothRefusalState.PENDING,
        })
        await flush()
        expect(fetch).toHaveBeenCalledTimes(2)
    })

    it.each([
        ["key", EBoothCheckReason.KEY_MISMATCH],
        ["styles", EBoothCheckReason.STYLES_MISMATCH],
        ["config", EBoothCheckReason.CONFIG_MISSING],
        ["opened", EBoothCheckReason.OPENED_MISSING],
    ] as const)("refuses and reports a %s mismatch", async (part, reason) => {
        const {s, options, fetch} = setup()
        const context = {
            ...s.context,
            ...(part === "config" ? {config: null} : {}),
            ...(part === "opened" ? {opened: null} : {}),
        }
        const style = {
            ...options.style,
            ...(part === "key" ? {publicKey: ""} : {}),
            ...(part === "styles" ? {styleText: "changed style"} : {}),
        }
        const result = await runBoothCheck({
            ...options,
            style,
            client: fakeClient(answering(context)),
        })
        expect(result).toMatchObject({ok: false, reason})
        await flush()
        expect(fetch).toHaveBeenCalledTimes(2)
        expect(String(fetch.mock.calls[0][1]?.body)).toContain(`problem ${reason}`)
    })

    it("sends the policy's monitors one observation of a config that contradicts the booth", async () => {
        const {s, fetch, schedule, options} = setup({release: "10.5.0"})
        const refused = {
            ok: false,
            reason: EBoothCheckReason.RELEASE_MISMATCH,
            state: EBoothRefusalState.FAILED,
        }
        await expect(runBoothCheck(options)).resolves.toEqual(refused)
        await flush()

        expect(schedule).toHaveBeenCalledTimes(2)
        expect(schedule.mock.calls[0][1]).toBe(MAX_REPORT_DELAY_MS / 2)
        expect(fetch).toHaveBeenCalledTimes(2)
        const [url, init] = fetch.mock.calls[0]
        expect(url).toBe(`${MONITOR_URL}observe`)
        expect(init).toMatchObject({
            method: "POST",
            credentials: "omit",
            referrerPolicy: "no-referrer",
            redirect: "error",
            headers: {"Content-Type": OBSERVATION_TYPE},
        })

        const body = String(init?.body)
        const doc = pure.blocks.parseBlockDoc(body, {
            header: OBSERVATION_HEADER,
            keys: ["policy", "kind", "time", "event", "election", "problem"],
            code: "bad-proof",
        })
        expect(doc.values).toEqual([
            [s.policyId],
            ["config-mismatch"],
            [String(NOW)],
            [EVENT_ID],
            [ELECTION_ID],
            [
                EBoothCheckReason.RELEASE_MISMATCH,
                "package",
                "sequent/voting-portal",
                "version",
                "10.5.0",
            ],
        ])
        expect(doc.blocks.map((b) => b.type)).toEqual([
            "config",
            "opened",
            "key",
            "styles",
            "style",
        ])
        expect(pure.bytes.fromUtf8(doc.blocks[0].body)).toBe(
            pure.bytes.fromUtf8(pure.parseChain(s.context.config?.chain ?? "").record!)
        )
        expect(Array.from(doc.blocks[2].body)).toEqual(
            Array.from(electionKeyBytes(pure, PUBLIC_KEY)!)
        )
        expect(pure.bytes.fromUtf8(doc.blocks[4].body)).toBe(STYLE_TEXT)
        expect(doc.blocks[4].args).toEqual([ELECTION_ID])

        await expect(runBoothCheck(options)).resolves.toEqual(refused)
        await flush()
        expect(fetch).toHaveBeenCalledTimes(2)
    })
})
