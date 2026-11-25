// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PublicVerificationReport, VerificationAnchors} from "./BrowserTrustee"
import type {TallyVerificationBoard} from "./TallyPublicVerification"

/** The public verifier, answering off the page's thread. */
export interface Verifier {
    /** Loads the verifier; an error means it cannot run in this browser. */
    ready(): Promise<void>
    /** Anchors derived from the platform's records of a board, never from the board itself. */
    anchors(board: TallyVerificationBoard): Promise<VerificationAnchors>
    verify(transcript: string, anchors: VerificationAnchors): Promise<PublicVerificationReport>
    /** Stops the verifier; questions still unanswered fail. */
    close(): void
}

/** The worker side, a module of its own: it loads the browser trustees' WASM bundle. */
const SOURCE = `
let loading
const load = (base, threads) =>
    (loading ??= (async () => {
        const module = await import(new URL("index.js", base).href)
        await module.default({module_or_path: new URL("index_bg.wasm", base).href})
        await module.initThreadPool(threads)
        return module.WasmVerifier
    })())

self.onmessage = async (event) => {
    const {id, kind, base, threads, args} = event.data
    try {
        const verifier = await load(base, threads)
        let result = null
        if (kind === "anchors") {
            const [configuration, ballots] = args
            result = {
                configuration_hash: verifier.configurationHash(configuration),
                tally_id: verifier.tallyIdentifier(configuration, ballots),
                ballots_hash: verifier.ballotsHash(ballots),
                ballot_ciphertexts: [],
            }
        } else if (kind === "verify") {
            result = verifier.verifyTranscript(args[0], args[1])
        }
        self.postMessage({id, result})
    } catch (error) {
        self.postMessage({id, error: String(error)})
    }
}
`

type Reply = {id: number; result: unknown} | {id: number; error: string}

/** What the page needs of a worker. */
export interface VerifierThread {
    onmessage: ((event: MessageEvent<Reply>) => void) | null
    onerror: ((event: ErrorEvent) => void) | null
    postMessage(message: unknown): void
    terminate(): void
}

const spawnModuleWorker = (url: string): VerifierThread => new Worker(url, {type: "module"})

/** The verifier's thread pool: the browser's cores, at most four, at least one. */
export const verifierThreads = (cores: number | undefined): number =>
    Math.min(Math.max(cores ?? 1, 1), 4)

/**
 * The verifier in a dedicated worker, so a tally of any size never blocks the
 * page: the bundle at `base` is loaded there, with a pool of `threads`.
 */
export function workerVerifier(
    base: string,
    threads: number,
    spawn: (url: string) => VerifierThread = spawnModuleWorker
): Verifier {
    const url = URL.createObjectURL(new Blob([SOURCE], {type: "text/javascript"}))
    const worker = spawn(url)
    const pending = new Map<
        number,
        {resolve: (value: unknown) => void; reject: (error: Error) => void}
    >()
    let next = 0
    const failAll = (reason: string) => {
        pending.forEach((call) => call.reject(new Error(reason)))
        pending.clear()
    }
    worker.onmessage = ({data}) => {
        const call = pending.get(data.id)
        if (!call) return
        pending.delete(data.id)
        if ("error" in data) call.reject(new Error(data.error))
        else call.resolve(data.result)
    }
    worker.onerror = (event) => failAll(event.message || "the verifier stopped")
    const ask = <T>(kind: string, args: unknown[]): Promise<T> =>
        new Promise<T>((resolve, reject) => {
            const id = next++
            pending.set(id, {resolve: resolve as (value: unknown) => void, reject})
            worker.postMessage({id, kind, base, threads, args})
        })
    return {
        ready: async () => {
            await ask<null>("ready", [])
        },
        anchors: (board) =>
            ask<VerificationAnchors>("anchors", [
                board.configuration_message,
                board.ballots_message,
            ]),
        verify: async (transcript, anchors) =>
            JSON.parse(await ask<string>("verify", [transcript, JSON.stringify(anchors)])),
        close: () => {
            worker.terminate()
            URL.revokeObjectURL(url)
            failAll("the verifier was closed")
        },
    }
}
