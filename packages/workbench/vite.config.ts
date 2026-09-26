// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createHash} from "node:crypto"
import {readFileSync, statSync} from "node:fs"
import {createRequire} from "node:module"
import {dirname, join, resolve, sep} from "node:path"
import {fileURLToPath} from "node:url"
import {defineConfig, type Plugin} from "vite"
import react from "@vitejs/plugin-react"

const packages = fileURLToPath(new URL("..", import.meta.url))
const sourceEntry = (workspace: string) => join(packages, workspace, "src/index.tsx")

/**
 * The only place the workbench resolves sequent-core. WORKBENCH_SEQUENT_CORE may name another
 * wasm-pack `--target web` output directory, such as a development build, instead of the
 * installed package; nothing is reinstalled and the page reloads when its files change.
 */
const sequentCore = resolve(
    process.env.WORKBENCH_SEQUENT_CORE ??
        dirname(createRequire(import.meta.url).resolve("sequent-core/package.json"))
)
const sequentCoreEntry = join(
    sequentCore,
    (JSON.parse(readFileSync(join(sequentCore, "package.json"), "utf8")) as {main: string}).main
)

const SEQUENT_CORE_INFO = "virtual:workbench/sequent-core"
const RESOLVED_SEQUENT_CORE_INFO = `\0${SEQUENT_CORE_INFO}`

/** Describes the loaded sequent-core, and reloads the page when that build changes. */
function sequentCoreBuild(): Plugin {
    const wasm = join(sequentCore, "index_bg.wasm")
    return {
        name: "workbench-sequent-core",
        resolveId: (id) => (id === SEQUENT_CORE_INFO ? RESOLVED_SEQUENT_CORE_INFO : undefined),
        load(id) {
            if (id !== RESOLVED_SEQUENT_CORE_INFO) return
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
            server.watcher.on("change", (file) => {
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

export default defineConfig({
    plugins: [react(), sequentCoreBuild()],
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
})
