// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//
// The React theme against a running development Keycloak with the Sequent
// extensions: the tenant realm's browser flow (password, then
// MessageOTPAuthenticator by email through the dummy sender), compared with the
// FreeMarker theme on the same realm. Needs:
//   KEYCLOAK_UI_URL   Keycloak as the browser reaches it
//   KEYCLOAK_UI_LOG   a file following the Keycloak container log (codes)
//   KEYCLOAK_UI_EVIDENCE_FILE  where the observations are written as JSON
//   KEYCLOAK_ADMIN, KEYCLOAK_ADMIN_PASSWORD  the development admin
import {randomBytes, randomUUID} from "node:crypto"
import {readFileSync, statSync, writeFileSync} from "node:fs"
import {join} from "node:path"
import {loadavg} from "node:os"
import {expect, test, type Page} from "@playwright/test"
import {scanPage} from "@sequentech/ui-test-kit/adapters/axe"
import {
    KEYCLOAK_SYNTHETIC_USER,
    KEYCLOAK_PROFILE_ATTRIBUTES,
} from "@sequentech/ui-test-kit/fixtures/keycloak"

const KEYCLOAK = process.env.KEYCLOAK_UI_URL ?? "http://localhost:5174"
const REALM = `keycloak-ui-${randomUUID()}`
const KEYCLOAK_LOG = process.env.KEYCLOAK_UI_LOG ?? ""
const EVIDENCE_FILE = process.env.KEYCLOAK_UI_EVIDENCE_FILE ?? ""
const CALLBACK = `${KEYCLOAK}/synthetic-callback`
const USERNAME = KEYCLOAK_SYNTHETIC_USER.username
const EMAIL = KEYCLOAK_SYNTHETIC_USER.email
// Generated per run: no credential is stored anywhere.
const PASSWORD = randomBytes(18).toString("base64url")
const KEYCLOAK_UI_THEME = "sequent-ui-admin"
const OVERRIDE_TEXT = "Synthetic realm override"

enum Theme {
    React = "react",
    Ftl = "ftl",
}

const CLIENT_IDS: Record<Theme, string> = {
    [Theme.React]: "ui-portal",
    [Theme.Ftl]: "ftl-portal",
}

type Json = Record<string, unknown>
const evidence: Record<string, unknown> = {keycloak: KEYCLOAK, realm: REALM}

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
    body?: unknown,
    contentType = "application/json"
): Promise<Response> {
    const response = await fetch(`${KEYCLOAK}/admin/realms/${REALM}${path}`, {
        method,
        headers: {Authorization: `Bearer ${token}`, "Content-Type": contentType},
        body:
            body === undefined
                ? undefined
                : contentType === "application/json"
                  ? JSON.stringify(body)
                  : String(body),
    })
    expect(response.ok, `${method} ${path}: ${response.status}`).toBe(true)
    return response
}

async function ensureClient(token: string, theme: Theme): Promise<void> {
    const clientId = CLIENT_IDS[theme]
    const existing = (await (
        await admin(token, "GET", `/clients?clientId=${clientId}`)
    ).json()) as Json[]
    const representation = {
        clientId,
        publicClient: true,
        standardFlowEnabled: true,
        directAccessGrantsEnabled: false,
        redirectUris: [CALLBACK],
        attributes: {
            login_theme: theme === Theme.React ? KEYCLOAK_UI_THEME : "sequent.admin-portal",
        },
    }
    if (existing.length === 0) {
        await admin(token, "POST", "/clients", representation)
    } else {
        await admin(token, "PUT", `/clients/${String(existing[0].id)}`, {
            ...existing[0],
            ...representation,
        })
    }
}

async function ensureUser(token: string): Promise<void> {
    const found = (await (
        await admin(token, "GET", `/users?username=${USERNAME}&exact=true`)
    ).json()) as Json[]
    if (found.length === 0) {
        await admin(token, "POST", "/users", {
            username: USERNAME,
            email: EMAIL,
            emailVerified: true,
            enabled: true,
            firstName: "Synthetic",
            lastName: "Voter",
        })
    }
    const [user] = (await (
        await admin(token, "GET", `/users?username=${USERNAME}&exact=true`)
    ).json()) as Json[]
    await admin(token, "PUT", `/users/${String(user.id)}/reset-password`, {
        type: "password",
        value: PASSWORD,
        temporary: false,
    })
}

