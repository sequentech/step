// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {fileURLToPath} from "node:url"
import {defineConfig, type Plugin} from "vite"
import react from "@vitejs/plugin-react"
import {problemTranslations} from "./src/problemTranslations"

/**
 * The second output of `yarn build`: `dist/problem-list/index.js`, see `src/problemList.ts`.
 *
 * A library build, so React, MUI, Emotion and i18next are the host's: two copies of React
 * or of an MUI theme context in one page do not work. The declarations are written by
 * `tsc -p tsconfig.problem-list.json` into `types/`, mirroring the source tree; `index.d.ts`
 * points at them.
 */
const HOST = /^(react|react-dom|react-i18next|i18next|@mui\/|@emotion\/)/

const VIRTUAL = "virtual:problem-translations"

/** `problems.*` of each `ui-core` catalogue, as the module `src/problemList.ts` imports. */
const translations = (): Plugin => ({
    name: "problem-list-translations",
    resolveId: (id) => (id === VIRTUAL ? `\0${VIRTUAL}` : undefined),
    load: (id) =>
        id === `\0${VIRTUAL}`
            ? `export default ${JSON.stringify(problemTranslations())}`
            : undefined,
})

const declarations = (): Plugin => ({
    name: "problem-list-declarations",
    generateBundle() {
        this.emitFile({
            type: "asset",
            fileName: "index.d.ts",
            source: 'export * from "./types/workbench/src/problemList"\n',
        })
    },
})

export default defineConfig({
    plugins: [react(), translations(), declarations()],
    build: {
        outDir: "dist/problem-list",
        emptyOutDir: false,
        sourcemap: true,
        lib: {
            entry: fileURLToPath(new URL("src/problemList.ts", import.meta.url)),
            formats: ["es"],
            fileName: () => "index.js",
        },
        rollupOptions: {external: (id) => HOST.test(id)},
    },
})
