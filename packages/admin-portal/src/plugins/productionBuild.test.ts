// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Plugin screens tell a conflict from other failures with `instanceof PluginRequestError`.
// Jest compiles for the current Node; the portal's build compiles to ES5, where an Error
// subclass loses its prototype unless the constructor restores it. Compile it as built.

import {readFileSync} from "node:fs"
import {dirname, resolve} from "node:path"
import ts from "typescript"
import {PluginErrorCode} from "./contracts"

const PACKAGE = resolve(__dirname, "../..")

const loadAsBuilt = (file: string): Record<string, unknown> => {
    const {config} = ts.readConfigFile(resolve(PACKAGE, "tsconfig.json"), ts.sys.readFile)
    const {options} = ts.parseJsonConfigFileContent(config, ts.sys, PACKAGE)
    const path = resolve(__dirname, file)
    const {outputText} = ts.transpileModule(readFileSync(path, "utf8"), {
        compilerOptions: {...options, module: ts.ModuleKind.CommonJS, noEmit: false},
        fileName: path,
    })
    const load = (specifier: string): unknown =>
        require(specifier.startsWith(".") ? resolve(dirname(path), specifier) : specifier)
    const module = {exports: {} as Record<string, unknown>}
    new Function("require", "module", "exports", outputText)(load, module, module.exports)
    return module.exports
}

test("a plugin conflict keeps its class, code and current record in the production build", () => {
    const Built = loadAsBuilt("contracts.ts").PluginRequestError as new (
        message: string,
        code: PluginErrorCode,
        current?: unknown
    ) => Error & {code: PluginErrorCode; current?: unknown}
    const current = {id: "task", version: 2}
    const error = new Built("This record changed", PluginErrorCode.Conflict, current)

    expect(error).toBeInstanceOf(Built)
    expect(error).toBeInstanceOf(Error)
    expect(error.code).toBe(PluginErrorCode.Conflict)
    expect(error.current).toBe(current)
    expect(error.message).toBe("This record changed")
})
