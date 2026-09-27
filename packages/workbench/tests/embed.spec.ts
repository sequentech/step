// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test as base, expect, type Page} from "@playwright/test"
import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"

const PROTOCOL = "sequent.voter-preview"
const VERSION = 1
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

/** Only the workbench origin may be reached, and no page may throw. */
const test = base.extend<{violations: string[]}>({
    violations: async ({context, baseURL}, use) => {
        const origin = new URL(baseURL!).origin
        const violations: string[] = []
        await context.route("**/*", async (route) => {
            const url = new URL(route.request().url())
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
