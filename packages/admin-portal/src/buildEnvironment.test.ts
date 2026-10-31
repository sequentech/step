// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {mkdtemp, readFile, rm, writeFile} from "node:fs/promises"
import {tmpdir} from "node:os"
import path from "node:path"
import {runInNewContext} from "node:vm"
import webpack, {Configuration} from "webpack"

const portalConfig = require("../webpack.config.cjs") as (
    env: Record<string, string>,
    argv: {mode: "development" | "production"}
) => Configuration

const fixture = `globalThis.output = {
    publicUrl: process.env.PUBLIC_URL,
    lines: process.env.MAX_DIFF_LINES,
    countdown: process.env.SECONDS_TO_SHOW_COUNTDOWN,
    alert: process.env.SECONDS_TO_SHOW_ALERT,
    mode: process.env.NODE_ENV,
    environment: typeof process === "undefined" ? undefined : process.env
};`

async function compileEnvironment(
    directory: string,
    environment: Record<string, string | undefined>,
    mode: "development" | "production" = "production"
) {
    const saved = process.env
    let plugins: Configuration["plugins"]
    try {
        process.env = environment
        plugins = portalConfig({}, {mode}).plugins?.filter(
            (plugin) => plugin instanceof webpack.DefinePlugin
        )
    } finally {
        process.env = saved
    }
    await writeFile(path.join(directory, "entry.js"), fixture)
    await new Promise<void>((resolve, reject) => {
        const compiler = webpack({
            mode,
            context: directory,
            entry: "./entry.js",
            output: {path: directory, filename: "output.js"},
            plugins,
            devtool: false,
            optimization: {minimize: false},
        })
        if (!compiler) {
            reject(new Error("webpack did not create a compiler"))
            return
        }
        compiler.run((error, stats) => {
            compiler.close((closeError) => {
                if (error || closeError || !stats || stats.hasErrors()) {
                    reject(error ?? closeError ?? new Error(stats?.toString({errors: true})))
                } else {
                    resolve()
                }
            })
        })
    })
    const bundle = await readFile(path.join(directory, "output.js"), "utf8")
    const browser: {output?: Record<string, unknown>} = {}
    runInNewContext(bundle, browser)
    return {bundle, output: browser.output}
}

let directory: string
beforeEach(async () => {
    directory = await mkdtemp(path.join(tmpdir(), "admin-build-environment-"))
})
afterEach(async () => {
    await rm(directory, {recursive: true, force: true})
})

test("unrelated build variables never reach the browser artifact", async () => {
    const result = await compileEnvironment(directory, {
        PRIVATE_BUILD_TOKEN: "synthetic-private-build-token",
        REACT_APP_PRIVATE_TOKEN: "synthetic-prefixed-private-token",
    })
    expect(result.bundle).not.toContain("synthetic-private-build-token")
    expect(result.bundle).not.toContain("synthetic-prefixed-private-token")
    expect(result.output).toEqual({
        publicUrl: "",
        lines: undefined,
        countdown: undefined,
        alert: undefined,
        mode: "production",
        environment: undefined,
    })
})

test("supported public settings retain their literal values", async () => {
    const result = await compileEnvironment(directory, {
        PUBLIC_URL: '/portal/"quoted"',
        MAX_DIFF_LINES: "100",
        SECONDS_TO_SHOW_COUNTDOWN: "30",
        SECONDS_TO_SHOW_ALERT: "90",
    })
    expect(result.output).toEqual({
        publicUrl: '/portal/"quoted"',
        lines: "100",
        countdown: "30",
        alert: "90",
        mode: "production",
        environment: undefined,
    })
})

test("changing unrelated environment values leaves identical output", async () => {
    const first = await compileEnvironment(directory, {PRIVATE_BUILD_TOKEN: "first"})
    const second = await compileEnvironment(directory, {PRIVATE_BUILD_TOKEN: "second"})
    expect(first.bundle).toBe(second.bundle)
})

test.each(["development", "production"] as const)(
    "%s builds use webpack mode even with a conflicting build environment",
    async (mode) => {
        const result = await compileEnvironment(
            directory,
            {NODE_ENV: mode === "production" ? "development" : "production"},
            mode
        )
        expect(result.output?.mode).toBe(mode)
    }
)
