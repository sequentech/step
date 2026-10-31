// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
/** @jest-environment jsdom */
import {selectImportFiles} from "./ImportFileSelection"
const sql = new File(["sql"], "source.SQL")
const xlsx = new File(["xlsx"], "supplement.xlsx")
const zip = new File(["zip"], "event.zip")
it.each([
    [sql, xlsx],
    [xlsx, sql],
])("recognizes either source ordering", (...files) => {
    expect(selectImportFiles(files, [], true)).toEqual({kind: "historical", files: [sql, xlsx]})
})
it.each([
    [sql, xlsx],
    [xlsx, sql],
])("accumulates separate drops in either order", (first, second) => {
    expect(selectImportFiles([second], [first], true)).toEqual({
        kind: "historical",
        files: [sql, xlsx],
    })
})
it("replaces a source on repeat drop without accumulating duplicates", () => {
    const replacement = new File(["changed"], "replacement.sql")
    expect(selectImportFiles([replacement], [sql, xlsx], true)).toEqual({
        kind: "historical",
        files: [replacement, xlsx],
    })
})
it.each([[sql, sql], [xlsx, xlsx], [sql, zip], [zip, zip], [new File([], "notes.txt")]])(
    "rejects invalid selections instead of uploading the first file",
    (...files) => {
        expect(selectImportFiles(files, [], true).kind).toBe("error")
    }
)
it("retains an incomplete source without treating it as a complete election", () => {
    expect(selectImportFiles([sql], [], true)).toEqual({kind: "historical", files: [sql]})
})
it("keeps native ZIP routing independent of previous sources", () => {
    expect(selectImportFiles([zip], [sql, xlsx], true)).toEqual({kind: "native", file: zip})
})
it("blocks sources when the opt-in plugin is disabled", () => {
    expect(selectImportFiles([sql, xlsx], [], false).kind).toBe("error")
    expect(selectImportFiles([zip], [], false)).toEqual({kind: "native", file: zip})
})
