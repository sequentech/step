// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

export interface TrusteeBoard {
    name: string
    kind: "DKG" | "TALLY"
    parent: string | null
}
export interface TrusteeKeys {
    signing_public_key: string
    share_encryption_public_key: string
}
export interface TrusteeEngine {
    publicKeys(): string
    attest(request: string): string
    advance(boards: string): Promise<string>
    settle(board: string): void
    backup(boards: string): Promise<string>
    restoreBackup(contents: string): Promise<string>
    free(): void
}
interface TrusteeModule {
    default(options: {module_or_path: string}): Promise<unknown>
    initThreadPool(threads: number): Promise<void>
    BrowserTrustee: {
        new (name: string, keys: string, boardUrl: string): TrusteeEngine
        generateKeys(): string
    }
}
let loading: Promise<TrusteeModule> | undefined
export function loadBrowserTrustee(): Promise<TrusteeModule> {
    if (!loading) {
        loading = (async () => {
            if (!window.crossOriginIsolated)
                throw new Error(
                    "Browser trustees require HTTPS and cross-origin isolation (COOP/COEP)."
                )
            const base = new URL("/wbraid-trustee/", window.location.origin)
            const module: TrusteeModule = await import(
                /* webpackIgnore: true */ /* @vite-ignore */ new URL("index.js", base).href
            )
            await module.default({module_or_path: new URL("index_bg.wasm", base).href})
            await module.initThreadPool(Math.min(navigator.hardwareConcurrency || 2, 4))
            return module
        })().catch((error: unknown) => {
            loading = undefined
            throw error
        })
    }
    return loading
}

interface TrusteeReport {
    board: string
    kind: "HALTED"
    detail: string
}
const boardPattern = /^(dkg_[0-9a-f]{32}|tally_[0-9a-f]{32}_[0-9]+)$/
export function parseBoards(value: unknown): TrusteeBoard[] {
    if (
        typeof value !== "object" ||
        value === null ||
        !("boards" in value) ||
        !Array.isArray(value.boards)
    )
        throw new Error("Invalid trustee board response.")
    return value.boards.map((board: unknown) => {
        if (
            typeof board !== "object" ||
            board === null ||
            !("name" in board) ||
            typeof board.name !== "string" ||
            !boardPattern.test(board.name) ||
            !("kind" in board) ||
            !("parent" in board)
        )
            throw new Error("Invalid trustee board response.")
        if (board.kind === "DKG" && board.name.startsWith("dkg_") && board.parent === null)
            return {name: board.name, kind: "DKG", parent: null}
        if (
            board.kind === "TALLY" &&
            board.name.startsWith("tally_") &&
            typeof board.parent === "string" &&
            /^dkg_[0-9a-f]{32}$/.test(board.parent)
        )
            return {name: board.name, kind: "TALLY", parent: board.parent}
        throw new Error("Invalid trustee board response.")
    })
}

/** One sequential cycle; callers never overlap cycles or release a busy engine. */
export async function runBrowserCycle(
    engine: TrusteeEngine,
    platform: string,
    token: string,
    remember: (boards: TrusteeBoard[]) => void
): Promise<TrusteeBoard[]> {
    if (!token) throw new Error("Sign in as a trustee to run assigned boards.")
    const headers = {Authorization: `Bearer ${token}`}
    const response = await fetch(`${platform.replace(/\/$/, "")}/trustee/boards`, {headers})
    if (!response.ok) throw new Error(`Could not read assigned boards (${response.status}).`)
    const boards = parseBoards(await response.json())
    remember(boards)
    const reports: TrusteeReport[] = JSON.parse(await engine.advance(JSON.stringify(boards)))
    for (const report of reports) {
        const response = await fetch(`${platform.replace(/\/$/, "")}/trustee/boards/report`, {
            method: "POST",
            headers: {...headers, "Content-Type": "application/json"},
            body: JSON.stringify(report),
        })
        if (!response.ok) throw new Error(`Could not report halted board (${response.status}).`)
        engine.settle(report.board)
    }
    return boards
}

/** Hold one identity's runner lock until the active protocol cycle has stopped. */
export async function runExclusiveBrowserTrustee(
    identity: string,
    run: () => Promise<void>
): Promise<void> {
    if (!navigator.locks) throw new Error("Browser trustees require the Web Locks API.")
    await navigator.locks.request(
        `sequent-wbraid-run:${identity}`,
        {ifAvailable: true},
        async (lock) => {
            if (!lock) throw new Error("This trustee is already running in another tab.")
            await run()
        }
    )
}