function authorizeUrl(theme: Theme, state: string, locale?: string): string {
    const parameters = new URLSearchParams({
        client_id: CLIENT_IDS[theme],
        redirect_uri: CALLBACK,
        response_type: "code",
        scope: "openid",
        state,
        nonce: randomUUID(),
    })
    if (locale !== undefined) {
        parameters.set("ui_locales", locale)
    }
    return `${KEYCLOAK}/realms/${REALM}/protocol/openid-connect/auth?${parameters}`
}

function logSize(): number {
    return statSync(KEYCLOAK_LOG).size
}

// The dummy email sender logs every message; the code is in its text body.
async function sentCode(offset: number): Promise<string> {
    const deadline = Date.now() + 30000
    while (Date.now() < deadline) {
        const log = readFileSync(KEYCLOAK_LOG, "utf8").slice(offset)
        const sent = log.lastIndexOf(`address=${EMAIL}`)
        if (sent >= 0) {
            const code = /\b(\d{6})\b/.exec(log.slice(sent, sent + 600))
            if (code !== null) {
                return code[1]
            }
        }
        await new Promise((resolve) => setTimeout(resolve, 250))
    }
    throw new Error(`no code sent to ${EMAIL}`)
}

async function passwordStep(
    page: Page,
    theme: Theme,
    locale?: string
): Promise<{state: string; offset: number}> {
    const state = randomUUID()
    await page.goto(authorizeUrl(theme, state, locale))
    await page.locator("#username").fill(USERNAME)
    await page.locator("#password").fill(PASSWORD)
    const offset = logSize()
    await page.locator("#kc-login").click()
    await expect(page.locator("#otp-1")).toBeVisible()
    return {state, offset}
}

async function enterCode(page: Page, code: string): Promise<void> {
    for (const [index, digit] of [...code].entries()) {
        await page.locator(`#otp-${index + 1}`).fill(digit)
    }
    await page.locator("#kc-form-submit").click()
}

// Keys two levels deep, to compare what reaches the browser without values.
function shape(value: Json): Record<string, string[] | string> {
    return Object.fromEntries(
        Object.entries(value).map(([key, entry]) => [
            key,
            entry !== null && typeof entry === "object" && !Array.isArray(entry)
                ? Object.keys(entry as Json).sort()
                : typeof entry,
        ])
    )
}

async function kcContext(page: Page): Promise<Json> {
    return page.evaluate(() => (window as unknown as {kcContext: Json}).kcContext)
}

test.describe.configure({mode: "serial"})

function copyForImport(value: Json): Json {
    const identifiers = new Map<string, string>()
    const visit = (entry: unknown) => {
        if (Array.isArray(entry)) entry.forEach(visit)
        else if (entry !== null && typeof entry === "object") {
            for (const [key, child] of Object.entries(entry)) {
                if (key === "id" && typeof child === "string") identifiers.set(child, randomUUID())
                visit(child)
            }
        }
    }
    visit(value)
    const replace = (entry: unknown): unknown => {
        if (typeof entry === "string") return identifiers.get(entry) ?? entry
        if (Array.isArray(entry)) return entry.map(replace)
        if (entry !== null && typeof entry === "object")
            return Object.fromEntries(
                Object.entries(entry).map(([key, child]) => [key, replace(child)])
            )
        return entry
    }
    return replace(value) as Json
}

let realmCreated = false

test.beforeAll(async () => {
    expect(KEYCLOAK_LOG, "KEYCLOAK_UI_LOG").not.toBe("")
    const token = await adminToken()
    const template = JSON.parse(
        readFileSync(
            join(
                process.cwd(),
                "../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
            ),
            "utf8"
        )
    ) as Json
    const created = await fetch(`${KEYCLOAK}/admin/realms`, {
        method: "POST",
        headers: {Authorization: `Bearer ${token}`, "Content-Type": "application/json"},
        body: JSON.stringify({...copyForImport(template), realm: REALM, users: []}),
    })
    expect(created.status, "create disposable realm").toBe(201)
    realmCreated = true
    const realm = (await (await admin(token, "GET", "")).json()) as Json
    const locales = new Set([...((realm.supportedLocales as string[]) ?? []), "en", "es"])
    await admin(token, "PUT", "", {
        ...realm,
        internationalizationEnabled: true,
        supportedLocales: [...locales],
    })
    await ensureClient(token, Theme.React)
    await ensureClient(token, Theme.Ftl)
    await ensureUser(token)
})

