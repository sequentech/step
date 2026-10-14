// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createServer} from "node:http"
import {readFile, realpath, stat} from "node:fs/promises"
import {extname, relative, resolve, sep} from "node:path"
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
export interface StaticServerWorker {
    parallelIndex: number
    config: {workers: number}
}

/** Divide configured ports between Playwright parallel slots without a fixed block size. */
export function createPortAllocator(base: number, limit = 65535) {
    if (!Number.isInteger(base) || base < 0 || base > 65535)
        throw new Error("STEP_UI_TEST_PORT_BASE must be an integer from 0 to 65535")
    if (!Number.isInteger(limit) || limit < 1 || limit > 65535 || limit < base)
        throw new Error(
            "STEP_UI_TEST_PORT_LIMIT must be an integer from 1 to 65535 and at least the base"
        )
    let next = 0
    return (worker?: StaticServerWorker) => {
        if (base === 0) return 0
        const index = worker?.parallelIndex ?? 0
        const workers = worker?.config.workers ?? 1
        if (
            !Number.isInteger(workers) ||
            workers < 1 ||
            !Number.isInteger(index) ||
            index < 0 ||
            index >= workers
        )
            throw new Error(
                "Static server requires a valid Playwright parallelIndex and worker count"
            )
        const port = base + index + next * workers
        if (port > limit)
            throw new Error(
                `Static server port range ${base}-${limit} is exhausted for worker ${index}`
            )
        next += 1
        return port
    }
}

const portBase = Number(process.env.STEP_UI_TEST_PORT_BASE ?? 0)
const allocatePort = createPortAllocator(
    portBase,
    Number(process.env.STEP_UI_TEST_PORT_LIMIT ?? 65535)
)
/** The directory behind each open origin, so coverage can map script URLs back to files. */
export const servedRoots: ReadonlyMap<string, string> = roots

/** Owns its listening socket until close; never probes then rebinds a free port. */
export async function serveDist(
    directory: string,
    overrides: ReadonlyMap<string, string> = new Map(),
    worker?: StaticServerWorker
) {
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
            if (
                request.headers["sec-fetch-mode"] === "navigate" &&
                request.headers.accept?.includes("text/html")
            )
                file = resolve(root, "index.html")
            const canonical = await realpath(file)
            if (!canonical.startsWith(root + sep)) {
                response.writeHead(403).end()
                return
            }
            // Overrides belong to this server and only replace existing files
            // after the same navigation and canonical-path checks as disk reads.
            const content = overrides.get(relative(root, canonical)) ?? (await readFile(canonical))
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
        if (portBase !== 0 && process.env.TEST_PARALLEL_INDEX !== undefined && !worker)
            throw new Error("Configured Playwright static servers require workerInfo")
        const port = allocatePort(worker)
        server.listen(port, "127.0.0.1", resolve)
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
