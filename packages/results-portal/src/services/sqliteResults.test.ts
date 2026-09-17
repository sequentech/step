// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, beforeAll, expect, it, jest} from "@jest/globals"
import initSqlJs, {Database, SqlJsStatic} from "sql.js"
import {loadSqliteDatabase, queryRows, readResultsDataset} from "./sqliteResults"

jest.mock("sql.js", () => ({__esModule: true, default: jest.fn()}))
let sql: SqlJsStatic
beforeAll(async () => {
    // The real SQLite engine, using its bundled asm build for this Node test.
    // Only the browser's WASM-loader boundary is replaced.
    const initialize = jest.requireActual<(options?: object) => Promise<SqlJsStatic>>("sql.js/dist/sql-asm.js")
    sql = await initialize()
})
afterEach(() => {jest.restoreAllMocks(); jest.mocked(initSqlJs).mockReset()})

it("queries literal SQLite rows with bound parameters and reads available dataset tables", () => {
    const db = new sql.Database()
    const warn = jest.spyOn(console, "warn").mockImplementation(() => {})
    try {
        db.run("CREATE TABLE election (id TEXT, name TEXT); INSERT INTO election VALUES ('e1', 'First'), ('e2', 'Second')")
        expect(queryRows(db, "SELECT id, name FROM election WHERE id = ?", ["e2"])).toEqual([{id: "e2", name: "Second"}])
        expect(queryRows(db, "SELECT id FROM election WHERE id = ?", ["e3"])).toEqual([])
        const dataset = readResultsDataset(db)
        expect(dataset.election).toEqual([{id: "e1", name: "First"}, {id: "e2", name: "Second"}])
        expect(dataset.results_contest).toEqual([])
        expect(Object.keys(dataset)).toHaveLength(12)
        expect(warn).toHaveBeenCalledTimes(11)
    } finally {db.close()}
})
it("frees a prepared statement when iteration or row decoding fails", () => {
    for (const failure of ["step", "getAsObject"] as const) {
        const statement = {step: jest.fn(() => true), getAsObject: jest.fn(() => ({id: 17})), free: jest.fn()}
        statement[failure].mockImplementation(() => {throw new Error(`synthetic ${failure} failure`)})
        const db = {prepare: jest.fn(() => statement)} as unknown as Database
        expect(() => queryRows(db, "SELECT id FROM fixture")).toThrow(`synthetic ${failure} failure`)
        expect(statement.free).toHaveBeenCalledTimes(1)
    }
})
it("loads real exported SQLite bytes with no-store fetch semantics", async () => {
    const source = new sql.Database()
    source.run("CREATE TABLE election(id TEXT); INSERT INTO election VALUES ('synthetic')")
    const bytes = source.export(); source.close()
    jest.mocked(initSqlJs).mockResolvedValue(sql)
    const fetch = jest.spyOn(globalThis, "fetch").mockResolvedValue(new Response(bytes))
    const loaded = await loadSqliteDatabase("https://files.invalid/data.sqlite")
    try {expect(queryRows(loaded, "SELECT id FROM election")).toEqual([{id: "synthetic"}])} finally {loaded.close()}
    expect(fetch).toHaveBeenCalledWith("https://files.invalid/data.sqlite", {cache: "no-store"})
    const options = jest.mocked(initSqlJs).mock.calls[0][0]!
    expect(options.locateFile!("sql-wasm.wasm", "ignored")).toBe("/sql-wasm.wasm")
})
it("rejects failed downloads and reports corrupt SQLite at the query boundary", async () => {
    jest.mocked(initSqlJs).mockResolvedValue(sql)
    const fetch = jest.spyOn(globalThis, "fetch").mockResolvedValue(new Response("missing", {status: 404}))
    await expect(loadSqliteDatabase("https://files.invalid/missing")).rejects.toThrow("Unable to load results artifact: HTTP 404")
    fetch.mockResolvedValue(new Response("not a SQLite database"))
    // SQLite opens lazily: an unreadable page fails when queried, not constructed.
    const corrupt = await loadSqliteDatabase("https://files.invalid/corrupt")
    try {expect(() => queryRows(corrupt, "SELECT name FROM sqlite_master")).toThrow("file is not a database")} finally {corrupt.close()}
})
