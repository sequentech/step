// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync, readdirSync} from "fs"
import {join, relative} from "path"
import ts from "typescript"
import * as essentials from "@sequentech/ui-essentials"

const sourceFiles = (directory: string): string[] =>
    readdirSync(directory, {withFileTypes: true}).flatMap((entry) => {
        if (entry.name === "__mocks__") return []
        const path = join(directory, entry.name)
        return entry.isDirectory()
            ? sourceFiles(path)
            : /\.tsx?$/.test(entry.name) && !/\.(test|stories)\./.test(entry.name)
              ? [path]
              : []
    })

// A value the package entry does not export is `undefined` at runtime, and a
// screen that renders it fails only when a voter reaches that screen.
const valueImports = (file: string): string[] => {
    const source = ts.createSourceFile(
        file,
        readFileSync(file, "utf8"),
        ts.ScriptTarget.Latest,
        true,
        ts.ScriptKind.TSX
    )
    return source.statements.flatMap((statement) => {
        if (
            !ts.isImportDeclaration(statement) ||
            !ts.isStringLiteral(statement.moduleSpecifier) ||
            statement.moduleSpecifier.text !== "@sequentech/ui-essentials" ||
            statement.importClause?.isTypeOnly
        ) {
            return []
        }
        const bindings = statement.importClause?.namedBindings
        if (!bindings || !ts.isNamedImports(bindings)) return []
        return bindings.elements
            .filter((element) => !element.isTypeOnly)
            .map((element) => (element.propertyName ?? element.name).text)
    })
}

const imports = sourceFiles(__dirname).flatMap((file) =>
    valueImports(file).map((name) => [relative(__dirname, file), name] as const)
)

it.each(imports)("%s imports %s from the UI Essentials entry", (_file, name) => {
    expect(essentials).toHaveProperty(name)
    expect((essentials as Record<string, unknown>)[name]).toBeDefined()
})
