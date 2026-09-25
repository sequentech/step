// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createServer} from "node:http"
import {readFile, realpath, stat} from "node:fs/promises"
import {extname, resolve, sep} from "node:path"
import type {AddressInfo} from "node:net"

const types: Record<string, string> = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript",
    ".css": "text/css",
    ".json": "application/json",
    ".wasm": "application/wasm",
    ".svg": "image/svg+xml",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".ico": "image/x-icon",
    ".woff": "font/woff",
    ".woff2": "font/woff2",
    ".ttf": "font/ttf",
}

const roots = new Map<string, string>()
/** The directory behind each open origin, so coverage can map script URLs back to files. */
export const servedRoots: ReadonlyMap<string, string> = roots

/** Owns its ephemeral listening socket until close; never probes then rebinds a free port. */
export async function serveDist(directory: string) {
    const root = await realpath(directory)
    await stat(resolve(root, "index.html"))
    const server = createServer(async (request, response) => {
        try {
            if (request.method !== "GET" && request.method !== "HEAD") {
                response.writeHead(405).end()
                return
            }
            const pathname = decodeURIComponent(
                new URL(request.url ?? "/", "http://localhost").pathname
            )
            let file = resolve(root, `.${pathname}`)
            if (file !== root && !file.startsWith(root + sep)) {
                response.writeHead(403).end()
                return
            }
            // Only navigations fall back to the SPA. Missing assets must remain failures.
            if (request.headers.accept?.includes("text/html")) file = resolve(root, "index.html")
            const canonical = await realpath(file)
            if (!canonical.startsWith(root + sep)) {
                response.writeHead(403).end()
                return
            }
            const content = await readFile(canonical)
            response.writeHead(200, {
                "content-type": types[extname(canonical)] ?? "application/octet-stream",
                "cache-control": "no-store",
                "cross-origin-opener-policy": "same-origin",
                "cross-origin-embedder-policy": "require-corp",
            })
            response.end(request.method === "HEAD" ? undefined : content)
        } catch {
            response.writeHead(404).end("Not found")
        }
    })
    await new Promise<void>((resolve, reject) => {
        server.once("error", reject)
        server.listen(0, "127.0.0.1", resolve)
    })
    const origin = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
    roots.set(origin, root)
    return {
        origin,
        close: async () => {
            roots.delete(origin)
            await new Promise<void>((resolve, reject) => {
                server.close((error) => (error ? reject(error) : resolve()))
                server.closeAllConnections()
            })
        },
    }
}
