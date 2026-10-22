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
export interface VerificationAnchors {
    configuration_hash: string
    tally_id: string | null
    ballots_hash?: string | null
    ballot_ciphertexts?: string[]
}
export interface PublicVerificationReport {
    status: "verified" | "incomplete" | "invalid"
    detail: string
    configuration_hash: string
    tally_id: string | null
    public_key_hash: string | null
    ballot_count: number | null
    plaintexts: string[]
    ballot_inclusions: {ciphertext_hash: string; occurrences: number}[]
}
export interface TrusteeModule {
    default(options: {module_or_path: string}): Promise<unknown>
    initThreadPool(threads: number): Promise<void>
    BrowserTrustee: {
        new (
            name: string,
            keys: string,
            boardUrl: string,
            tokenProvider?: () => Promise<string>
        ): TrusteeEngine
        generateKeys(): string
    }
    WasmVerifier: {
        fetchTranscript(boardUrl: string, dkgBoard: string, tallyBoard?: string): Promise<string>
        verifyTranscript(evidence: string, anchors: string): string
        configurationHash(trustedConfigurationMessage: string): string
        ballotsHash(trustedBallotsMessage: string): string
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
interface CustodyRequest {
    tenant_id: string
    election_event_id: string
    keys_ceremony_id: string
    tally_session_id: string | null
    trustee_name: string
    signing_public_key: string
    share_encryption_public_key: string
}
interface CustodyAction {
    id: string
    kind: "CREATE_CUSTODY_PROOF"
    request: CustodyRequest
}
type CustodyOutcome =
    | {status: "READY"; custody_proof_json: string}
    | {status: "FAILED"; detail: string}

function parseCustodyActions(value: unknown): CustodyAction[] {
    if (typeof value !== "object" || value === null)
        throw new Error("Invalid trustee event updates.")
    if (!("actions" in value)) return []
    if (!Array.isArray(value.actions)) throw new Error("Invalid trustee actions.")
    const actions: CustodyAction[] = []
    for (const action of value.actions as unknown[]) {
        if (typeof action !== "object" || action === null || !("kind" in action))
            throw new Error("Invalid trustee action.")
        // Future action kinds must not stop this runner's protocol boards.
        if (action.kind !== "CREATE_CUSTODY_PROOF") continue
        if (!("id" in action) || typeof action.id !== "string" || !("request" in action))
            throw new Error("Invalid custody action.")
        const request = action.request
        if (
            typeof request !== "object" ||
            request === null ||
            ![
                "tenant_id",
                "election_event_id",
                "keys_ceremony_id",
                "trustee_name",
                "signing_public_key",
                "share_encryption_public_key",
            ].every((key) => key in request && typeof Reflect.get(request, key) === "string") ||
            !("tally_session_id" in request) ||
            !(request.tally_session_id === null || typeof request.tally_session_id === "string")
        )
            throw new Error("Invalid custody request.")
        actions.push({
            id: action.id,
            kind: "CREATE_CUSTODY_PROOF",
            request: request as CustodyRequest,
        })
    }
    return actions
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

/**
 * The board's token provider over `token`, the signed-in user's in-memory
 * access token. It is read before each board request, so it is fresh after
 * long computations; the trustee never persists it.
 */
export function boardTokenProvider(token: () => string): () => Promise<string> {
    return async () => {
        const current = token()
        if (!current) throw new Error("Sign in as a trustee to reach the board service.")
        return current
    }
}

/** Every engine reaches the board service with the portal's current access token. */
export function createBrowserTrustee(
    module: Pick<TrusteeModule, "BrowserTrustee">,
    name: string,
    keys: string,
    boardUrl: string,
    token: () => string
): TrusteeEngine {
    return new module.BrowserTrustee(name, keys, boardUrl, boardTokenProvider(token))
}

async function trusteeGraphql<T>(
    platform: string,
    token: string,
    query: string,
    variables: Record<string, unknown> = {}
): Promise<T> {
    if (!token) throw new Error("Sign in as a trustee to run assigned boards.")
    const response = await fetch(platform, {
        method: "POST",
        headers: {
            "Authorization": `Bearer ${token}`,
            "Content-Type": "application/json",
            "x-hasura-role": "trustee-ceremony",
        },
        body: JSON.stringify({query, variables}),
    })
    if (!response.ok) throw new Error(`Trustee API request failed (${response.status}).`)
    const result: {data?: T; errors?: {message: string}[]} = await response.json()
    if (result.errors?.length)
        throw new Error(result.errors.map((error) => error.message).join("; "))
    if (!result.data) throw new Error("Invalid trustee API response.")
    return result.data
}

/** Discover assigned boards through the platform's public GraphQL endpoint. */
export async function listBrowserTrusteeBoards(
    platform: string,
    token: string
): Promise<TrusteeBoard[]> {
    const data = await trusteeGraphql<{list_trustee_boards: unknown}>(
        platform,
        token,
        "query ListTrusteeBoards { list_trustee_boards { boards { name kind parent } } }"
    )
    return parseBoards(data.list_trustee_boards)
}

/**
 * One sequential cycle; callers never overlap cycles or release a busy engine.
 * `token` is read before each request: `advance` can outlast an access token.
 */
export async function runBrowserCycle(
    engine: TrusteeEngine,
    platform: string,
    token: () => string,
    remember: (boards: TrusteeBoard[]) => void
): Promise<TrusteeBoard[]> {
    const data = await trusteeGraphql<{trustee_event_updates: unknown}>(
        platform,
        token(),
        "query TrusteeEventUpdates { trustee_event_updates }"
    )
    const boards = parseBoards(data.trustee_event_updates)
    const actions = parseCustodyActions(data.trustee_event_updates)
    remember(boards)
    for (const action of actions) {
        let outcome: CustodyOutcome
        try {
            outcome = {
                status: "READY",
                custody_proof_json: engine.attest(JSON.stringify(action.request)),
            }
        } catch (error: unknown) {
            outcome = {
                status: "FAILED",
                detail: error instanceof Error ? error.message : String(error),
            }
        }
        const acknowledgement = await trusteeGraphql<{
            submit_trustee_action_result: {accepted: boolean} | null
        }>(
            platform,
            token(),
            "mutation SubmitTrusteeActionResult($result: jsonb!) { submit_trustee_action_result(result: $result) { accepted } }",
            {result: {id: action.id, request: action.request, ...outcome}}
        )
        if (acknowledgement.submit_trustee_action_result?.accepted !== true)
            throw new Error("The platform did not acknowledge the custody action.")
    }
    const reports: TrusteeReport[] = JSON.parse(await engine.advance(JSON.stringify(boards)))
    for (const report of reports) {
        const data = await trusteeGraphql<{report_trustee_board: {accepted: boolean} | null}>(
            platform,
            token(),
            "mutation ReportTrusteeBoard($report: jsonb!) { report_trustee_board(report: $report) { accepted } }",
            {report}
        )
        if (data.report_trustee_board?.accepted !== true)
            throw new Error("The platform did not acknowledge the halted board.")
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
