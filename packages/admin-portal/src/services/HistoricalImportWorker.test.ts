/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
const mockLoad = jest.fn()
jest.mock("./HistoricalImportConverter", () => ({
    loadHistoricalImportConverter: (...args: unknown[]) => mockLoad(...args),
}))
interface Scope {
    onmessage: ((event: MessageEvent) => Promise<void>) | null
    postMessage: (message: unknown) => void
}
const scope = globalThis as unknown as Scope
let previous: Scope["onmessage"]
let posted: jest.SpyInstance
beforeEach(() => {
    previous = scope.onmessage
    posted = jest.spyOn(scope, "postMessage").mockImplementation(() => {})
    mockLoad.mockReset()
    jest.isolateModules(() => require("./HistoricalImportWorker"))
})
afterEach(() => {
    scope.onmessage = previous
    posted.mockRestore()
})
const send = (data: unknown) => scope.onmessage!({data} as MessageEvent)
it("does not convert data before the configured plugin has initialized", async () => {
    await send({
        id: 1,
        operation: "convert",
        sql: new Uint8Array([1]),
        workbook: new Uint8Array([2]),
    })
    expect(posted).toHaveBeenCalledWith({id: 1, error: expect.stringContaining("not initialized")})
    expect(mockLoad).not.toHaveBeenCalled()
})
it("initializes the requested asset and passes a conversion result through intact", async () => {
    const output = {summary: {sourceRows: {candidate: 1}}, output: {report: {problems: []}}}
    const build = jest.fn().mockResolvedValue(output)
    mockLoad.mockResolvedValue({buildFromHistorical: build})
    await send({
        id: 1,
        operation: "initialize",
        baseUrl: "https://assets.example.invalid/converter",
    })
    expect(mockLoad).toHaveBeenCalledWith("https://assets.example.invalid/converter")
    expect(posted).toHaveBeenLastCalledWith({id: 1, output: null})
    const sql = new Uint8Array([1]),
        workbook = new Uint8Array([2])
    await send({
        id: 2,
        operation: "convert",
        sql,
        workbook,
        decisions: {census: "configuration-only"},
        options: {},
    })
    expect(build).toHaveBeenCalledWith(sql, workbook, {census: "configuration-only"}, {})
    expect(posted).toHaveBeenLastCalledWith({id: 2, output})
    await send({id: 3, operation: "convert", sql})
    expect(posted).toHaveBeenLastCalledWith({
        id: 3,
        error: expect.stringContaining("not initialized"),
    })
})
it("keeps controlled module-load errors separate from conversion failures", async () => {
    mockLoad.mockRejectedValue(new Error("Incompatible converter ABI"))
    await send({id: 1, operation: "initialize"})
    expect(mockLoad).toHaveBeenCalledWith("")
    expect(posted).toHaveBeenCalledWith({id: 1, error: "Incompatible converter ABI"})
})
it("never returns source values from unexpected runtime exceptions", async () => {
    mockLoad.mockResolvedValue({
        buildFromHistorical: () => {
            throw new Error("synthetic-private-voter-value")
        },
    })
    await send({id: 1, operation: "initialize", baseUrl: "/local/converter"})
    await send({
        id: 2,
        operation: "convert",
        sql: new Uint8Array([1]),
        workbook: new Uint8Array([2]),
    })
    expect(posted).toHaveBeenLastCalledWith({
        id: 2,
        error: expect.stringContaining("runtime failed"),
    })
    expect(JSON.stringify(posted.mock.calls)).not.toContain("synthetic-private-voter-value")
})
