// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {DataProvider, GetListParams, RaRecord, SortPayload} from "react-admin"
import {dataBoundary} from "./dataBoundary"
import {pending} from "../../../ui-essentials/.storybook/screens"

/** What the story's reads do: answer from the records, never settle, or fail. */
export type ReadState = "records" | "loading" | "error"

export interface ResourceBoundaryOptions {
    /** Reads of every resource, or of the resources listed; default "records". */
    reads?: ReadState | Partial<Record<string, ReadState>>
    /** A write rejects with this message instead of changing the records. */
    writeError?: string
    /** Message of a failing read. */
    readError?: string
}

export interface RecordedWrite {
    method: "create" | "update" | "updateMany" | "delete" | "deleteMany"
    resource: string
    params: Record<string, unknown>
}

const ILIKE = /@_i?like$/

/** Hasura-style list filters: plain values compare equal, arrays contain, `@_ilike` matches. */
function matches(record: RaRecord, filter: Record<string, unknown> = {}) {
    return Object.entries(filter).every(([key, expected]) => {
        if (expected === undefined || key === "q") return true
        if (ILIKE.test(key)) {
            const value = record[key.replace(ILIKE, "")]
            const pattern = String(expected).replaceAll("%", "").toLowerCase()
            return value == null || String(value).toLowerCase().includes(pattern)
        }
        if (key.includes("@") || !(key in record)) return true
        const value = record[key]
        if (Array.isArray(expected)) return expected.includes(value)
        if (expected !== null && typeof expected === "object") return true
        return value === expected
    })
}

function sorted<T extends RaRecord>(records: T[], sort?: SortPayload) {
    if (!sort?.field) return records
    const direction = sort.order === "DESC" ? -1 : 1
    return [...records].sort((left, right) => {
        const a = left[sort.field]
        const b = right[sort.field]
        if (a === b) return 0
        if (a == null) return -direction
        if (b == null) return direction
        return String(a).localeCompare(String(b), "en", {numeric: true}) * direction
    })
}

/**
 * A react-admin data provider over synthetic records, one list per resource.
 * Reads filter, sort and paginate them; writes change them for the rest of the
 * story and are recorded in `writes`. A resource without an entry is an
 * unexpected request, like every method `dataBoundary` does not handle.
 */
export function resourceBoundary(
    initial: Record<string, RaRecord[]>,
    options: ResourceBoundaryOptions = {}
) {
    const records: Record<string, RaRecord[]> = Object.fromEntries(
        Object.entries(initial).map(([resource, rows]) => [resource, rows.map((row) => ({...row}))])
    )
    const writes: RecordedWrite[] = []
    let boundary: ReturnType<typeof dataBoundary>
    const readState = (resource: string): ReadState =>
        typeof options.reads === "string" ? options.reads : (options.reads?.[resource] ?? "records")
    const rows = (resource: string) => {
        const list = records[resource]
        if (!list) {
            boundary.unexpected.push(`resource ${resource}`)
            throw new Error(`Unexpected resource ${resource}`)
        }
        return list
    }
    const read = async <T>(resource: string, answer: () => T): Promise<T> => {
        const state = readState(resource)
        if (state === "loading") return pending<T>()
        if (state === "error") throw new Error(options.readError ?? "Synthetic service unavailable")
        return answer()
    }
    const write = async <T>(
        method: RecordedWrite["method"],
        resource: string,
        params: object,
        apply: () => T
    ): Promise<T> => {
        writes.push({method, resource, params: {...params}})
        if (options.writeError) throw new Error(options.writeError)
        return apply()
    }
    const list = (resource: string, params: Partial<GetListParams>) => {
        const filtered = sorted(
            rows(resource).filter((record) => matches(record, params.filter)),
            params.sort
        )
        const {page = 1, perPage = filtered.length || 1} = params.pagination ?? {}
        return {
            data: filtered.slice((page - 1) * perPage, page * perPage),
            total: filtered.length,
        }
    }
    const handlers: Partial<DataProvider> = {
        getList: (resource, params) => read(resource, () => list(resource, params) as never),
        getOne: (resource, {id}) =>
            read(resource, () => {
                const record = rows(resource).find((row) => row.id === id)
                if (!record) {
                    boundary.unexpected.push(`${resource}/${String(id)}`)
                    throw new Error(`No synthetic ${resource} ${String(id)}`)
                }
                return {data: record} as never
            }),
        getMany: (resource, {ids}) =>
            read(
                resource,
                () => ({data: rows(resource).filter((row) => ids.includes(row.id))}) as never
            ),
        getManyReference: (resource, params) =>
            read(
                resource,
                () =>
                    list(resource, {
                        ...params,
                        filter: {...params.filter, [params.target]: params.id},
                    }) as never
            ),
        create: (resource, params) =>
            write("create", resource, params, () => {
                const record = {id: `created-${writes.length}`, ...params.data} as RaRecord
                rows(resource).push(record)
                return {data: record} as never
            }),
        update: (resource, params) =>
            write("update", resource, params, () => {
                const list = rows(resource)
                const index = list.findIndex((row) => row.id === params.id)
                const record = {...list[index], ...params.data, id: params.id} as RaRecord
                if (index >= 0) list[index] = record
                return {data: record} as never
            }),
        updateMany: (resource, params) =>
            write("updateMany", resource, params, () => {
                for (const row of rows(resource)) {
                    if (params.ids.includes(row.id)) Object.assign(row, params.data)
                }
                return {data: params.ids} as never
            }),
        delete: (resource, params) =>
            write("delete", resource, params, () => {
                const list = rows(resource)
                const record = list.find((row) => row.id === params.id)
                records[resource] = list.filter((row) => row.id !== params.id)
                return {data: record ?? {id: params.id}} as never
            }),
        deleteMany: (resource, params) =>
            write("deleteMany", resource, params, () => {
                records[resource] = rows(resource).filter((row) => !params.ids.includes(row.id))
                return {data: params.ids} as never
            }),
    }
    boundary = dataBoundary(handlers)
    return {...boundary, records, writes}
}