test.afterAll(async () => {
    if (realmCreated) await admin(await adminToken(), "DELETE", "")
    if (EVIDENCE_FILE !== "") {
        writeFileSync(EVIDENCE_FILE, JSON.stringify(evidence, null, 2) + "\n")
    }
})

for (const theme of [Theme.React, Theme.Ftl]) {
    test(`${theme}: password and message OTP end in a code the client can redeem`, async ({
        page,
    }) => {
        await page.route(`${CALLBACK}**`, (route) => route.fulfill({status: 200, body: "callback"}))
        const {state, offset} = await passwordStep(page, theme)
        await expect(page.locator("#kc-page-title")).toContainText(
            "OTP (One Time Password) was sent to"
        )
        const otpAxe = await scanPage(page)
        const instruction = (await page.locator(".kc-message-otl-instructions").innerText()).trim()
        expect(instruction).toBe("Enter the code we sent to your email.")
        if (theme === Theme.React) expect(otpAxe).toEqual([])
        const context = theme === Theme.React ? await kcContext(page) : undefined
        const html = await page.content()
        const code = await sentCode(offset)
        await enterCode(page, code)
        await page.waitForURL((url) => url.href.startsWith(CALLBACK))
        const callback = new URL(page.url())
        expect(callback.searchParams.get("state")).toBe(state)
        expect(callback.searchParams.get("iss")).toBe(`${KEYCLOAK}/realms/${REALM}`)
        const tokenResponse = await fetch(
            `${KEYCLOAK}/realms/${REALM}/protocol/openid-connect/token`,
            {
                method: "POST",
                body: new URLSearchParams({
                    grant_type: "authorization_code",
                    client_id: CLIENT_IDS[theme],
                    code: callback.searchParams.get("code") ?? "",
                    redirect_uri: CALLBACK,
                }),
            }
        )
        expect(tokenResponse.status).toBe(200)
        const {id_token: idToken} = (await tokenResponse.json()) as {id_token: string}
        const claims = JSON.parse(
            Buffer.from(idToken.split(".")[1], "base64url").toString()
        ) as Json
        expect(claims.preferred_username).toBe(USERNAME)
        expect(claims.email).toBe(EMAIL)
        // Neither theme may hand the browser the password or the expected code.
        expect(html).not.toContain(PASSWORD)
        expect(html).not.toContain(code)
        evidence[`${theme}.login`] = {
            ok: true,
            callbackParameters: [...callback.searchParams.keys()].sort(),
            otpAxeViolations: otpAxe,
            otpInstruction: instruction,
            otpHtmlBytes: html.length,
        }
        if (context !== undefined) {
            expect(context.courier).toBe("EMAIL")
            const serialized = JSON.stringify(context)
            expect(serialized).not.toContain(PASSWORD)
            expect(serialized).not.toContain(code)
            evidence[`${theme}.otpContext`] = {
                keys: Object.keys(context).sort(),
                authenticatorAttributes: Object.fromEntries(
                    [
                        "address",
                        "courier",
                        "isOtl",
                        "codeJustSent",
                        "resendTimer",
                        "ttl",
                        "codeLength",
                    ].map((key) => [key, context[key] ?? null])
                ),
                realmKeys: Object.keys((context.realm as Json) ?? {}).sort(),
                realmAttributesPresent:
                    (context.realm as Json | undefined)?.attributes !== undefined,
                bytes: serialized.length,
            }
        }
    })

    test(`${theme}: a wrong code keeps the OTP page with the authenticator's error`, async ({
        page,
    }) => {
        const {offset} = await passwordStep(page, theme)
        const code = await sentCode(offset)
        await enterCode(page, code === "000000" ? "111111" : "000000")
        await expect(page.getByText("Invalid code entered, please enter it again.")).toBeVisible()
        await expect(page.locator("#otp-1")).toBeVisible()
        evidence[`${theme}.wrongCode`] = {ok: true}
    })

    test(`${theme}: Spanish through ui_locales`, async ({page}) => {
        await page.goto(authorizeUrl(theme, randomUUID(), "es"))
        await expect(page.locator("#kc-page-title")).toContainText(
            "Iniciar sesión en el Portal de Administración"
        )
        const loginAxe = await scanPage(page)
        if (theme === Theme.React) expect(loginAxe).toEqual([])
        const loginContext = theme === Theme.React ? await kcContext(page) : undefined
        await page.locator("#username").fill(USERNAME)
        await page.locator("#password").fill(PASSWORD)
        await page.locator("#kc-login").click()
        await expect(page.locator("#kc-page-title")).toContainText(
            "Su OTP (Código de Autenticación) fue enviado a"
        )
        evidence[`${theme}.spanish`] = {
            ok: true,
            loginAxeViolations: loginAxe,
            loginContextKeys: loginContext === undefined ? null : Object.keys(loginContext).sort(),
            loginContextShape: loginContext === undefined ? null : shape(loginContext),
            loginRealmAttributesPresent:
                loginContext === undefined
                    ? null
                    : (loginContext.realm as Json | undefined)?.attributes !== undefined,
        }
    })
}

