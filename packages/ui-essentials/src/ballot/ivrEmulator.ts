// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {IvrCallDriver} from "./IvrCall"

/**
 * What a call needs to know before it can be placed.
 *
 * The emulator runs the real IVR Lambda compiled to WebAssembly, so it is given
 * exactly what the Lambda gets in production: the event, the ballot styles of the
 * elections that are open, and who is calling. Everything else it works out for
 * itself, which is the point of it.
 */
export interface IvrEmulatorConfig {
    /** E.164, the number the call appears to come from. */
    caller_number: string
    contact_id: string
    tenant_id: string
    election_event_id: string
    /** The election event, as JSON. */
    election_event: string
    /** Each open election's `ballot_eml`, as JSON. */
    ballot_styles: string[]
    open_elections: string[]
    /** E.164 numbers the flow's blacklist step should refuse. */
    blacklisted_numbers: string[]
}

export interface IvrEmulatorApi {
    IvrEmulatorDriver: new (config: IvrEmulatorConfig) => IvrCallDriver
}

/** Which stage failed, because each means something different to a reader. */
export type IvrEmulatorFailure = "fetch" | "load" | "init"

export class IvrEmulatorError extends Error {
    constructor(
        readonly operation: IvrEmulatorFailure,
        readonly msg: string,
        options?: ErrorOptions
    ) {
        super(`Ivr emulator "${operation}" failed: ${msg}`, options)

        this.name = new.target.name
        Object.setPrototypeOf(this, new.target.prototype)
    }
}

/**
 * The two files wasm-bindgen emits, from the one base URL a deployment configures.
 *
 * Query and fragment are dropped rather than carried: they would end up inside the
 * `.js` and `_bg.wasm` names and fetch two things that are not there.
 */
const resolveUrls = (baseUrl: string): {jsUrl: URL; wasmUrl: URL} => {
    const base = new URL(baseUrl, document.baseURI)
    base.search = ""
    base.hash = ""

    const jsUrl = new URL(base)
    jsUrl.pathname += ".js"

    const wasmUrl = new URL(base)
    wasmUrl.pathname += "_bg.wasm"

    return {jsUrl, wasmUrl}
}

const fetchWasm = async (url: URL): Promise<Response> => {
    let response: Response
    try {
        response = await fetch(url)
    } catch (cause) {
        throw new IvrEmulatorError("fetch", `failed to fetch the wasm binary from "${url}"`, {
            cause,
        })
    }

    if (!response.ok) {
        throw new IvrEmulatorError("fetch", `response for "${url}" was "${response.status}"`)
    }

    return response
}

/**
 * Bring in the generated shim, which the bundler must not touch.
 *
 * `importModule` is a parameter because of what the magic comments below do — or
 * rather, where they have to be. They are instructions to *the bundler compiling
 * this line*, and this file is compiled twice: once into the library, and again
 * into whatever application consumes it. A comment cannot survive that. So a host
 * whose bundler would rewrite the import passes its own one-line function with the
 * comments its own bundler reads; the default is right for anything unbundled and
 * for tests.
 */
const defaultImport = (href: string): Promise<unknown> =>
    import(
        /* @vite-ignore */
        /* webpackIgnore: true */
        href
    )

export type ImportModule = (href: string) => Promise<unknown>

const fetchShim = async (url: URL, importModule: ImportModule): Promise<IvrWasmModule> => {
    try {
        return (await importModule(url.href)) as IvrWasmModule
    } catch (cause) {
        throw new IvrEmulatorError("fetch", `import of js shim failed from ${url}`, {cause})
    }
}

interface IvrWasmModule extends IvrEmulatorApi {
    default: (init: {module_or_path: Response}) => Promise<unknown>
    init: (config: {logging: string | undefined}) => void
}

const fetchAndLoad = async (
    baseUrl: string,
    importModule: ImportModule
): Promise<IvrEmulatorApi> => {
    const {jsUrl, wasmUrl} = resolveUrls(baseUrl)
    const [ivrModule, wasmResponse] = await Promise.all([
        fetchShim(jsUrl, importModule),
        fetchWasm(wasmUrl),
    ])

    try {
        // Let wasm bindgen init itself
        await ivrModule.default({module_or_path: wasmResponse})
    } catch (cause) {
        throw new IvrEmulatorError("load", `failed to load the wasm from "${wasmUrl}"`, {cause})
    }
    try {
        ivrModule.init({logging: localStorage.getItem("sq.ivr-emulator.logging") ?? undefined})
    } catch (cause) {
        throw new IvrEmulatorError("init", `init call failed`, {cause})
    }
    return ivrModule
}

let initPromise: Promise<IvrEmulatorApi> | null = null

/**
 * Fetch and start the emulator, once per page.
 *
 * Memoised because it is called from a component that mounts whenever somebody
 * opens the panel, and starting a second copy of a multi-megabyte WebAssembly
 * module is a page that pauses for no reason a reader can see. A failed attempt
 * clears the memo, so opening the panel again after a deploy retries rather than
 * repeating the first answer forever.
 */
export const loadIvrEmulator = (
    baseUrl: string,
    importModule: ImportModule = defaultImport
): Promise<IvrEmulatorApi> => {
    initPromise ??= fetchAndLoad(baseUrl, importModule).catch((e: unknown) => {
        // Allow retry
        initPromise = null

        console.error("Failed to init the emulator", e)
        throw e
    })

    return initPromise
}

/** Only for tests, which would otherwise see whatever the last one loaded. */
export const forgetIvrEmulator = (): void => {
    initPromise = null
}
