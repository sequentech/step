// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createRequire} from "node:module"
import initSqlJs, {type SqlJsStatic} from "sql.js"
import type {ResultsSqliteDataset, ResultsRow} from "../../src/types/results"

const require = createRequire(import.meta.url)
let sqlite: Promise<SqlJsStatic> | undefined

/** Export a real SQLite database at test time; the browser loads it with its own WASM. */
export async function sqliteBytes(dataset: ResultsSqliteDataset): Promise<Uint8Array> {
    sqlite ??= initSqlJs({locateFile: (name) => require.resolve(`sql.js/dist/${name}`)})
    const sql = await sqlite
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
