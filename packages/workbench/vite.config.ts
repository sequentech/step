// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createHash} from "node:crypto"
import {existsSync, readFileSync, statSync} from "node:fs"
import {createRequire} from "node:module"
import {dirname, join, resolve, sep} from "node:path"
import {fileURLToPath} from "node:url"
import {defineConfig, type Plugin} from "vite"
import react from "@vitejs/plugin-react"
import {sequentCoreViteAlias} from "../ui-core/sequent-core-dev.cjs"

const packages = fileURLToPath(new URL("..", import.meta.url))
const sourceEntry = (workspace: string) => join(packages, workspace, "src/index.tsx")

const SEQUENT_CORE_INFO = "virtual:workbench/sequent-core"
const RESOLVED_SEQUENT_CORE_INFO = `\0${SEQUENT_CORE_INFO}`

/** Describes the loaded sequent-core, and reloads the page when that build changes. */
function sequentCoreBuild(sequentCore: string): Plugin {
    return {
        name: "workbench-sequent-core",
        resolveId: (id) => (id === SEQUENT_CORE_INFO ? RESOLVED_SEQUENT_CORE_INFO : undefined),
        load(id) {
            if (id !== RESOLVED_SEQUENT_CORE_INFO) return
            const statusFile = join(sequentCore, "status.json")
            const build: unknown = existsSync(statusFile)
                ? JSON.parse(readFileSync(statusFile, "utf8")).published?.build
                : undefined
            const wasm =
                typeof build === "string" && /^[a-f0-9]{16}$/.test(build)
                    ? join(sequentCore, "builds", build, "index_bg.wasm")
                    : join(sequentCore, "index_bg.wasm")
            const bytes = readFileSync(wasm)
            return `export default ${JSON.stringify({
                directory: sequentCore,
                wasmBytes: bytes.length,
                wasmSha256: createHash("sha256").update(bytes).digest("hex"),
                modifiedAt: statSync(wasm).mtime.toISOString(),
            })}`
        },
        configureServer(server) {
            server.watcher.add(sequentCore)
            server.watcher.on("all", (_event, file) => {
                if (!file.startsWith(`${sequentCore}${sep}`)) return
                for (const module of server.moduleGraph.getModulesByFile(file) ?? [])
                    server.moduleGraph.invalidateModule(module)
                const info = server.moduleGraph.getModuleById(RESOLVED_SEQUENT_CORE_INFO)
                if (info) server.moduleGraph.invalidateModule(info)
                // The WASM binary is fetched at runtime, outside the module graph.
                server.ws.send({type: "full-reload"})
            })
        },
    }
}

const port = Number(process.env.WORKBENCH_PORT ?? 5173)

export default defineConfig(({command}) => {
    // Explicit overrides support comparison builds; otherwise only development
    // loads the package published by step-dev wasm.
    const developmentEntry =
        command === "serve" ? sequentCoreViteAlias()[0]?.replacement : undefined
    const sequentCore = resolve(
        process.env.WORKBENCH_SEQUENT_CORE ??
            (developmentEntry
                ? dirname(developmentEntry)
                : dirname(createRequire(import.meta.url).resolve("sequent-core/package.json")))
    )
    const sequentCoreEntry = join(
        sequentCore,
        (JSON.parse(readFileSync(join(sequentCore, "package.json"), "utf8")) as {main: string}).main
    )

    return {
        plugins: [react(), sequentCoreBuild(sequentCore)],
        envPrefix: "WORKBENCH_",
        resolve: {
            dedupe: ["react", "react-dom"],
            // Exact matches: shared UI packages compile from source, like in Storybook.
            alias: [
                {find: /^@sequentech\/ui-core$/, replacement: sourceEntry("ui-core")},
                {find: /^@sequentech\/ui-essentials$/, replacement: sourceEntry("ui-essentials")},
                {find: /^sequent-core$/, replacement: sequentCoreEntry},
            ],
        },
        optimizeDeps: {
            // sequent-core fetches its .wasm relative to its own module URL.
            exclude: ["sequent-core"],
            // Screens reach these through routes the dependency scan cannot follow.
            include: ["keycloak-js", "@apollo/client/link/context", "web-vitals"],
        },
        server: {
            host: "127.0.0.1",
            port,
            strictPort: true,
            fs: {allow: [packages, sequentCore]},
            watch: {ignored: ["!**/node_modules/sequent-core/**"]},
        },
        preview: {host: "127.0.0.1", port, strictPort: true},
        build: {sourcemap: true, chunkSizeWarningLimit: 8000},
    }
})
