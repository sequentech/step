// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The signing errors are told apart with `instanceof`. Jest compiles for the current Node, where a
// subclass of Error keeps its prototype; the portal's build (ts-loader with tsconfig.json) compiles
// to ES5, where `super(message)` returns a plain Error unless the constructor restores the prototype.
// So these tests compile the real modules with the production compiler options before checking.

import {readFileSync} from "node:fs"
import {dirname, resolve} from "node:path"
import ts from "typescript"
import {CertificateFileErrorCode} from "./errors"
import {PayloadProblem} from "./request"
import {SigningApiErrorKind} from "./api"

const PACKAGE = resolve(__dirname, "../../..")

const productionOptions = (): ts.CompilerOptions => {
    const configFile = resolve(PACKAGE, "tsconfig.json")
    const {config, error} = ts.readConfigFile(configFile, ts.sys.readFile)
    if (error) throw new Error(ts.flattenDiagnosticMessageText(error.messageText, "\n"))
    const {options} = ts.parseJsonConfigFileContent(config, ts.sys, PACKAGE)
    // Only the module format changes, so the emitted code can run here; the target stays.
    return {...options, module: ts.ModuleKind.CommonJS, noEmit: false}
}

/** The module's exports, compiled as the portal's build compiles it; imports load as usual. */
const loadAsBuilt = (file: string): Record<string, unknown> => {
    const path = resolve(__dirname, file)
    const {outputText} = ts.transpileModule(readFileSync(path, "utf8"), {
        compilerOptions: productionOptions(),
        fileName: path,
    })
    const load = (specifier: string): unknown =>
        require(specifier.startsWith(".") ? resolve(dirname(path), specifier) : specifier)
    const module = {exports: {} as Record<string, unknown>}
    new Function("require", "module", "exports", outputText)(load, module, module.exports)
    return module.exports
}

type ErrorClass = new (...args: never[]) => Error

describe("signing errors in the production build", () => {
    it("compiles to ES5, the target that loses an Error subclass's prototype", () => {
        expect(productionOptions().target).toBe(ts.ScriptTarget.ES5)
    })

    it.each([
        ["api.ts", "SigningApiError", [SigningApiErrorKind.Forbidden, "refused", {status: 403}]],
        [
            "errors.ts",
            "CertificateFileError",
            [CertificateFileErrorCode.WrongPassword, "bad password"],
        ],
        ["request.ts", "PayloadMismatchError", [PayloadProblem.Digest, "digest differs"]],
    ] as const)("%s: %s keeps its class", (file, name, args) => {
        const ErrorType = loadAsBuilt(file)[name] as ErrorClass
        const error = new ErrorType(...(args as unknown as never[]))

        expect(Object.getPrototypeOf(error)).toBe(ErrorType.prototype)
        expect(error).toBeInstanceOf(ErrorType)
        expect(error).toBeInstanceOf(Error)
        expect(error.name).toBe(name)
        expect(error.message).toBe(args[1])
    })
})
