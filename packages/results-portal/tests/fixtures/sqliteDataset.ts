// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {SqlJsStatic} from "sql.js"
import type {ResultsSqliteDataset, ResultsRow} from "../../src/types/results"

/** Writes each dataset table into a new database and exports its bytes, in Node or a browser. */
export function exportDataset(sql: SqlJsStatic, dataset: ResultsSqliteDataset): Uint8Array {
    const db = new sql.Database()
    try {
        for (const [name, rows] of Object.entries(dataset) as [string, ResultsRow[]][]) {
            const columns = [...new Set(rows.flatMap((row) => Object.keys(row)))]
            if (columns.length === 0) columns.push("id")
            db.run(`CREATE TABLE "${name}" (${columns.map((column) => `"${column}"`).join(", ")})`)
            for (const row of rows) {
                const values = columns.map((column) => {
                    const value = (row as Record<string, unknown>)[column]
                    if (value === undefined || value === null) return null
                    if (typeof value === "number" || typeof value === "string") return value
                    if (typeof value === "boolean") return Number(value)
                    return JSON.stringify(value)
                })
                db.run(
                    `INSERT INTO "${name}" VALUES (${columns.map(() => "?").join(", ")})`,
                    values
                )
            }
        }
        return db.export()
    } finally {
        db.close()
    }
}