test("realm localization overrides reach each theme", async ({page}) => {
    const token = await adminToken()
    await admin(token, "PUT", "/localization/en/loginAccountTitle", OVERRIDE_TEXT, "text/plain")
    try {
        const titles: Record<string, string> = {}
        for (const theme of [Theme.Ftl, Theme.React]) {
            await page.goto(authorizeUrl(theme, randomUUID()))
            titles[theme] = (await page.locator("#kc-page-title").innerText()).trim()
        }
        evidence.realmOverride = {expected: OVERRIDE_TEXT, titles}
        expect(titles[Theme.Ftl]).toBe(OVERRIDE_TEXT)
        expect(titles[Theme.React]).toBe(OVERRIDE_TEXT)
    } finally {
        await admin(token, "DELETE", "/localization/en/loginAccountTitle")
    }
})

// Standard annotations (select options, helper text) and the Sequent
// html5-tel widget that sequent-theme renders with intl-tel-input.
const PROFILE_ATTRIBUTES = KEYCLOAK_PROFILE_ATTRIBUTES

test("register pages render the realm's User Profile metadata", async ({page}) => {
    const token = await adminToken()
    const realm = (await (await admin(token, "GET", "")).json()) as Json
    const profile = (await (await admin(token, "GET", "/users/profile")).json()) as Json
    await admin(token, "PUT", "", {...realm, registrationAllowed: true})
    await admin(token, "PUT", "/users/profile", {
        ...profile,
        attributes: [...(profile.attributes as Json[]), ...PROFILE_ATTRIBUTES],
    })
    try {
        const rendered: Record<string, unknown> = {}
        for (const theme of [Theme.Ftl, Theme.React]) {
            const parameters = new URLSearchParams({
                client_id: CLIENT_IDS[theme],
                redirect_uri: CALLBACK,
                response_type: "code",
                scope: "openid",
            })
            await page.goto(
                `${KEYCLOAK}/realms/${REALM}/protocol/openid-connect/registrations?${parameters}`
            )
            await expect(page.locator("form").first()).toBeVisible()
            rendered[theme] = await page.evaluate(() => {
                const colour = document.querySelector<HTMLSelectElement>(
                    "select[name='synthetic-colour']"
                )
                return {
                    fields: [
                        ...document.querySelectorAll<HTMLInputElement>("form input, form select"),
                    ]
                        .filter((field) => field.type !== "hidden" && field.name !== "")
                        .map(
                            (field) => `${field.tagName.toLowerCase()}:${field.type}:${field.name}`
                        ),
                    colourOptions:
                        colour === null ? null : [...colour.options].map((option) => option.value),
                    helperText: document.body.innerText.includes("Synthetic helper text"),
                    telWidget: document.querySelector(".iti") !== null,
                }
            })
            rendered[`${theme}.axe`] = await scanPage(page)
        }
        evidence.userProfile = rendered
        expect(rendered[Theme.React]).toEqual(rendered[Theme.Ftl])
        expect((rendered[Theme.React] as Json).telWidget).toBe(true)
    } finally {
        await admin(token, "PUT", "/users/profile", profile)
        await admin(token, "PUT", "", {...realm, registrationAllowed: realm.registrationAllowed})
    }
})

