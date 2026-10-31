// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

export interface HistoricalImportConverterApi {
    buildFromHistorical: (
        sql: Uint8Array,
        workbook: Uint8Array,
        decisions: unknown,
        options: unknown
    ) => unknown
}
interface ConverterModule extends HistoricalImportConverterApi {
    default: (input: {module_or_path: Response}) => Promise<unknown>
    historicalImportInfo: () => unknown
}
type ImportModule = (href: string) => Promise<unknown>

// Keep these comments in the host, as in services/IvrEmulator.ts: they must
// reach the application's bundler, rather than a precompiled shared library.
const importShim: ImportModule = (href) =>
    import(
        /* @vite-ignore */
        /* webpackIgnore: true */
        href
    )

const loads = new Map<string, Promise<HistoricalImportConverterApi>>()

/** The configured extensionless base names the same two assets as IVR. */
export const loadHistoricalImportConverter = async (
    baseUrl: string,
    importModule: ImportModule = importShim
): Promise<HistoricalImportConverterApi> => {
    if (!baseUrl.trim()) throw new Error("Historical import converter is not configured.")
    const base = new URL(
        baseUrl,
        typeof document === "undefined" ? globalThis.location.href : document.baseURI
    )
    if (!["http:", "https:"].includes(base.protocol)) {
        throw new Error("Historical import converter requires an HTTP(S) asset URL.")
    }
    base.search = ""
    base.hash = ""
    const jsUrl = new URL(base)
    jsUrl.pathname += ".js"
    const wasmUrl = new URL(base)
    wasmUrl.pathname += "_bg.wasm"
    const key = jsUrl.href
    const previous = loads.get(key)
    if (previous) return previous
    const load = (async () => {
        let loaded: unknown
        let response: Response
        try {
            ;[loaded, response] = await Promise.all([importModule(key), fetch(wasmUrl)])
        } catch {
            throw new Error(
                "Could not load historical import converter assets. Check the configured local path."
            )
        }
        if (!response.ok) {
            throw new Error(`Historical import converter WASM returned HTTP ${response.status}.`)
        }
        const module = loaded as Partial<ConverterModule> | null
        if (
            !module ||
            typeof module.default !== "function" ||
            typeof module.historicalImportInfo !== "function" ||
            typeof module.buildFromHistorical !== "function"
        ) {
            throw new Error("Historical import converter API exports are missing.")
        }
        try {
            await module.default({module_or_path: response})
        } catch {
            throw new Error("Could not initialize historical import converter WASM.")
        }
        let info: {abiVersion?: number; profile?: string; packageVersion?: string} | null
        try {
            info = module.historicalImportInfo() as typeof info
        } catch {
            throw new Error("Historical import converter ABI information could not be read.")
        }
        if (
            info?.abiVersion !== 1 ||
            info.profile !== "legacy-seat-v1" ||
            info.packageVersion !== "0.1.0"
        ) {
            throw new Error(
                "Historical import converter has an incompatible ABI, profile or package version."
            )
        }
        return {buildFromHistorical: module.buildFromHistorical}
    })().catch((error: unknown) => {
        loads.delete(key)
        throw error
    })
    loads.set(key, load)
    return load
}
