// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createRequire} from "node:module"
import initSqlJs, {type SqlJsStatic} from "sql.js"
import type {ResultsSqliteDataset} from "../../src/types/results"
import {exportDataset} from "./sqliteDataset"

const require = createRequire(import.meta.url)
let sqlite: Promise<SqlJsStatic> | undefined

/** Export a real SQLite database at test time; the browser loads it with its own WASM. */
export async function sqliteBytes(dataset: ResultsSqliteDataset): Promise<Uint8Array> {
    sqlite ??= initSqlJs({locateFile: (name) => require.resolve(`sql.js/dist/${name}`)})
    return exportDataset(await sqlite, dataset)
}