async function selectVotingTheme(token: string, theme: Theme): Promise<void> {
    const [client] = (await (
        await admin(token, "GET", `/clients?clientId=${CLIENT_IDS[theme]}`)
    ).json()) as Json[]
    await admin(token, "PUT", `/clients/${String(client.id)}`, {
        ...client,
        attributes: {
            login_theme: theme === Theme.React ? "sequent-ui-voting" : "sequent.voting-portal",
        },
    })
}

test("voting login policies preserve the hint and keep unrelated realm attributes off the page", async ({
    page,
}) => {
    const token = await adminToken()
    const realm = (await (await admin(token, "GET", "")).json()) as Json
    const privateMarker = `synthetic-private-${randomUUID()}`
    await admin(token, "PUT", "", {
        ...realm,
        attributes: {
            ...(realm.attributes as Json),
            "login-validation-policy": "SERVER_ONLY",
            loginHintUsernamePolicy: "READ_ONLY",
            "synthetic-private-attribute": privateMarker,
        },
    })
    try {
        for (const theme of [Theme.Ftl, Theme.React]) {
            await selectVotingTheme(token, theme)
            const url = new URL(authorizeUrl(theme, randomUUID()))
            url.searchParams.set("login_hint", USERNAME)
            await page.goto(url.href)
            await expect(page.locator("#username")).toHaveValue(USERNAME)
            await expect(page.locator("#username")).toHaveJSProperty("readOnly", true)
            await expect(page.locator("#kc-form-login")).toHaveJSProperty("noValidate", true)
            expect(await page.content()).not.toContain(privateMarker)
            if (theme === Theme.React) {
                expect((await kcContext(page)).sequent).toEqual({
                    loginValidationPolicy: "SERVER_ONLY",
                    loginHintUsernamePolicy: "READ_ONLY",
                })
                expect(JSON.stringify(await kcContext(page))).not.toContain(privateMarker)
            }
            await page.locator("#password").fill(PASSWORD)
            await page.locator("#kc-login").click()
            await expect(page.locator("#otp-1")).toBeVisible()
        }
        evidence.loginPolicies = {
            ok: true,
            readOnlyHint: true,
            serverOnlyValidation: true,
            unrelatedAttributeExcluded: true,
        }
    } finally {
        await admin(token, "PUT", "", realm)
        for (const theme of [Theme.Ftl, Theme.React]) await ensureClient(token, theme)
    }
})

test("structured credentials retain the Sequent FreeMarker widget in the opt-in theme", async ({
    page,
}) => {
    const token = await adminToken()
    const realm = (await (await admin(token, "GET", "")).json()) as Json
    await admin(token, "PUT", "", {
        ...realm,
        attributes: {
            ...(realm.attributes as Json),
            "credential-input-policy": "structured",
            "credential-input-pattern": "dddd-dddd",
        },
    })
    try {
        const widgets: Record<string, unknown> = {}
        for (const theme of [Theme.Ftl, Theme.React]) {
            await selectVotingTheme(token, theme)
            await page.goto(authorizeUrl(theme, randomUUID()))
            await expect(page.locator("[data-structured-credential]")).toBeVisible()
            await expect(page.locator("#structured-password")).toBeVisible()
            await page.locator("#structured-password").pressSequentially("12345678")
            await expect(page.locator("#password")).toHaveValue("12345678")
            widgets[theme] = await page
                .locator("[data-structured-credential]")
                .evaluate((element) => ({
                    pattern: element.getAttribute("data-credential-pattern"),
                    inputmode: element.querySelector("input")?.getAttribute("inputmode"),
                    value: element.querySelector("input")?.value,
                    toggle: element.querySelector("[data-structured-credential-toggle]") !== null,
                }))
        }
        expect(widgets[Theme.React]).toEqual(widgets[Theme.Ftl])
        evidence.structuredCredential = {ok: true, widgets}
    } finally {
        await admin(token, "PUT", "", realm)
        for (const theme of [Theme.Ftl, Theme.React]) await ensureClient(token, theme)
    }
})

