// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// @ts-check

// Runs the Vitest browser test of one story, of one title or of one story file
// in the Storybook of the current package. Plain JavaScript, so it runs on the
// devcontainer's Node without a TypeScript loader.
import {spawn} from "node:child_process"
import {existsSync, readdirSync, statSync} from "node:fs"
import {readFile} from "node:fs/promises"
import {createRequire} from "node:module"
import {basename, dirname, join, relative, resolve, sep} from "node:path"
import {getStoryTitle, loadMainConfig, normalizeStories} from "storybook/internal/common"
import {loadCsf} from "storybook/internal/csf-tools"
import {
    parseArguments,
    selectStories,
    StorySelectionError,
    vitestArguments,
} from "./storySelection.mjs"

const USAGE = `Usage: yarn test:story <story file | story ID | title ID | Storybook URL> [--watch] [--vitest-option=value ...]

Runs the selected stories' interaction and accessibility tests in Chromium.
Coverage stays off unless --coverage is passed.`

const packageDir = process.cwd()
const configDir = resolve(packageDir, ".storybook")

/** @param {string} path */
const packagePath = (path) => relative(packageDir, path).split(sep).join("/")

/** @param {string} argument */
const quote = (argument) =>
    /^[\w./=:-]+$/.test(argument) ? argument : `'${argument.replaceAll("'", "'\\''")}'`

/** @returns {Promise<import("./storySelection.mjs").StoryEntry[]>} */
async function indexStories() {
    const {stories} = await loadMainConfig({configDir})
    const specifiers = normalizeStories(stories, {configDir, workingDir: packageDir})
    const files = new Set(
        specifiers.flatMap(({directory, importPathMatcher}) =>
            readdirSync(resolve(packageDir, directory), {recursive: true, encoding: "utf8"})
                .map((file) => join(resolve(packageDir, directory), file))
                .filter((file) => !file.includes(`${sep}node_modules${sep}`))
                .filter((file) => importPathMatcher.test(`./${packagePath(file)}`))
                .filter((file) => !file.endsWith(".mdx"))
        )
    )
    const entries = await Promise.all(
        [...files].map(async (file) => {
            const csf = loadCsf(await readFile(file, "utf8"), {
                fileName: file,
                makeTitle: (userTitle) =>
                    getStoryTitle({
                        storyFilePath: file,
                        configDir,
                        stories,
                        workingDir: packageDir,
                        userTitle,
                    }) ?? userTitle,
            }).parse()
            return csf.indexInputs.flatMap((input) =>
                input.type === "story" && input.__id
                    ? [
                          {
                              id: input.__id,
                              componentId: input.__id.split("--")[0],
                              name: input.name ?? input.exportName,
                              exportName: input.exportName,
                              file: packagePath(file),
                          },
                      ]
                    : []
            )
        })
    )
    return entries.flat()
}

/** @param {string} target */
function existingFile(target) {
    const path = resolve(packageDir, target)
    return existsSync(path) && statSync(path).isFile() ? packagePath(path) : undefined
}

async function main() {
    const options = parseArguments(process.argv.slice(2))
    if (options.help || !options.target) {
        console.log(USAGE)
        process.exitCode = options.help ? 0 : 1
        return
    }
    const selection = selectStories(
        options.target,
        await indexStories(),
        existingFile(options.target)
    )
    const args = vitestArguments(selection, options)
    const coverage = options.vitestOptions.some((option) => option.startsWith("--coverage"))
    console.log(
        `Story tests in ${basename(packageDir)} (${options.mode}, coverage ${coverage ? "on" : "off"})`
    )
    for (const file of selection.files) {
        console.log(`  ${file}`)
        for (const story of selection.stories.filter((entry) => entry.file === file)) {
            console.log(`    ${story.id}  (${story.name})`)
        }
    }
    console.log(`  vitest ${args.map(quote).join(" ")}\n`)

    const require = createRequire(join(packageDir, "package.json"))
    const vitest = join(dirname(require.resolve("vitest/package.json")), "vitest.mjs")
    const child = spawn(process.execPath, [vitest, ...args], {cwd: packageDir, stdio: "inherit"})
    child.on("exit", (code, signal) => {
        process.exitCode = code ?? (signal ? 1 : 0)
    })
}

main().catch((error) => {
    console.error(error instanceof StorySelectionError ? error.message : error)
    process.exitCode = 1
})
