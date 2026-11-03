/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {loadHistoricalImportConverter} from "./HistoricalImportConverter"

const info = {abiVersion: 1, profile: "legacy-seat-v1", packageVersion: "0.1.0"}
const plugin = () => ({
    default: jest.fn().mockResolvedValue(undefined),
    historicalImportInfo: jest.fn(() => info),
    buildFromHistorical: jest.fn(),
})
beforeEach(() => {
    global.fetch = jest.fn().mockResolvedValue({ok: true, status: 200})
})
it("resolves the configured IVR-style JS/WASM pair and initializes once", async () => {
    const module = plugin()
    const importer = jest.fn().mockResolvedValue(module)
    const first = await loadHistoricalImportConverter("/test-pair/converter?v=1#ignored", importer)
    const second = await loadHistoricalImportConverter("/test-pair/converter", importer)
    expect(first).toBe(second)
    expect(importer).toHaveBeenCalledTimes(1)
    expect(importer).toHaveBeenCalledWith("http://localhost/test-pair/converter.js")
    expect(fetch).toHaveBeenCalledWith(new URL("http://localhost/test-pair/converter_bg.wasm"))
    expect(module.default).toHaveBeenCalledWith({module_or_path: {ok: true, status: 200}})
})
it("rejects missing configuration without loading any asset", async () => {
    const importer = jest.fn()
    await expect(loadHistoricalImportConverter("", importer)).rejects.toThrow("not configured")
    expect(importer).not.toHaveBeenCalled()
    expect(fetch).not.toHaveBeenCalled()
})
it("reports missing assets and permits retry", async () => {
    const importer = jest
        .fn()
        .mockRejectedValueOnce(new Error("offline"))
        .mockResolvedValue(plugin())
    await expect(loadHistoricalImportConverter("/retry/converter", importer)).rejects.toThrow(
        "load"
    )
    await expect(loadHistoricalImportConverter("/retry/converter", importer)).resolves.toBeDefined()
})
it("refuses an unsuccessful WASM response", async () => {
    ;(fetch as jest.Mock).mockResolvedValue({ok: false, status: 404})
    await expect(
        loadHistoricalImportConverter("/missing/converter", async () => plugin())
    ).rejects.toThrow("404")
})
it.each([
    {abiVersion: 2, profile: "legacy-seat-v1", packageVersion: "0.1.0"},
    {abiVersion: 1, profile: "other", packageVersion: "0.1.0"},
    {abiVersion: 1, profile: "legacy-seat-v1", packageVersion: "9.0.0"},
])("refuses an incompatible ABI/profile/version: %j", async (unsupported) => {
    const module = {...plugin(), historicalImportInfo: () => unsupported}
    await expect(
        loadHistoricalImportConverter(
            `/unsupported/${JSON.stringify(unsupported)}`,
            async () => module
        )
    ).rejects.toThrow("incompatible")
    expect(module.buildFromHistorical).not.toHaveBeenCalled()
})
it("refuses a missing API export before initialization", async () => {
    const module = {...plugin(), buildFromHistorical: undefined}
    await expect(
        loadHistoricalImportConverter("/missing-api/converter", async () => module)
    ).rejects.toThrow("API")
    expect(module.default).not.toHaveBeenCalled()
})
it("reports initialization failure", async () => {
    const module = {...plugin(), default: jest.fn().mockRejectedValue(new Error("bad wasm"))}
    await expect(
        loadHistoricalImportConverter("/bad-init/converter", async () => module)
    ).rejects.toThrow("initialize")
})
