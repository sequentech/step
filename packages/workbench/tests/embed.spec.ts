// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test as base, expect, type Page} from "@playwright/test"
import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"

const PROTOCOL = "sequent.voter-preview"
const VERSION = 2
const HOST = "/embed-host.html"

/**
 * A page that frames the embed as another tool does, e.g. the Election Architect, and
 * keeps every message the embed sends it.
 */
const HOST_PAGE = `<!doctype html>
<html><body>
<iframe id="preview" title="Voter preview" src="/embed.html" style="width:1200px;height:860px"></iframe>
<script>
    window.replies = []
    window.addEventListener("message", (event) => {
        if (event.source === document.getElementById("preview").contentWindow)
            window.replies.push(event.data)
    })
</script>
</body></html>`

/**
 * A stand-in for the IVR emulator, served by the "framing tool" from another origin as the
 * Election Architect serves the real one: wasm-bindgen's shim and its binary.
 */
const EMULATOR = "http://architect.test/wasm/ivr_emulator_wasm"
const EMULATOR_SHIM = `
const says = (text) => ({prompt_text: "<speak>" + text + "</speak>", language: "en-US", voice_id: "Joanna"})
export default async function initWasm() {}
export function init() {}
export class IvrEmulatorDriver {
    constructor(config) {
        this.queue = [
            {type: "Prompt", prompt: says("Welcome, caller " + config.caller_number + ".")},
            {type: "Prompt", prompt: says('<lang xml:lang="es-ES">Hola</lang>, <lang xml:lang="en-US">hello</lang>.')},
            {type: "ExpectInput", prompt: says("Press 1."), valid_inputs: "1", max_digits: 1, timeout: 5},
        ]
    }
    async execute() { return this.queue.shift() ?? {type: "Noop"} }
    send_input(input) {
        // A PIN next: the Lambda lists no valid inputs when any digits will do.
        this.queue.push(this.pin
            ? {type: "Disconnect", prompt: says("You pressed " + input + ".")}
            : {type: "ExpectInput", prompt: says("Enter your PIN."), valid_inputs: "", max_digits: 8, timeout: 5})
        this.pin = true
    }
    send_timeout() {}
    free() {}
}`
const CORS = {"access-control-allow-origin": "*"}

/** Only the workbench origin may be reached, and no page may throw. */
const test = base.extend<{violations: string[]}>({
    violations: async ({context, baseURL}, use) => {
        const origin = new URL(baseURL!).origin
        const violations: string[] = []
        await context.route("**/*", async (route) => {
            const url = new URL(route.request().url())
            if (url.href === `${EMULATOR}.js`)
                return route.fulfill({
                    contentType: "text/javascript",
                    headers: CORS,
                    body: EMULATOR_SHIM,
                })
            if (url.href === `${EMULATOR}_bg.wasm`)
                return route.fulfill({contentType: "application/wasm", headers: CORS, body: ""})
            if (url.origin === new URL(EMULATOR).origin)
                return route.fulfill({status: 404, headers: CORS, body: "not here"})
            if (url.origin !== origin) {
                violations.push(`${route.request().method()} ${url.href}`)
                return route.abort("blockedbyclient")
            }
            if (url.pathname === HOST)
                return route.fulfill({contentType: "text/html", body: HOST_PAGE})
            return route.continue()
        })
        await use(violations)
        expect(violations, "requests outside the workbench").toEqual([])
    },
    page: async ({page, violations}, use) => {
        void violations
        const errors: string[] = []
        page.on("pageerror", (error) => errors.push(error.message))
        await use(page)
        expect(errors, "unhandled page errors").toEqual([])
    },
})

type Reply = {protocol: string; version: number; type: string; [key: string]: unknown}

const replies = (page: Page) =>
    page.evaluate(() => (window as unknown as {replies: Reply[]}).replies)

const lastReply = async (page: Page) => (await replies(page)).at(-1)

/** Opens the framing page once the embed says it listens (StrictMode may say so twice). */
async function openHost(page: Page) {
    await page.goto(HOST)
    await expect.poll(async () => (await replies(page)).map(({type}) => type)).toContain("ready")
}

async function send(page: Page, message: Record<string, unknown>) {
    await page.evaluate((data) => {
        const frame = document.getElementById("preview") as HTMLIFrameElement
        frame.contentWindow!.postMessage(data, window.location.origin)
    }, message)
}

const show = (fields: Record<string, unknown> = {}) => {
    const {preview, areaId} = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    return {
        protocol: PROTOCOL,
        version: VERSION,
        type: "show",
        document: preview,
        areaId,
        screen: "vote",
        ...fields,
    }
}

