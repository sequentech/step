// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test as base, expect, type Locator, type Page} from "@playwright/test"
import {readFile, writeFile} from "node:fs/promises"
import {parseSnapshot, ScenarioId} from "@sequentech/ui-test-kit/fixtures/scenarios"

const STORAGE_PREFIX = "sequent.workbench.v1."
const OVER_VOTE_DISABLES = "not-allowed-with-msg-and-disable"

/** Each test gets a fresh context; only the workbench origin may be reached. */
const test = base.extend<{violations: string[]}>({
    violations: async ({context, baseURL}, use) => {
        const origin = new URL(baseURL!).origin
        const violations: string[] = []
        await context.route("**/*", async (route) => {
            const url = new URL(route.request().url())
            if (url.origin === origin) return route.continue()
            violations.push(`${route.request().method()} ${url.href}`)
            await route.abort("blockedbyclient")
        })
        await context.routeWebSocket(/.*/, (socket) => {
            // Vite's own reload socket is the only one the workbench opens.
            if (new URL(socket.url()).host === new URL(origin).host) socket.connectToServer()
            else {
                violations.push(`WebSocket ${socket.url()}`)
                void socket.close()
            }
        })
        await use(violations)
        expect(violations, "requests outside the workbench").toEqual([])
    },
    page: async ({page, violations}, use) => {
        const errors: string[] = []
        page.on("pageerror", (error) => errors.push(error.message))
        await use(page)
        expect(errors, "unhandled page errors").toEqual([])
    },
})

const preview = (page: Page) => page.getByRole("region", {name: "Voting portal preview"})
const panels = (page: Page) => page.getByRole("complementary", {name: "Workbench panels"})
const scenarioSelect = (page: Page) =>
    page.getByRole("banner").getByRole("combobox", {name: /^Scenario/})
const candidate = (page: Page, name: string) =>
    preview(page).getByRole("checkbox", {name: new RegExp(name)})

async function choose(combobox: Locator, option: string) {
    await combobox.click()
    await combobox.page().getByRole("option", {name: option, exact: true}).click()
}

async function openScreen(page: Page, name: string) {
    await page.getByRole("navigation", {name: "Voter screens"}).getByRole("button", {name}).click()
}

