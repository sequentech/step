// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import initSqlJs, {Database} from "sql.js"
import {
    ResultsAreaContestCandidateRow,
    ResultsAreaContestRow,
    ResultsContestCandidateRow,
    ResultsContestRow,
    ResultsElectionAreaRow,
    ResultsElectionRow,
    ResultsRow,
    ResultsSqliteDataset,
} from "@/types/results"

const fetchArtifactBytes = async (url: string): Promise<Uint8Array> => {
    const response = await fetch(url, {cache: "no-store"})
    if (!response.ok) {
        throw new Error(`Unable to load results artifact: HTTP ${response.status}`)
    }

    return new Uint8Array(await response.arrayBuffer())
}

export const loadSqliteDatabase = async (url: string): Promise<Database> => {
    const sql = await initSqlJs({
        locateFile: (file) => `/${file}`,
    })
    const bytes = await fetchArtifactBytes(url)
    return new sql.Database(bytes)
}

type SqlValue = number | string | Uint8Array | null

export const queryRows = <T extends ResultsRow = ResultsRow>(
    db: Database,
    sql: string,
    params: SqlValue[] = []
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

const queryTable = <T extends ResultsRow>(db: Database, table: string): T[] => {
    try {
        return queryRows<T>(db, `SELECT * FROM ${table}`)
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
    results_election: queryTable<ResultsElectionRow>(db, "results_election"),
    results_election_area: queryTable<ResultsElectionAreaRow>(db, "results_election_area"),
    results_contest: queryTable<ResultsContestRow>(db, "results_contest"),
    results_contest_candidate: queryTable<ResultsContestCandidateRow>(
        db,
        "results_contest_candidate"
    ),
    results_area_contest: queryTable<ResultsAreaContestRow>(db, "results_area_contest"),
    results_area_contest_candidate: queryTable<ResultsAreaContestCandidateRow>(
        db,
        "results_area_contest_candidate"
    ),
})
