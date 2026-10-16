// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    loadHistoricalImportConverter,
    type HistoricalImportConverterApi,
} from "./HistoricalImportConverter"
import type {ConverterWorkerRequest, ConverterWorkerReply} from "./HistoricalImportWorkerClient"

const scope = globalThis as unknown as {
    onmessage: (event: MessageEvent<ConverterWorkerRequest>) => void
    postMessage: (message: ConverterWorkerReply) => void
}
let api: HistoricalImportConverterApi | undefined
scope.onmessage = async ({data}) => {
    try {
        if (data.operation === "initialize") {
            api = await loadHistoricalImportConverter(data.baseUrl ?? "")
            scope.postMessage({id: data.id, output: null})
            return
        }
        if (!api || !data.sql || !data.workbook) {
            scope.postMessage({
                id: data.id,
                error: "Historical import converter is not initialized. Reopen the drawer to retry.",
            })
            return
        }
        const output = await api.buildFromHistorical(
            data.sql,
            data.workbook,
            data.decisions,
            data.options
        )
        scope.postMessage({id: data.id, output})
    } catch (error) {
        // Loader errors are controlled strings. Runtime exceptions may contain
        // source values and must never be copied into logs or user diagnostics.
        const message =
            data.operation === "initialize" && error instanceof Error
                ? error.message
                : "Historical import converter runtime failed. Keep the original files and check the source format before retrying. No source file was uploaded."
        scope.postMessage({id: data.id, error: message})
    }
}
