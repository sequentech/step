/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {createHistoricalImportConverter} from "./HistoricalImportWorkerClient"
const fakeWorker = () => ({
    onmessage: undefined as unknown as Worker["onmessage"],
    onerror: undefined as unknown as Worker["onerror"],
    postMessage: jest.fn(),
    terminate: jest.fn(),
})
it("waits for initialization and transfers local byte buffers to an asynchronous worker", async () => {
    const worker = fakeWorker()
    const client = createHistoricalImportConverter(
        "/wasm/converter",
        () => worker as unknown as Worker
    )
    expect(worker.postMessage.mock.calls[0][0]).toMatchObject({
        operation: "initialize",
        baseUrl: "http://localhost/wasm/converter",
    })
    worker.onmessage?.call(
        worker as unknown as Worker,
        {data: {id: 1, output: null}} as MessageEvent
    )
    const api = await client.ready
    const sql = new Uint8Array([1]),
        workbook = new Uint8Array([2])
    const converted = api.buildFromHistorical(
        sql,
        workbook,
        {census: "configuration-only"},
        {}
    ) as Promise<unknown>
    expect(worker.postMessage.mock.calls[1][1]).toEqual([sql.buffer, workbook.buffer])
    worker.onmessage?.call(
        worker as unknown as Worker,
        {data: {id: 2, output: {archive: "synthetic"}}} as MessageEvent
    )
    await expect(converted).resolves.toEqual({archive: "synthetic"})
    client.dispose()
    expect(worker.terminate).toHaveBeenCalled()
})
it("reports worker startup failure and stops the worker", async () => {
    const worker = fakeWorker()
    const client = createHistoricalImportConverter(
        "/wasm/converter",
        () => worker as unknown as Worker
    )
    worker.onerror?.call(worker as unknown as Worker, {} as ErrorEvent)
    await expect(client.ready).rejects.toThrow("worker failed")
    expect(worker.terminate).toHaveBeenCalled()
})
it("rejects ABI/load errors before exposing conversion", async () => {
    const worker = fakeWorker()
    const client = createHistoricalImportConverter(
        "/wasm/converter",
        () => worker as unknown as Worker
    )
    worker.onmessage?.call(
        worker as unknown as Worker,
        {data: {id: 1, error: "Incompatible ABI"}} as MessageEvent
    )
    await expect(client.ready).rejects.toThrow("Incompatible ABI")
    client.dispose()
})
