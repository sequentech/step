// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//
// The checked-in Election Architect realm on a running development Keycloak
// with the Sequent extensions and the sequent-ui-architect theme: username-first
// email-code sign-in, admission by the election-architect `access` role and the
// Google provider button. A disposable copy of the realm is created and deleted;
// its MessageOTP runs in test mode with a per-run random code. Needs the same
// environment as real-keycloak.spec.ts (KEYCLOAK_UI_URL, KEYCLOAK_ADMIN,
// KEYCLOAK_ADMIN_PASSWORD).
import {randomInt, randomUUID} from "node:crypto"
import {readFileSync} from "node:fs"
import {join} from "node:path"
import {expect, test, type Page} from "@playwright/test"
import {scanPage} from "@sequentech/ui-test-kit/adapters/axe"

type Json = Record<string, unknown>

const KEYCLOAK = process.env.KEYCLOAK_UI_URL ?? "http://localhost:5174"
const REALM = `election-architect-ui-${randomUUID()}`
const CALLBACK = `${KEYCLOAK}/synthetic-callback`
const TEST_CLIENT = "architect-ui-test"
const TEST_MODE_CODE = String(randomInt(0, 1000000)).padStart(6, "0")
const ADMITTED = "admitted-trainee@example.test"
const NOT_ADMITTED = "unlisted-trainee@example.test"
let realmCreated = false

async function adminToken(): Promise<string> {
    const response = await fetch(`${KEYCLOAK}/realms/master/protocol/openid-connect/token`, {
        method: "POST",
        body: new URLSearchParams({
            grant_type: "password",
            client_id: "admin-cli",
            username: process.env.KEYCLOAK_ADMIN ?? "",
            password: process.env.KEYCLOAK_ADMIN_PASSWORD ?? "",
        }),
    })
    expect(response.status, "admin token").toBe(200)
    return ((await response.json()) as {access_token: string}).access_token
}

async function admin(
    token: string,
    method: string,
    path: string,
    body?: unknown
): Promise<Response> {
    const response = await fetch(`${KEYCLOAK}/admin/realms/${REALM}${path}`, {
        method,
        headers: {Authorization: `Bearer ${token}`, "Content-Type": "application/json"},
        body: body === undefined ? undefined : JSON.stringify(body),
    })
    expect(response.ok, `${method} ${path}: ${response.status}`).toBe(true)
    return response
}

function authorize(page: Page, locale?: string): Promise<unknown> {
    const parameters = new URLSearchParams({
        client_id: TEST_CLIENT,
        redirect_uri: CALLBACK,
        response_type: "code",
        scope: "openid",
        state: randomUUID(),
    })
    if (locale) parameters.set("ui_locales", locale)
    return page.goto(`${KEYCLOAK}/realms/${REALM}/protocol/openid-connect/auth?${parameters}`)
}

async function enterAddress(page: Page, email: string): Promise<void> {
    await page.getByLabel("Email").fill(email)
    await page.locator("#kc-login").click()
}

async function enterCode(page: Page): Promise<void> {
    await expect(page.getByRole("group", {name: "Verification code"})).toBeVisible()
    await page.locator("#otp-1").fill(TEST_MODE_CODE)
    await page.locator("#kc-form-submit").click()
}

test.describe.configure({mode: "serial"})

test.beforeAll(async () => {
    const token = await adminToken()
    const template = JSON.parse(
        readFileSync(
            join(process.cwd(), "../../.devcontainer/keycloak/import/election-architect.json"),
            "utf8"
        )
    ) as Json
    const created = await fetch(`${KEYCLOAK}/admin/realms`, {
        method: "POST",
        headers: {Authorization: `Bearer ${token}`, "Content-Type": "application/json"},
        body: JSON.stringify({
            ...template,
            realm: REALM,
            loginTheme: "sequent-ui-architect",
            // The Google provider only needs to be offered, never followed.
            identityProviders: (template.identityProviders as Json[]).map((provider) => ({
                ...provider,
                enabled: true,
            })),
        }),
    })
    expect(created.status, "create disposable realm").toBe(201)
    realmCreated = true
    // A public client for this test; admission is by the user's role, not the client.
    await admin(token, "POST", "/clients", {
        clientId: TEST_CLIENT,
        publicClient: true,
        standardFlowEnabled: true,
        directAccessGrantsEnabled: false,
        redirectUris: [CALLBACK],
    })
    const executions = (await (
        await admin(token, "GET", "/authentication/flows/architect%20email%20code/executions")
    ).json()) as {providerId?: string; authenticationConfig?: string}[]
    const otp = executions.find(({providerId}) => providerId === "message-otp-authenticator")
    const path = `/authentication/config/${otp?.authenticationConfig}`
    const configuration = (await (await admin(token, "GET", path)).json()) as Json
    await admin(token, "PUT", path, {
        ...configuration,
        config: {
            ...(configuration.config as Json),
            "test-mode": "true",
            "test-mode-code": TEST_MODE_CODE,
        },
    })
    for (const [email, groups] of [
        [ADMITTED, ["/architect-access"]],
        [NOT_ADMITTED, []],
    ] as const) {
        await admin(token, "POST", "/users", {
            username: email,
            email,
            emailVerified: true,
            enabled: true,
            groups,
        })
    }
})