test("select a scenario, change a policy, run the WASM pipeline, export, reset and import", async ({
    page,
}, testInfo) => {
    await page.goto("/")
    await expect(
        preview(page).getByRole("heading", {level: 1, name: "Community Council"})
    ).toBeVisible()

    // The kiosk scenario opens its chooser for the kiosk voter although online voting closed.
    await choose(scenarioSelect(page), "Kiosk voter")
    await openScreen(page, "Chooser")
    await expect(preview(page).getByRole("button", {name: /click to vote/i})).toBeEnabled()

    await choose(scenarioSelect(page), "Simple plurality")
    await openScreen(page, "Vote")
    await candidate(page, "Alice Example").click()
    await expect(candidate(page, "Bob Example")).toBeEnabled()

    // The policy reloads the snapshot; the production screen then disables a second choice.
    const council = panels(page).getByRole("region", {name: "Council representative"})
    await choose(council.getByRole("combobox", {name: "Over vote"}), OVER_VOTE_DISABLES)
    await expect(candidate(page, "Alice Example")).not.toBeChecked()
    await candidate(page, "Alice Example").click()
    await expect(candidate(page, "Bob Example")).toBeDisabled()

    await panels(page).getByRole("tab", {name: "Pipeline"}).click()
    await expect(panels(page).getByText("sequent-core WASM")).toBeVisible()
    await panels(page).getByRole("button", {name: "Run ballot pipeline"}).click()
    const steps = panels(page).getByRole("table", {name: "Pipeline steps"}).getByRole("row")
    await expect(steps).toHaveCount(6)
    for (const step of ["interpret", "check", "encrypt", "hash", "decode"])
        await expect(steps.filter({hasText: step})).toContainText("passed")
    await expect(steps.filter({hasText: "hash"})).toContainText(/[0-9a-f]{64}/)
    await expect(
        panels(page).getByText(/Decoded: Council representative: Alice Example/)
    ).toBeVisible()

    const download = page.waitForEvent("download")
    await page.getByRole("button", {name: "Export snapshot"}).click()
    const exported = await download
    expect(exported.suggestedFilename()).toBe(`${ScenarioId.SIMPLE_PLURALITY}-snapshot.json`)
    const text = await readFile((await exported.path())!, "utf8")
    const snapshot = parseSnapshot(text)
    expect(snapshot.provenance).toMatchObject({
        origin: "exported",
        changes: [`Council representative: over_vote_policy = ${OVER_VOTE_DISABLES}`],
    })
    expect(snapshot.preview.ballot_styles[0].contests[0].presentation).toMatchObject({
        over_vote_policy: OVER_VOTE_DISABLES,
    })

    await page.getByRole("button", {name: "Reset"}).click()
    await expect(page.getByText("Bundled", {exact: true})).toBeVisible()
    await candidate(page, "Alice Example").click()
    await expect(candidate(page, "Bob Example")).toBeEnabled()
    const stored = await page.evaluate(() => ({...window.localStorage}))
    expect(Object.keys(stored).every((key) => key.startsWith(STORAGE_PREFIX))).toBe(true)
    expect(JSON.parse(stored[`${STORAGE_PREFIX}state`])).toMatchObject({
        overrides: {},
        base: {provenance: {origin: "bundled"}},
    })

    // The exported document carries the policy, so importing it restores the behaviour.
    const file = testInfo.outputPath("exported.json")
    await writeFile(file, text)
    await page.locator("input[type=file]").setInputFiles(file)
    await expect(page.getByText("Imported", {exact: true})).toBeVisible()
    await candidate(page, "Alice Example").click()
    await expect(candidate(page, "Bob Example")).toBeDisabled()

    const invalid = testInfo.outputPath("invalid.json")
    await writeFile(invalid, JSON.stringify({...snapshot, version: 2}))
    await page.locator("input[type=file]").setInputFiles(invalid)
    const rejection = page.getByRole("banner").getByRole("alert")
    await expect(rejection).toContainText("The snapshot was not imported")
    await expect(rejection).toContainText("version: expected 1, found 2")
    await expect(page.getByText("Imported", {exact: true})).toBeVisible()
})

test("a scenario link opens the same screen the story shows", async ({page}) => {
    await page.goto(`/#/scenario/${ScenarioId.RANKED_MULTI_CONTEST}/review`)
    await expect(preview(page).getByRole("button", {name: "Cast ballot"})).toBeEnabled()
    await expect(preview(page).getByText("Park renovation", {exact: true})).toBeVisible()
    const nav = page.getByRole("navigation", {name: "Voter screens"})
    await expect(nav).toContainText("#/scenario/ranked-multi-contest/review")
    await expect(
        nav.getByRole("link", {name: "scenarios-ranked-multi-contest--review"})
    ).toBeVisible()
    await preview(page).getByRole("button", {name: "Cast ballot"}).click()
    await expect(
        preview(page).getByRole("heading", {level: 1, name: "Your vote has been cast"})
    ).toBeVisible()
    expect(new URL(page.url()).hash).toMatch(/\/confirmation$/)
})

test("requests to other origins are refused inside the page", async ({page}) => {
    await page.goto(`/#/scenario/${ScenarioId.SIMPLE_PLURALITY}/start`)
    await expect(preview(page).getByRole("button", {name: "Start Voting"})).toBeEnabled()
    const refused = await page.evaluate(() =>
        fetch("https://example.org/v1/graphql", {method: "POST"}).then(
            () => "sent",
            (error: Error) => error.message
        )
    )
    expect(refused).toBe("The workbench blocks POST https://example.org/v1/graphql in offline mode")
    await panels(page).getByRole("tab", {name: "Inspector"}).click()
    await expect(panels(page).getByRole("list", {name: "Workbench events"})).toContainText(
        "Blocked POST https://example.org/v1/graphql"
    )
})
