// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {HistoricalImportConverterApi} from "./HistoricalImportConverter"

export class HistoricalImportRuntimeError extends Error {}

export interface ConverterWorkerReply {
    id: number
    output?: unknown
    error?: string
}
export interface ConverterWorkerRequest {
    id: number
    operation: "initialize" | "convert"
    baseUrl?: string
    sql?: Uint8Array
    workbook?: Uint8Array
    decisions?: unknown
    options?: unknown
}

/** Keep WASM loading and source parsing off the drawer's rendering thread. */
export const createHistoricalImportConverter = (
    baseUrl: string,
    makeWorker: () => Worker = () =>
        new Worker(new URL("/historical-import-worker.js", document.baseURI), {type: "module"})
): {ready: Promise<HistoricalImportConverterApi>; dispose: () => void} => {
    const absoluteBase = new URL(baseUrl, document.baseURI).href
    const worker = makeWorker()
    let sequence = 0
    let disposed = false
    const pending = new Map<
        number,
        {
            resolve: (value: unknown) => void
            reject: (error: Error) => void
            timer: ReturnType<typeof setTimeout>
        }
    >()
    const rejectAll = (message: string) => {
        pending.forEach((request) => {
            clearTimeout(request.timer)
            request.reject(new HistoricalImportRuntimeError(message))
        })
        pending.clear()
    }
    const dispose = () => {
        disposed = true
        worker.terminate()
        rejectAll("Historical import converter was closed. Reload the converter to retry.")
    }
    worker.onmessage = ({data}: MessageEvent<ConverterWorkerReply>) => {
        const request = pending.get(data.id)
        if (!request) return
        clearTimeout(request.timer)
        pending.delete(data.id)
        if (data.error) request.reject(new HistoricalImportRuntimeError(data.error))
        else request.resolve(data.output)
    }
    worker.onerror = () => {
        rejectAll(
            "Historical import converter worker failed. Reload the converter to retry; source files were not uploaded."
        )
        disposed = true
        worker.terminate()
    }
    const request = (payload: Omit<ConverterWorkerRequest, "id">, transfer: ArrayBuffer[] = []) =>
        new Promise<unknown>((resolve, reject) => {
            if (disposed) {
                reject(
                    new HistoricalImportRuntimeError(
                        "Historical import converter is unavailable. Reload the converter to retry."
                    )
                )
                return
            }
            const id = ++sequence
            const timer = setTimeout(
                () => {
                    rejectAll(
                        "Historical import converter timed out. Reload the converter and check the configured assets or source size."
                    )
                    disposed = true
                    worker.terminate()
                },
                payload.operation === "initialize" ? 30_000 : 120_000
            )
            pending.set(id, {resolve, reject, timer})
            try {
                worker.postMessage({...payload, id}, transfer)
            } catch {
                clearTimeout(timer)
                pending.delete(id)
                reject(
                    new HistoricalImportRuntimeError(
                        "Could not send local input to the converter worker. No source file was uploaded."
                    )
                )
            }
        })
    const ready = request({operation: "initialize", baseUrl: absoluteBase}).then(() => ({
        buildFromHistorical: (
            sql: Uint8Array,
            workbook: Uint8Array,
            decisions: unknown,
            options: unknown
        ) =>
            request({operation: "convert", sql, workbook, decisions, options}, [
                sql.buffer as ArrayBuffer,
                workbook.buffer as ArrayBuffer,
            ]),
    }))
    return {ready, dispose}
}
