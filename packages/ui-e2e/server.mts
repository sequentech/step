// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFile, writeFile, mkdir} from "node:fs/promises"
import {fileURLToPath} from "node:url"
import {resolve} from "node:path"
import {serveDist} from "@sequentech/ui-test-kit/server/static"

const packages = fileURLToPath(new URL("../", import.meta.url))
const output = process.env.STEP_UI_E2E_OUTPUT_DIR ?? "/out"
const portals = {
    voting: "voting-portal",
    admin: "admin-portal",
    results: "results-portal",
    verifier: "ballot-verifier",
} as const
const servers = await Promise.all(
    Object.entries(portals).map(async ([name, directory]) => ({
        name,
        directory: resolve(packages, directory, "dist"),
        server: await serveDist(resolve(packages, directory, "dist")),
    }))
)
const origins = Object.fromEntries(servers.map(({name, server}) => [name, server.origin]))
for (const {directory} of servers) {
    const settings = JSON.parse(await readFile(resolve(directory, "global-settings.json"), "utf8"))
    Object.assign(settings, {
        DISABLE_AUTH: false,
        DEFAULT_TENANT_ID: process.env.SUPER_ADMIN_TENANT_ID,
        KEYCLOAK_URL: "http://keycloak:8090/",
        HASURA_URL: "http://graphql-engine:8080/v1/graphql",
        PUBLIC_BUCKET_URL: "http://minio:9000/public/",
        VOTING_PORTAL_URL: origins.voting,
        BALLOT_VERIFIER_URL: `${origins.verifier}/`,
        RESULTS_PORTAL_URL: origins.results,
        QUERY_POLL_INTERVAL_MS: 600000,
    })
    await writeFile(resolve(directory, "global-settings.json"), JSON.stringify(settings, null, 2))
}
await mkdir(output, {recursive: true})
await writeFile(resolve(output, "origins.json"), JSON.stringify(origins, null, 2))
console.log("Serving production portals", origins)
for (const signal of ["SIGINT", "SIGTERM"] as const) {
    process.once(signal, async () => {
        await Promise.all(servers.map(({server}) => server.close()))
        process.exit(0)
    })
}