test("automatic reload preserves a real login and OTP session", async ({page, browser}) => {
    const samples = Number(process.env.KEYCLOAK_UI_HMR_SAMPLES ?? 0)
    test.skip(samples < 1, "Set KEYCLOAK_UI_HMR_SAMPLES=10 with the Vite development server")
    test.setTimeout(180000)
    const state = randomUUID()
    await page.route(`${CALLBACK}**`, (route) => route.fulfill({status: 200, body: "callback"}))
    await page.goto(authorizeUrl(Theme.React, state))
    await page.locator("#username").fill(USERNAME)
    await page.locator("#password").fill(PASSWORD)
    const observations: Json = {
        cache: "warm: Vite module graph and Keycloak theme prepared, page already rendered",
        versions: {
            node: process.version,
            browser: browser.version(),
            ...Object.fromEntries(
                ["vite", "keycloakify", "react", "@playwright/test"].map((name) => [
                    name,
                    (
                        JSON.parse(
                            readFileSync(
                                join(process.cwd(), "../node_modules", name, "package.json"),
                                "utf8"
                            )
                        ) as {version: string}
                    ).version,
                ])
            ),
        },
        samples: {},
    }
    for (const [step, file, anchor] of [
        ["login", "Login.tsx", 'headerNode={msg("loginAccountTitle")}'],
        ["otp", "MessageOtpLogin.tsx", "headerNode={msg(`messageOtp.${flow}.address`, address)}"],
    ]) {
        const source = join(process.cwd(), "src/login/pages", file)
        const original = readFileSync(source, "utf8")
        expect(original.split(anchor)).toHaveLength(2)
        const beforeUrl = page.url()
        let navigations = 0
        const onNavigation = () => {
            navigations += 1
        }
        page.on("load", onNavigation)
        const timings: Json[] = []
        try {
            for (let index = 0; index <= samples; index += 1) {
                const marker = `hmr-${step}-${index}-${randomUUID()}`
                const loadBefore = loadavg()
                const startedAt = new Date().toISOString()
                const started = performance.now()
                writeFileSync(
                    source,
                    original.replace(
                        anchor,
                        `headerNode={<>{${anchor.slice(12, -1)}}<span data-testid="hmr-marker">${marker}</span></>}`
                    )
                )
                await expect(page.getByTestId("hmr-marker")).toHaveText(marker)
                timings.push({
                    index,
                    role: index === 0 ? "warmup" : "measured",
                    seconds: (performance.now() - started) / 1000,
                    startedAt,
                    finishedAt: new Date().toISOString(),
                    loadBefore,
                    loadAfter: loadavg(),
                })
                expect(page.url()).toBe(beforeUrl)
                expect(navigations).toBe(0)
                await expect(page.locator(step === "login" ? "#username" : "#otp-1")).toHaveValue(
                    step === "login" ? USERNAME : "1"
                )
            }
        } finally {
            writeFileSync(source, original)
            page.off("load", onNavigation)
        }
        await expect(page.getByTestId("hmr-marker")).toHaveCount(0)
        ;(observations.samples as Json)[step] = timings
        if (step === "login") {
            await expect(page.locator("#password")).toHaveValue(PASSWORD)
            const offset = logSize()
            await page.locator("#kc-login").click()
            await expect(page.locator("#otp-1")).toBeVisible()
            const code = await sentCode(offset)
            await page.locator("#otp-1").fill("1")
            // Held only in this test's memory and removed before recording evidence.
            observations.code = code
        }
    }
    const code = String(observations.code)
    delete observations.code
    await enterCode(page, code)
    await page.waitForURL((url) => url.href.startsWith(CALLBACK))
    expect(new URL(page.url()).searchParams.get("state")).toBe(state)
    expect(new URL(page.url()).searchParams.get("code")).toBeTruthy()
    evidence.hotReload = {...observations, loginAndOtpSessionRetained: true}

    // Source messages belong to Keycloak: editing one must reload the real page.
    await page.context().clearCookies()
    await page.goto(authorizeUrl(Theme.React, randomUUID()))
    const messages = join(
        process.cwd(),
        "../keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/login/messages/messages_en.properties"
    )
    const originalMessages = readFileSync(messages, "utf8")
    const title = `Synthetic auto reload ${randomUUID()}`
    try {
        writeFileSync(messages, `${originalMessages}\nloginAccountTitle=${title}\n`)
        await expect(page.locator("#kc-page-title")).toHaveText(title)
        evidence.messageAutoReload = {ok: true}
    } finally {
        writeFileSync(messages, originalMessages)
    }
    await expect(page.locator("#kc-page-title")).toHaveText("Login to the Admin Portal")
})
