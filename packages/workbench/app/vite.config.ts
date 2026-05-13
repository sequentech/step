// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig} from "vite"
import react from "@vitejs/plugin-react"
import wasm from "vite-plugin-wasm"
import topLevelAwait from "vite-plugin-top-level-await"
import {fileURLToPath} from "node:url"
import path from "node:path"

const here = path.dirname(fileURLToPath(import.meta.url))
const pkgs = path.resolve(here, "../..")

// Vite config for the Sequentech workbench. `vite-plugin-wasm` lets us
// import the wasm-pack-generated `velvet-wasm` package as if it were any
// other ES module; `vite-plugin-top-level-await` is required because the
// wasm init is async.
//
// `resolve.alias` for `@sequentech/ui-core` and `@sequentech/ui-essentials`
// points Vite at the TypeScript sources of those workspace packages
// rather than at their (uncompiled) `dist/` outputs. Compiling them on
// the fly avoids running their webpack/tsc build step before each dev
// session, and keeps the workbench faithful to the same source the
// production voting-portal consumes — only the *bundler* differs.
export default defineConfig({
    plugins: [react(), wasm(), topLevelAwait()],
    resolve: {
        alias: [
            // Point package-name imports at the workspace TS sources so
            // Vite compiles them on the fly (no need to run their
            // webpack/tsc build before each dev session). Keeps the
            // workbench faithful to the same source the production
            // voting-portal consumes — only the *bundler* differs.
            {
                find: "@sequentech/ui-core",
                replacement: path.join(pkgs, "ui-core/src/index.tsx"),
            },
            {
                find: "@sequentech/ui-essentials",
                replacement: path.join(pkgs, "ui-essentials/src/index.tsx"),
            },
            // ui-core's own internal imports use a `@root/*` tsconfig
            // path alias resolving to `ui-core/src/*`. Vite doesn't read
            // tsconfig paths, so reproduce it here. (ui-essentials
            // doesn't use `@root` internally — verified by grep.)
            {
                find: /^@root\/(.*)$/,
                replacement: path.join(pkgs, "ui-core/src/$1"),
            },
        ],
    },
    server: {
        port: 5173,
        strictPort: true,
    },
    optimizeDeps: {
        // velvet-wasm ships an ES module that imports a `.wasm` URL;
        // Vite's dep optimizer chokes on that unless we exclude it.
        exclude: ["velvet-wasm"],
    },
})
