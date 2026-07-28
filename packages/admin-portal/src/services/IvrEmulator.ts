import type {WasmConfig} from "./generated/ivr_emulator_wasm"
type IvrWasmModule = typeof import("./generated/ivr_emulator_wasm")

export type IvrEmulatorApi = Pick<IvrWasmModule, "IvrEmulatorDriver">

export type {
    IvrEmulatorDriver,
    Action,
    PromptInfo,
    EmulatorConfig,
} from "./generated/ivr_emulator_wasm"
let initPromise: Promise<IvrEmulatorApi> | null = null

export class IvrEmulatorError extends Error {
    constructor(
        readonly operation: "fetch" | "load" | "init",
        readonly msg: string,
        options?: ErrorOptions
    ) {
        super(`Ivr emulator "${operation}" failed: ${msg}`, options)
    }
}

const resolveUrls = (baseUrl: string) => {
    const base = new URL(baseUrl, document.baseURI)
    base.search = ""
    base.hash = ""

    const jsUrl = new URL(base)
    jsUrl.pathname += ".js"

    const wasmUrl = new URL(base)
    wasmUrl.pathname += "_bg.wasm"

    return {jsUrl, wasmUrl}
}

const fetchWasm = async (url: string | URL): Promise<Response> => {
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

const fetchShim = async (url: URL): Promise<IvrWasmModule> => {
    try {
        return (await import(
            /* @vite-ignore */
            /* webpackIgnore: true */
            url.href
        )) as IvrWasmModule
    } catch (cause) {
        throw new IvrEmulatorError("load", `import of js shim failed from ${url}`, {cause})
    }
}

const fetchAndLoad = async (baseUrl: string): Promise<IvrEmulatorApi> => {
    const {jsUrl, wasmUrl} = resolveUrls(baseUrl)
    const [ivrModule, wasmResponse] = await Promise.all([fetchShim(jsUrl), fetchWasm(wasmUrl)])

    try {
        // Let wasm bindgen init itself
        await ivrModule.default({module_or_path: wasmResponse})
    } catch (cause) {
        throw new IvrEmulatorError("load", `failed to load the wasm from "${wasmUrl}"`, {cause})
    }
    try {
        let config: WasmConfig = {
            logging: localStorage.getItem("sq.ivr-emulator.logging") ?? undefined,
        }
        ivrModule.init(config)
    } catch (cause) {
        throw new IvrEmulatorError("init", `init call failed`, {cause})
    }
    return ivrModule
}

export const loadIvrEmulator = (baseUrl: string): Promise<IvrEmulatorApi> | null => {
    if (!initPromise) {
        initPromise ??= fetchAndLoad(baseUrl).catch((e: any) => {
            // Allow retry
            initPromise = null

            console.error("Failed to init the emulator", e)
            throw e
        })
    }

    return initPromise
}