const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`

test("a framing page sends a document, the voter votes, and every screen is reported", async ({
    page,
}) => {
    await openHost(page)
    const frame = page.frameLocator("#preview")
    await expect(frame.getByText("Waiting for a ballot to preview")).toBeVisible()
    for (const reply of await replies(page))
        expect(reply).toEqual({protocol: PROTOCOL, version: VERSION, type: "ready"})

    await send(page, show())
    // The portal's own chrome surrounds the screen, as in production.
    await expect(frame.locator(".voting-portal-wrapper .voting-portal.app-root")).toBeAttached()
    await frame.getByRole("checkbox", {name: /Alice Example/}).click()
    await expect
        .poll(() => lastReply(page))
        .toMatchObject({
            type: "shown",
            screen: "vote",
            path: `${electionPath}/vote`,
        })

    await frame.getByRole("button", {name: "Next"}).click()
    await expect(frame.getByRole("heading", {level: 1, name: "Review your ballot"})).toBeVisible()
    await expect(frame.getByText("Alice Example", {exact: true})).toBeVisible()
    await expect.poll(() => lastReply(page)).toMatchObject({type: "shown", screen: "review"})

    // A new request opens a new session at the requested screen.
    await send(page, show({screen: "chooser"}))
    await expect(frame.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    await expect.poll(() => lastReply(page)).toMatchObject({type: "shown", screen: "chooser"})
})

test("a request the embed cannot follow is refused with its reasons", async ({page}) => {
    await openHost(page)
    await send(page, show({version: VERSION + 1}))
    await expect
        .poll(() => lastReply(page))
        .toEqual({
            protocol: PROTOCOL,
            version: VERSION,
            type: "failed",
            issues: [`version: expected ${VERSION}, found ${VERSION + 1}`],
        })
    await expect(page.frameLocator("#preview").getByRole("alert")).toContainText(
        "The voter preview could not open this request"
    )
})

test("a document the portal rejects is reported to the framing page", async ({page}) => {
    await openHost(page)
    const message = show()
    const [contest] = message.document.ballot_styles[0].contests
    contest.candidates = contest.candidates.map((candidate) => ({
        ...candidate,
        presentation: {is_explicit_invalid: true},
    }))
    await send(page, message)
    await expect.poll(() => lastReply(page)).toMatchObject({type: "failed"})
    await expect(page.frameLocator("#preview").getByRole("alert")).toContainText(
        "The portal could not load this snapshot"
    )
})

test("the demo watermark is the bundled banner, not one at the host's root", async ({page}) => {
    await openHost(page)
    await send(page, show())
    const watermark = page.frameLocator("#preview").locator(".watermark-background")
    await expect(watermark).toBeAttached()
    const image = await watermark.evaluate(
        (element) => getComputedStyle(element, "::before").backgroundImage
    )
    const url = /^url\("?(.*?)"?\)$/.exec(image)?.[1]
    expect(url, `the watermark's background, ${image}`).toBeTruthy()
    expect(new URL(url!).pathname).not.toBe("/demo-banner.png")
    const response = await page.request.get(url!)
    expect(response.ok()).toBe(true)
    expect(response.headers()["content-type"]).toContain("image/png")
})

const call = (fields: Record<string, unknown> = {}) => ({
    protocol: PROTOCOL,
    version: VERSION,
    type: "call",
    emulatorUrl: EMULATOR,
    config: {
        caller_number: "+1234567890",
        contact_id: "00000000-0000-4000-8000-000000000001",
        tenant_id: IDS.tenant,
        election_event_id: IDS.event,
        election_event: JSON.stringify({id: IDS.event}),
        ballot_styles: [JSON.stringify({id: "style"})],
        open_elections: [IDS.election],
        blacklisted_numbers: [],
    },
    labels: {input: "Keys to press", send: "Press"},
    ...fields,
})

test("a framing page places a call against the emulator it serves", async ({page}) => {
    await openHost(page)
    await send(page, call())
    const frame = page.frameLocator("#preview")
    await expect(frame.getByText("Welcome, caller +1234567890.")).toBeVisible()
    // SSML is shown as its words: a badge for the other language, and no tags.
    const greeting = frame.getByTestId("ivr-call-prompt").filter({hasText: "Hola"})
    await expect(greeting).toHaveText("ES Hola, hello.")
    await expect(greeting.getByText("Hola")).toHaveAttribute("lang", "es-ES")
    await expect
        .poll(() => lastReply(page))
        .toMatchObject({type: "calling", status: "ExpectingInput"})

    const keypad = frame.getByRole("textbox", {name: "Keys to press"})
    await expect(keypad).toHaveAttribute("placeholder", "Up to 1 of 1, within 5s")
    await keypad.fill("1")
    await frame.getByRole("button", {name: "Press"}).click()
    await expect(frame.getByText("Enter your PIN.")).toBeVisible()
    await expect(keypad).toHaveAttribute("placeholder", "Up to 8 digits, within 5s")
    await keypad.fill("12345678")
    await frame.getByRole("button", {name: "Press"}).click()
    await expect(frame.getByText("You pressed 12345678.")).toBeVisible()
    await expect
        .poll(() => lastReply(page))
        .toMatchObject({type: "calling", status: "Disconnected"})

    // The voter's screens are still one request away.
    await send(page, show())
    await expect(frame.getByRole("checkbox", {name: /Alice Example/})).toBeVisible()
})

test("a call to an emulator that is not served says it is absent", async ({page}) => {
    await openHost(page)
    await send(page, call({emulatorUrl: "http://architect.test/nowhere/ivr_emulator_wasm"}))
    await expect.poll(() => lastReply(page)).toMatchObject({type: "calling", status: "absent"})
    await expect(page.frameLocator("#preview").getByRole("status")).toContainText(
        "No telephone emulator is served at"
    )
})
