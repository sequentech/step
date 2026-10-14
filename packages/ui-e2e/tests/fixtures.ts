// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test as base, expect, type BrowserContext, type Page, type Response} from "@playwright/test"
import {copyFile, mkdtemp, rm} from "node:fs/promises"
import {readFileSync} from "node:fs"
import {resolve} from "node:path"
import {tmpdir} from "node:os"
import {execFile} from "node:child_process"
import {promisify} from "node:util"

export const output = process.env.STEP_UI_E2E_OUTPUT_DIR ?? "/out"
export interface Fixture {
    tenantId: string
    eventId: string
    eventName: string
    elections: Record<string, string>
    areas: Record<string, string>
    contests: Record<string, string>
    candidates: Record<string, Record<string, string>>
    voters: Record<string, string[]>
    voterPassword: string
    adminOtp: string
    origins: Record<"voting" | "admin" | "results" | "verifier", string>
}
export const fixture: Fixture = JSON.parse(readFileSync(resolve(output, "fixture.json"), "utf8"))
export const eventPath = `/tenant/${fixture.tenantId}/event/${fixture.eventId}`
const allowed = new Set([
    ...Object.values(fixture.origins),
    "http://keycloak:8090",
    "http://graphql-engine:8080",
    "http://minio:9000",
])

export const telemetryUrl = `https://react-admin-telemetry.marmelab.com/react-admin-telemetry?domain=${new URL(fixture.origins.admin).hostname}`
interface TelemetryFixture {
    enabled: boolean
    requests: string[]
}
export async function contain(
    context: BrowserContext,
    violations: string[],
    telemetry?: TelemetryFixture
) {
    await context.route("**/*", async (route) => {
        const request = route.request()
        const url = new URL(request.url())
        // React-admin emits one optional production telemetry image. Only the
        // admin journey opts into this exact local response; no external I/O.
        if (
            telemetry?.enabled &&
            request.method() === "GET" &&
            request.resourceType() === "image" &&
            url.href === telemetryUrl
        ) {
            telemetry.requests.push(url.href)
            await route.fulfill({status: 204, body: ""})
            return
        }
        if (allowed.has(url.origin)) await route.continue()
        else {
            violations.push(`${route.request().method()} ${url.href}`)
            await route.abort("blockedbyclient")
        }
    })
    await context.routeWebSocket(/.*/, (socket) => {
        violations.push(`WEBSOCKET ${socket.url()}`)
        socket.close()
    })
}
export const test = base.extend<{violations: string[]; telemetry: TelemetryFixture}>({
    telemetry: async ({browserName}, use) => {
        const telemetry = {enabled: false, requests: [] as string[]}
        await use(telemetry)
        expect(telemetry.requests).toEqual(telemetry.enabled ? [telemetryUrl] : [])
    },
    violations: async ({browserName}, use) => {
        const violations: string[] = []
        await use(violations)
        expect(violations, "Unexpected browser network traffic").toEqual([])
    },
    context: async ({context, violations, telemetry}, use) => {
        await contain(context, violations, telemetry)
        await use(context)
    },
})
export {expect}
test.afterEach(async ({page}, info) => {
    if (info.status !== info.expectedStatus) {
        await info.attach("page-text", {
            body: `${page.url()}\n${await page
                .locator("body")
                .innerText()
                .catch(() => "Page unavailable")}`,
            contentType: "text/plain",
        })
    }
})

export async function login(page: Page, username: string, portal = "voting") {
    await page.goto(
        `${fixture.origins[portal as keyof Fixture["origins"]]}${eventPath}/login?lang=en`
    )
    await page.locator('input[name="username"]').fill(username)
    await page.locator('input[name="password"]').fill(fixture.voterPassword)
    await page.locator("#kc-login").click()
}
export async function review(page: Page, username: string, candidate: string) {
    await login(page, username)
    const election = page.locator(".election-item").filter({hasText: "E2E main election"})
    await election.getByRole("button", {name: /click to vote/i}).click()
    const declaration = page.locator('.security-confirmation-checkbox input[type="checkbox"]')
    await expect(declaration).toBeVisible()
    await declaration.check()
    await page.getByRole("button", {name: "Start Voting", exact: true}).click()
    await page.getByRole("checkbox", {name: new RegExp(candidate)}).check()
    await expect(page.getByRole("checkbox", {name: new RegExp(candidate)})).toBeChecked()
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page.getByRole("heading", {name: /^Review your ballot/})).toBeVisible()
    await expect(page.getByText(candidate, {exact: true})).toBeVisible()
}
export function operation(response: Response, name: string) {
    if (response.request().method() !== "POST" || !response.url().includes("/v1/graphql"))
        return false
    return response.request().postDataJSON()?.operationName === name
}
export async function cast(page: Page) {
    const pending = page.waitForResponse((response) => operation(response, "InsertCastVote"))
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    const confirm = page.getByRole("button", {name: /Yes, I want to cast my vote/i})
    await expect(confirm).toBeVisible()
    await confirm.click()
    return await pending
}
export async function db<T>(query: string, variables: Record<string, unknown> = {}): Promise<T> {
    const response = await fetch("http://graphql-engine:8080/v1/graphql", {
        method: "POST",
        headers: {
            "content-type": "application/json",
            "x-hasura-admin-secret": process.env.HASURA_ADMIN_SECRET!,
        },
        body: JSON.stringify({query, variables}),
    })
    expect(response.status).toBe(200)
    const body = await response.json()
    expect(body.errors).toBeUndefined()
    return body.data as T
}
const run = promisify(execFile)
export async function step(...args: string[]) {
    const directory = await mkdtemp(resolve(tmpdir(), "step-ui-cli-"))
    const binary = resolve(directory, "step-cli")
    await copyFile("/opt/step-e2e/bin/step-cli", binary)
    const options = {
        env: {...process.env, HOME: directory, NO_COLOR: "1"},
        timeout: 900000,
        maxBuffer: 4 * 1024 * 1024,
    }
    try {
        await run(
            binary,
            [
                "step",
                "config",
                "--tenant-id",
                fixture.tenantId,
                "--endpoint-url",
                "http://graphql-engine:8080/v1/graphql",
                "--keycloak-url",
                "http://keycloak:8090",
                "--keycloak-user",
                process.env.ADMIN_USERNAME!,
                "--keycloak-password",
                process.env.ADMIN_PASSWORD!,
                "--keycloak-client-id",
                "api-key-client",
                "--keycloak-client-secret",
                process.env.API_KEY_CLIENT_SECRET!,
            ],
            options
        )
        const {stdout, stderr} = await run(binary, ["step", ...args], options)
        expect(stdout + stderr).not.toMatch(/^Error!/m)
        return stdout
    } finally {
        await rm(directory, {recursive: true, force: true})
    }
}