test.afterAll(async () => {
    if (realmCreated) await admin(await adminToken(), "DELETE", "")
})

test("the address comes first, with Google offered beside it", async ({page}) => {
    await authorize(page)
    await expect(
        page.getByRole("heading", {level: 1, name: "Sign in to Election Architect"})
    ).toBeVisible()
    await expect(page.getByRole("banner")).toContainText("Election Architect")
    await expect(page.getByLabel("Email")).toHaveAttribute("autocomplete", "username")
    await expect(page.getByLabel("Password")).toHaveCount(0)
    const google = page.getByRole("link", {name: "Continue with Google"})
    await expect(google).toHaveAttribute(
        "href",
        new RegExp(`/realms/${REALM}/broker/google/login\\?`)
    )
    expect(await scanPage(page)).toEqual([])
})

test("an admitted address signs in with its email code", async ({page}) => {
    await page.route(`${CALLBACK}**`, (route) => route.fulfill({status: 200, body: "callback"}))
    await authorize(page)
    await enterAddress(page, ADMITTED)
    await expect(page.getByText("Enter the code we sent to your email.")).toBeVisible()
    expect(await scanPage(page)).toEqual([])
    await enterCode(page)
    await page.waitForURL((url) => url.href.startsWith(CALLBACK))
    const code = new URL(page.url()).searchParams.get("code") ?? ""
    const token = await fetch(`${KEYCLOAK}/realms/${REALM}/protocol/openid-connect/token`, {
        method: "POST",
        body: new URLSearchParams({
            grant_type: "authorization_code",
            client_id: TEST_CLIENT,
            code,
            redirect_uri: CALLBACK,
        }),
    })
    expect(token.status).toBe(200)
})

test("a wrong code is refused", async ({page}) => {
    await authorize(page)
    await enterAddress(page, ADMITTED)
    await expect(page.getByRole("group", {name: "Verification code"})).toBeVisible()
    const wrong = String((Number(TEST_MODE_CODE) + 1) % 1000000).padStart(6, "0")
    await page.locator("#otp-1").fill(wrong)
    await page.locator("#kc-form-submit").click()
    await expect(page.getByRole("alert")).toBeVisible()
    await expect(page).toHaveURL((url) => url.href.startsWith(`${KEYCLOAK}/realms/${REALM}/`))
})

test("an address without the access role is denied after its code", async ({page}) => {
    await authorize(page)
    await enterAddress(page, NOT_ADMITTED)
    await enterCode(page)
    await expect(page.locator("body")).toContainText(/access denied/i)
    await expect(page).toHaveURL((url) => !url.href.startsWith(CALLBACK))
})

test("an unknown address is told so before any code is sent", async ({page}) => {
    await authorize(page)
    await enterAddress(page, "nobody@example.test")
    await expect(page.getByRole("alert")).toBeVisible()
    await expect(page.getByLabel("Email")).toHaveAttribute("aria-invalid", "true")
    await expect(page.locator("#otp-1")).toHaveCount(0)
})

test("Spanish", async ({page}) => {
    await authorize(page, "es")
    await expect(
        page.getByRole("heading", {level: 1, name: "Iniciar sesión en Election Architect"})
    ).toBeVisible()
    await expect(page.getByRole("link", {name: "Continuar con Google"})).toBeVisible()
    await expect(page.locator("html")).toHaveAttribute("lang", /^es/)
})
