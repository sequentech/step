// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Node module hooks that load the workspace's TypeScript test kit with the
// checkout's own compiler, so the probe reuses its mocks without a build step.
import {existsSync} from "node:fs"
import {readFile} from "node:fs/promises"
import {createRequire} from "node:module"
import {fileURLToPath} from "node:url"

const TYPESCRIPT_SUFFIXES = [".ts", ".mts"]
let typescript

export async function initialize({packages}) {
    typescript = createRequire(`${packages}/package.json`)("typescript")
}

export async function resolve(specifier, context, nextResolve) {
    try {
        return await nextResolve(specifier, context)
    } catch (error) {
        // The kit imports sibling modules without extensions, as bundlers allow.
        if (!specifier.startsWith(".") || !context.parentURL) throw error
        for (const suffix of [...TYPESCRIPT_SUFFIXES, "/index.ts"]) {
            const url = new URL(specifier + suffix, context.parentURL)
            if (existsSync(fileURLToPath(url))) return {url: url.href, shortCircuit: true}
        }
        throw error
    }
}

export async function load(url, context, nextLoad) {
    if (!TYPESCRIPT_SUFFIXES.some((suffix) => url.endsWith(suffix))) return nextLoad(url, context)
    const fileName = fileURLToPath(url)
    const {outputText} = typescript.transpileModule(await readFile(fileName, "utf8"), {
        fileName,
        compilerOptions: {
            module: typescript.ModuleKind.ESNext,
            target: typescript.ScriptTarget.ES2022,
            esModuleInterop: true,
        },
    })
    return {format: "module", source: outputText, shortCircuit: true}
}
