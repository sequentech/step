// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {existsSync, readFileSync, writeFileSync} from "node:fs"
import {fileURLToPath} from "node:url"
import {execFile} from "node:child_process"
import {promisify} from "node:util"
import {defineConfig} from "vite"
import react from "@vitejs/plugin-react"
import {keycloakify} from "keycloakify/vite-plugin"

import themes from "./themes.json"

const ROOT = fileURLToPath(new URL("../../", import.meta.url))
const THEME_SOURCE = `${ROOT}packages/keycloak-extensions/sequent-theme/src/main/resources/theme`
const run = promisify(execFile)
// Keycloakify only generates the page templates it finds as `"<id>.ftl":` keys
// without a dot before .ftl, so the authenticator's dotted template ids are
// derived from the generated login.ftl here.
const DOTTED_PAGE_IDS = ["message-otp.login.ftl"]

export default defineConfig({
    server: {
        host: "127.0.0.1",
        strictPort: true,
        proxy: process.env.KEYCLOAK_UI_UPSTREAM
            ? Object.fromEntries(
                  ["/realms", "/resources", "/admin", "/robots.txt"].map((path) => [
                      path,
                      {target: process.env.KEYCLOAK_UI_UPSTREAM, changeOrigin: false},
                  ])
              )
            : undefined,
    },
    plugins: [
        react(),
        {
            name: "sequent-keycloak-theme-reload",
            configureServer(server) {
                server.watcher.add([THEME_SOURCE, `${ROOT}packages/keycloak-ui/context.ftl`])
            },
            async handleHotUpdate({file, server}) {
                if (
                    !file.startsWith(THEME_SOURCE) &&
                    file !== `${ROOT}packages/keycloak-ui/context.ftl`
                )
                    return
                if (!existsSync(`${ROOT}.cache/keycloak-ui/themes`)) return
                await run(
                    "python3",
                    ["-m", "scripts.dev.keycloak", "prepare", "--skip-build", "--runtime", "hot"],
                    {cwd: ROOT}
                )
                server.ws.send({type: "full-reload"})
                return []
            },
        },
        keycloakify({
            themeName: Object.keys(themes),
            accountThemeImplementation: "none",
            keycloakVersionTargets: {
                "22-to-25": false,
                "all-other-versions": "sequent-ui.jar",
            },
            postBuild: async () => {
                for (const theme of Object.keys(themes)) {
                    const directory = `theme/${theme}/login`
                    const login = readFileSync(`${directory}/login.ftl`, "utf8")
                    for (const pageId of DOTTED_PAGE_IDS) {
                        writeFileSync(
                            `${directory}/${pageId}`,
                            login
                                .replace('"pageId": "login.ftl"', `"pageId": "${pageId}"`)
                                .replace(
                                    '"ftlTemplateFileName": "login.ftl"',
                                    `"ftlTemplateFileName": "${pageId}"`
                                )
                        )
                    }
                }
            },
        }),
    ],
    resolve: {
        alias: {
            "@sequentech/ui-essentials/theme": fileURLToPath(
                new URL("../ui-essentials/src/services/theme.ts", import.meta.url)
            ),
        },
        dedupe: ["react", "react-dom", "@emotion/react", "@mui/material"],
    },
})
