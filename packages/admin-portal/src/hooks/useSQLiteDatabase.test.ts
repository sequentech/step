/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, renderHook, waitFor} from "@testing-library/react"
import {
    DatabaseContext,
    useDatabaseContext,
    useDatabaseManager,
    useManagedDatabase,
} from "./useSQLiteDatabase"

const mockGet = jest.fn()
const mockPut = jest.fn()
const mockInitSql = jest.fn()
jest.mock("idb", () => ({openDB: async () => ({get: mockGet, put: mockPut})}))
jest.mock("sql.js", () => ({
    __esModule: true,
    default: (...args: unknown[]) => mockInitSql(...args),
}))
jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useQuery: (_query: unknown, options: any) => ({
        loading: false,
        data: options.skip ? undefined : {fetchDocument: {url: options.variables.documentId}},
    }),
}))

function deferred<T>() {
    let resolve!: (value: T) => void
    const promise = new Promise<T>((done) => {
        resolve = done
    })
    return {promise, resolve}
}

function Wrapper({children}: {children: React.ReactNode}) {
    const {contextValue} = useDatabaseManager()
    return React.createElement(DatabaseContext.Provider, {value: contextValue}, children)
}

const sql = {
    Database: class {
        bytes: Uint8Array
        constructor(bytes: Uint8Array) {
            this.bytes = bytes
        }
    },
}
const response = (value: number) => ({
    ok: true,
    arrayBuffer: async () => new Uint8Array([value]).buffer,
})

function setup() {
    return renderHook(
        ({documentId}) => {
            const state = useManagedDatabase(documentId, "event")
            return {...state, databases: useDatabaseContext().databases}
        },
        {initialProps: {documentId: "A"}, wrapper: Wrapper}
    )
}

beforeEach(() => {
    jest.clearAllMocks()
    mockGet.mockResolvedValue(undefined)
    mockPut.mockResolvedValue(undefined)
    mockInitSql.mockResolvedValue(sql)
})

test("a late download from A cannot register A bytes as document B", async () => {
    const a = deferred<any>()
    const b = deferred<any>()
    global.fetch = jest.fn((url) => (url === "A" ? a.promise : b.promise)) as any
    const {result, rerender} = setup()
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("A"))
    rerender({documentId: "B"})
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("B"))
    await act(async () => {
        a.resolve(response(11))
    })
    expect(result.current.databases.has("B")).toBe(false)
    await act(async () => {
        b.resolve(response(22))
    })
    await waitFor(() => expect(result.current.isReady).toBe(true))
    expect((result.current.databases.get("B") as any).bytes).toEqual(new Uint8Array([22]))
})

test("switching during SQL initialization cannot reuse A bytes for B", async () => {
    const init = deferred<typeof sql>()
    const b = deferred<any>()
    mockInitSql.mockReturnValue(init.promise)
    global.fetch = jest.fn((url) =>
        url === "A" ? Promise.resolve(response(11)) : b.promise
    ) as any
    const {result, rerender} = setup()
    await waitFor(() => expect(mockInitSql).toHaveBeenCalled())
    rerender({documentId: "B"})
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("B"))
    await act(async () => {
        init.resolve(sql)
    })
    expect(result.current.databases.has("B")).toBe(false)
    await act(async () => {
        b.resolve(response(22))
    })
    await waitFor(() => expect(result.current.isReady).toBe(true))
    expect((result.current.databases.get("B") as any).bytes).toEqual(new Uint8Array([22]))
})

test("a late IndexedDB read cannot register A bytes as document B", async () => {
    const a = deferred<Uint8Array>()
    const b = deferred<Uint8Array>()
    mockGet.mockImplementation((_store, id) => (id === "A" ? a.promise : b.promise))
    global.fetch = jest.fn()
    const {result, rerender} = setup()
    await waitFor(() => expect(mockGet).toHaveBeenCalledWith("database-cache", "A"))
    rerender({documentId: "B"})
    await waitFor(() => expect(mockGet).toHaveBeenCalledWith("database-cache", "B"))
    await act(async () => {
        a.resolve(new Uint8Array([11]))
    })
    expect(result.current.databases.has("B")).toBe(false)
    await act(async () => {
        b.resolve(new Uint8Array([22]))
    })
    await waitFor(() => expect(result.current.isReady).toBe(true))
    expect((result.current.databases.get("B") as any).bytes).toEqual(new Uint8Array([22]))
    expect(fetch).not.toHaveBeenCalled()
})

test("B remains unchanged when its download finishes before A", async () => {
    const a = deferred<any>()
    const b = deferred<any>()
    global.fetch = jest.fn((url) => (url === "A" ? a.promise : b.promise)) as any
    const {result, rerender} = setup()
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("A"))
    rerender({documentId: "B"})
    await waitFor(() => expect(fetch).toHaveBeenCalledWith("B"))
    await act(async () => {
        b.resolve(response(22))
    })
    await waitFor(() => expect(result.current.isReady).toBe(true))
    const database = result.current.databases.get("B")
    await act(async () => {
        a.resolve(response(11))
    })
    expect(result.current.databases.get("B")).toBe(database)
    expect((database as any).bytes).toEqual(new Uint8Array([22]))
})
