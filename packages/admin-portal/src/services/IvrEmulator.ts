import loadIvrWasm, {IvrEmulatorDriver} from "./generated/ivr_emulator_wasm"

const api = {IvrEmulatorDriver}
export type IvrEmulatorApi = typeof api
export type {IvrEmulatorDriver, Action, PromptInfo} from "./generated/ivr_emulator_wasm"
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

export const loadIvrEmulator = (wasmUrl: string): Promise<IvrEmulatorApi> | null => {
    if (!initPromise) {
        initPromise ??= fetchWasm(wasmUrl)
            .then(async (moduleResp) => {
                try {
                    await loadIvrWasm({module_or_path: moduleResp})
                } catch (cause) {
                    throw new IvrEmulatorError("load", "failed to load the wasm", {cause})
                }

                return api
            })
            .catch((e: any) => {
                console.error("Failed to init the emulator", e)
                throw e
            })
    }

    return initPromise
}
