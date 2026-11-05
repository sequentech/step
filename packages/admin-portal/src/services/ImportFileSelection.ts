// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

type Selection =
    | {kind: "native"; file: File}
    | {kind: "historical"; files: File[]}
    | {kind: "error"; message: string}

/** Extensions route local inputs; the converter validates the schema and workbook contents. */
export function selectImportFiles(files: File[], previous: File[], enabled: boolean): Selection {
    const sql = files.filter((file) => /\.sql$/i.test(file.name))
    const xlsx = files.filter((file) => /\.xlsx$/i.test(file.name))
    if (sql.length || xlsx.length) {
        if (!enabled)
            return {
                kind: "error",
                message:
                    "Historical import converter is not configured. Ask your administrator to enable it, or drop a native ZIP.",
            }
        if (sql.length + xlsx.length !== files.length)
            return {
                kind: "error",
                message:
                    "Drop a native ZIP or the SQL and XLSX sources separately; do not mix formats.",
            }
        if (sql.length > 1 || xlsx.length > 1)
            return {
                kind: "error",
                message:
                    "Drop exactly one SQL dump and one XLSX workbook. Duplicate source types were not accepted.",
            }
        const dump = sql[0] ?? previous.find((file) => /\.sql$/i.test(file.name))
        const workbook = xlsx[0] ?? previous.find((file) => /\.xlsx$/i.test(file.name))
        return {kind: "historical", files: [dump, workbook].filter((file): file is File => !!file)}
    }
    if (files.length === 1 && /\.(zip|ezip|json)$/i.test(files[0].name))
        return {kind: "native", file: files[0]}
    return {
        kind: "error",
        message:
            "Drop one native ZIP, or one supported SQL dump and its supplemental XLSX workbook.",
    }
}
