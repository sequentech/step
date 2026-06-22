// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import initSqlJs, {Database} from "sql.js"
import {openDB} from "idb"
import {ResultsRow, ResultsSqliteDataset} from "@/types/results"

const IDB_NAME = "results-portal-artifacts"
const IDB_STORE_NAME = "sqlite-cache"

const initCache = () =>
    openDB(IDB_NAME, 1, {
        upgrade(db) {
            db.createObjectStore(IDB_STORE_NAME)
        },
    })

const fetchArtifactBytes = async (url: string): Promise<Uint8Array> => {
    const cache = await initCache()
    const cached = await cache.get(IDB_STORE_NAME, url)
    if (cached) {
        return cached as Uint8Array
    }

    const response = await fetch(url)
    if (!response.ok) {
        throw new Error(`Unable to load results artifact: HTTP ${response.status}`)
    }

    const bytes = new Uint8Array(await response.arrayBuffer())
    await cache.put(IDB_STORE_NAME, bytes, url)
    return bytes
}

export const loadSqliteDatabase = async (url: string): Promise<Database> => {
    const sql = await initSqlJs({
        locateFile: (file) => `/${file}`,
    })
    const bytes = await fetchArtifactBytes(url)
    return new sql.Database(bytes)
}

export const queryRows = <T extends ResultsRow = ResultsRow>(
    db: Database,
    sql: string,
    params: any[] = []
): T[] => {
    const statement = db.prepare(sql, params)
    const rows: T[] = []

    try {
        while (statement.step()) {
            rows.push(statement.getAsObject() as T)
        }
    } finally {
        statement.free()
    }

    return rows
}

const queryTable = (db: Database, table: string): ResultsRow[] => {
    try {
        return queryRows(db, `SELECT * FROM ${table}`)
    } catch (error) {
        console.warn(`Results SQLite table ${table} is not available`, error)
        return []
    }
}

export const readResultsDataset = (db: Database): ResultsSqliteDataset => ({
    election_event: queryTable(db, "election_event"),
    election: queryTable(db, "election"),
    contest: queryTable(db, "contest"),
    candidate: queryTable(db, "candidate"),
    area: queryTable(db, "area"),
    results_event: queryTable(db, "results_event"),
    results_election: queryTable(db, "results_election"),
    results_election_area: queryTable(db, "results_election_area"),
    results_contest: queryTable(db, "results_contest"),
    results_contest_candidate: queryTable(db, "results_contest_candidate"),
    results_area_contest: queryTable(db, "results_area_contest"),
    results_area_contest_candidate: queryTable(db, "results_area_contest_candidate"),
})
