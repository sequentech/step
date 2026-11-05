// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {readFile} from "node:fs/promises"
import {fileURLToPath} from "node:url"
import {extname, resolve} from "node:path"
import react from "@vitejs/plugin-react"
import {defineConfig, normalizePath, type Plugin} from "vite"
import {sequentCoreViteAlias} from "../ui-core/sequent-core-dev.cjs"

const directory = fileURLToPath(new URL(".", import.meta.url))
const packages = resolve(directory, "..")
const template = resolve(directory, "public/index.html")
const headers = {
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
    "Cross-Origin-Resource-Policy": "cross-origin",
}
const contexts = [
    "react",
    "react-dom",
    "react-router",
    "react-router-dom",
    "react-i18next",
    "i18next",
    "@emotion/react",
    "@emotion/styled",
    "@mui/material",
    "@mui/system",
    "@mui/x-data-grid",
    "@apollo/client",
    "sequent-core",
]

function portalHtml(): Plugin {
    return {
        name: "verifier-html",
        transformIndexHtml: {
            order: "pre",
            handler: (html) => ({
                html: html.replaceAll("%PUBLIC_URL%", ""),
                tags: [
                    {
                        tag: "script",
                        attrs: {type: "module", src: "/src/index.tsx"},
                        injectTo: "body",
                    },
                ],
            }),
        },
        configureServer(server) {
            // Reuse the webpack template for every SPA navigation, including deep links.
            server.middlewares.use(async (request, response, next) => {
                if (request.method !== "GET" && request.method !== "HEAD") return next()
                const pathname = new URL(request.url ?? "/", "http://localhost").pathname
                if (
                    pathname !== "/" &&
                    pathname !== "/index.html" &&
                    (!request.headers.accept?.includes("text/html") || extname(pathname))
                )
                    return next()
                try {
                    const html = await server.transformIndexHtml(
                        request.url ?? "/",
                        await readFile(template, "utf8")
                    )
                    response.setHeader("Content-Type", "text/html")
                    for (const [name, value] of Object.entries(headers))
                        response.setHeader(name, value)
                    response.end(html)
                } catch (error) {
                    next(error)
                }
            })
        },
        generateBundle: {
            order: "post",
            handler(_options, bundle) {
                const html = bundle["public/index.html"]
                if (!html) throw new Error("Vite did not generate the verifier HTML entry")
                delete bundle["public/index.html"]
                html.fileName = "index.html"
                bundle["index.html"] = html
            },
        },
    }
}

export default defineConfig(({command, isPreview}) => {
    const development = command === "serve" && !isPreview
    const port = Number(process.env.PORT ?? 3001)
    return {
        plugins: [
            // Bootstrap owns createRoot. When its imports change it must reload,
            // rather than accept React Refresh and create a second root.
            react({exclude: [normalizePath(resolve(directory, "src/index.tsx"))]}),
            portalHtml(),
        ],
        base: "/",
        cacheDir: process.env.VITE_CACHE_DIR ?? "node_modules/.vite-verifier",
        resolve: {
            dedupe: contexts,
            alias: [
                {find: /^@root\//, replacement: `${resolve(directory, "src")}/`},
                {find: /^@\//, replacement: `${resolve(directory, "src")}/`},
                // Rollup consumes source directly; rebundling the webpack libraries
                // changes the default-import semantics of their Emotion externals.
                {
                    find: /^@sequentech\/ui-core$/,
                    replacement: resolve(packages, "ui-core/src/index.tsx"),
                },
                {
                    find: /^@sequentech\/ui-essentials$/,
                    replacement: resolve(packages, "ui-essentials/src/index.tsx"),
                },
                ...(development ? sequentCoreViteAlias() : []),
            ],
        },
        optimizeDeps: {
            entries: ["src/index.tsx"],
            exclude: ["sequent-core"],
            include: ["keycloak-js", "@apollo/client/link/context", "web-vitals"],
        },
        server: {
            host: "127.0.0.1",
            port,
            strictPort: true,
            headers,
            fs: {allow: [packages]},
        },
        preview: {host: "127.0.0.1", port, strictPort: true, headers},
        build: {
            outDir: "dist-vite",
            sourcemap: true,
            rollupOptions: {input: template},
        },
    }
})
