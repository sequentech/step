// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {createReadStream, readFileSync} from "node:fs"
import {createRequire} from "node:module"
import {mergeConfig, type Plugin} from "vite"
import type {StorybookConfig} from "@storybook/react-vite"
import config from "../../ui-essentials/.storybook/main.ts"

const SQL_WASM = "sql-wasm.wasm"
const sqlWasmFile = createRequire(import.meta.url).resolve(`sql.js/dist/${SQL_WASM}`)

// The portal loads sql.js's WASM from the site root, where its webpack build
// copies the file; screen stories read the same path.
const sqlWasm: Plugin = {
    name: "results-sql-wasm",
    configureServer(server) {
        server.middlewares.use(`/${SQL_WASM}`, (_request, response) => {
            response.setHeader("Content-Type", "application/wasm")
            createReadStream(sqlWasmFile).pipe(response)
        })
    },
    generateBundle() {
        this.emitFile({type: "asset", fileName: SQL_WASM, source: readFileSync(sqlWasmFile)})
    },
}

export default {
    ...config,
    staticDirs: ["../public"],
    viteFinal: async (viteConfig, options) =>
        mergeConfig(await config.viteFinal!(viteConfig, options), {
            plugins: [sqlWasm],
            optimizeDeps: {include: ["sql.js"]},
        }),
} satisfies StorybookConfig
