/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {verifierThreads, workerVerifier, type VerifierThread} from "./VerifierWorker"
import type {TallyVerificationBoard} from "./TallyPublicVerification"

interface Question {
    id: number
    kind: string
    base: string
    threads: number
    args: unknown[]
}

/** A worker the test answers by hand. */
class FakeThread implements VerifierThread {
    onmessage: VerifierThread["onmessage"] = null
    onerror: VerifierThread["onerror"] = null
    questions: Question[] = []
    terminated = false
    postMessage(message: unknown) {
        this.questions.push(message as Question)
    }
    terminate() {
        this.terminated = true
    }
    answer(data: unknown) {
        this.onmessage?.({data} as MessageEvent<never>)
    }
    fail(message: string) {
        this.onerror?.({message} as ErrorEvent)
    }
}

const board = {
    configuration_message: "Y29uZmlndXJhdGlvbg==",
    ballots_message: "YmFsbG90cw==",
} as TallyVerificationBoard

let urls: string[]
let revoked: string[]
beforeEach(() => {
    urls = []
    revoked = []
    Object.assign(URL, {
        createObjectURL: jest.fn(() => {
            urls.push(`blob:verifier-${urls.length}`)
            return urls[urls.length - 1]
        }),
        revokeObjectURL: jest.fn((url: string) => revoked.push(url)),
    })
})

function start() {
    const thread = new FakeThread()
    const spawned: string[] = []
    const verifier = workerVerifier("https://admin.example/wbraid-trustee/", 3, (url) => {
        spawned.push(url)
        return thread
    })
    return {thread, spawned, verifier}
}

describe("verifierThreads", () => {
    it("uses the browser's cores, at most four and at least one", () => {
        expect(verifierThreads(8)).toBe(4)
        expect(verifierThreads(2)).toBe(2)
        expect(verifierThreads(0)).toBe(1)
        expect(verifierThreads(undefined)).toBe(1)
    })
})

describe("workerVerifier", () => {
    it("starts a module worker from its own source and asks it each question with the bundle's address", async () => {
        const {thread, spawned, verifier} = start()
        expect(spawned).toEqual(["blob:verifier-0"])

        const ready = verifier.ready()
        const anchors = verifier.anchors(board)
        const report = verifier.verify("transcript", {
            configuration_hash: "c",
            tally_id: "42",
            ballots_hash: "b",
            ballot_ciphertexts: [],
        })
        expect(thread.questions).toEqual([
            {
                id: 0,
                kind: "ready",
                base: "https://admin.example/wbraid-trustee/",
                threads: 3,
                args: [],
            },
            {
                id: 1,
                kind: "anchors",
                base: "https://admin.example/wbraid-trustee/",
                threads: 3,
                args: [board.configuration_message, board.ballots_message],
            },
            {
                id: 2,
                kind: "verify",
                base: "https://admin.example/wbraid-trustee/",
                threads: 3,
                args: [
                    "transcript",
                    JSON.stringify({
                        configuration_hash: "c",
                        tally_id: "42",
                        ballots_hash: "b",
                        ballot_ciphertexts: [],
                    }),
                ],
            },
        ])

        // Answers arrive in any order, and one for no question is ignored.
        thread.answer({id: 99, result: "stray"})
        thread.answer({id: 2, result: JSON.stringify({status: "verified"})})
        thread.answer({id: 1, result: {configuration_hash: "c"}})
        thread.answer({id: 0, result: null})
        await expect(ready).resolves.toBeUndefined()
        await expect(anchors).resolves.toEqual({configuration_hash: "c"})
        await expect(report).resolves.toEqual({status: "verified"})
    })

    it("rejects a question the verifier could not answer", async () => {
        const {thread, verifier} = start()
        const ready = verifier.ready()
        thread.answer({id: 0, error: "RuntimeError: unreachable"})
        await expect(ready).rejects.toThrow("RuntimeError: unreachable")
    })

    it("fails every open question when the worker stops", async () => {
        const {thread, verifier} = start()
        const first = verifier.ready()
        const second = verifier.verify("transcript", {
            configuration_hash: "c",
            tally_id: null,
        })
        thread.fail("SharedArrayBuffer is not defined")
        await expect(first).rejects.toThrow("SharedArrayBuffer is not defined")
        await expect(second).rejects.toThrow("SharedArrayBuffer is not defined")

        const third = verifier.ready()
        thread.fail("")
        await expect(third).rejects.toThrow("the verifier stopped")
    })

    it("stops the worker and releases its source when closed", async () => {
        const {thread, verifier} = start()
        const open = verifier.anchors(board)
        verifier.close()
        expect(thread.terminated).toBe(true)
        expect(revoked).toEqual(["blob:verifier-0"])
        await expect(open).rejects.toThrow("the verifier was closed")
    })

    it("runs in a module worker by default", () => {
        const created: {url: string; options: unknown}[] = []
        const original = (globalThis as {Worker?: unknown}).Worker
        ;(globalThis as {Worker?: unknown}).Worker = class {
            constructor(url: string, options: unknown) {
                created.push({url, options})
            }
            postMessage() {}
            terminate() {}
        }
        try {
            workerVerifier("https://admin.example/wbraid-trustee/", 2)
            expect(created).toEqual([{url: "blob:verifier-0", options: {type: "module"}}])
        } finally {
            ;(globalThis as {Worker?: unknown}).Worker = original
        }
    })
})
