// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import {chromium} from "playwright-core"

/** Read a required fixture setting without including its value in errors. */
const required = (name) => {
    assert(process.env[name], `Missing ${name}`)
    return process.env[name]
}
const baseURL = required("PORTAL_URL")
const endpoint = required("HASURA_URL")
const secret = required("HASURA_ADMIN_SECRET")
const eventId = required("EVENT_ID")
const firstId = required("ORIGINAL_FIRST_ID")
const targetId = required("TARGET_FIRST_ID")
const targetName = required("TARGET_FIRST_NAME")
const originalName = required("ORIGINAL_FIRST_NAME")
const baseLanguage = process.env.BASE_LANGUAGE ?? "es"
const addedLanguage = process.env.ADDED_LANGUAGE ?? "en"
const broken = process.env.EXPECT_BROKEN === "true"
assert.equal(process.env.ALLOW_FIXTURE_RESET, "true", "Explicit disposable fixture reset required")
assert.notEqual(baseLanguage, addedLanguage)
assert.notEqual(firstId, targetId)

/** Execute against the real local Hasura instance and reject transport/schema errors. */
async function graphql(query, variables = {}) {
    const res = await fetch(endpoint, {
        method: "POST",
        headers: {"content-type": "application/json", "x-hasura-admin-secret": secret},
        body: JSON.stringify({query, variables}),
        signal: AbortSignal.timeout(30000),
    })
    assert.equal(res.ok, true)
    const result = await res.json()
    assert.equal(result.errors, undefined, JSON.stringify(result.errors))
    return result.data
}

/** Read both election positions and the event settings independently of browser state. */
async function read() {
    const data = await graphql(
        `
            query ($id: uuid!) {
                elections: sequent_backend_election(where: {election_event_id: {_eq: $id}}) {
                    id
                    presentation
                }
                events: sequent_backend_election_event(where: {id: {_eq: $id}}) {
                    id
                    presentation
                }
            }
        `,
        {id: eventId}
    )
    assert.equal(data.events.length, 1)
    return {elections: data.elections, event: data.events[0]}
}

/** Mutate exactly one fixture record, preserving its unrelated presentation properties. */
async function setPresentation(table, id, presentation) {
    assert(["election", "election_event"].includes(table))
    const data = await graphql(
        `mutation($id: uuid!, $presentation: jsonb!) {
        updated: update_sequent_backend_${table}(where: {id: {_eq: $id}}, _set: {presentation: $presentation}) { affected_rows }
    }`,
        {id, presentation}
    )
    assert.equal(data.updated.affected_rows, 1, "Fixture reset must update exactly one record")
}

const initial = await read()
assert.deepEqual(initial.elections.map((e) => e.id).sort(), [firstId, targetId].sort())
// Explicit distinct ranks avoid relying on Hasura's unspecified row order for null ties.
for (const election of initial.elections) {
    await setPresentation("election", election.id, {
        ...election.presentation,
        sort_order: election.id === firstId ? 0 : 1,
    })
}
await setPresentation("election_event", eventId, {
    ...initial.event.presentation,
    elections_order: "custom",
    language_conf: {
        ...initial.event.presentation.language_conf,
        enabled_language_codes: [baseLanguage],
        default_language_code: baseLanguage,
    },
})
const reset = await read()
assert.equal(reset.elections.find((e) => e.id === firstId).presentation.sort_order, 0)
assert.equal(reset.elections.find((e) => e.id === targetId).presentation.sort_order, 1)
assert.deepEqual(reset.event.presentation.language_conf.enabled_language_codes, [baseLanguage])
const browser = await chromium.launch({
    channel: "chrome",
    headless: true,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
})
try {
    const page = await browser.newPage({viewport: {width: 1600, height: 1000}})
    // Opening the fixture URL before login preserves the route through Keycloak's redirect.
    const eventURL = new URL(
        `sequent_backend_election_event/${eventId}`,
        baseURL.endsWith("/") ? baseURL : `${baseURL}/`
    ).href
    await page.goto(eventURL, {waitUntil: "domcontentloaded", timeout: 120000})
    await page.locator("#username").fill(required("PORTAL_USERNAME"))
    await page.locator("#password").fill(required("PORTAL_PASSWORD"))
    await page.locator("#kc-login").click()
    await page.getByRole("tab", {name: "Data", exact: true}).waitFor({timeout: 60000})
    assert.equal(new URL(page.url()).pathname.replace(/\/$/, ""), new URL(eventURL).pathname)

    /** Open the real fixture's Data form after navigation or a full reload. */
    async function openData() {
        await page.getByRole("tab", {name: "Data", exact: true}).click({timeout: 60000})
    }

    /** Expand custom ordering and wait until both fixture rows are rendered. */
    async function openDesign() {
        await page.getByRole("button", {name: "Ballot Design", exact: false}).click()
        await page.locator("[draggable=true]").nth(1).waitFor()
    }

    await openData()
    await page.getByRole("button", {name: "Language", exact: true}).click()
    const addedCheckbox = page.locator(`input[name="enabled_languages.${addedLanguage}"]`)
    assert.equal(await addedCheckbox.isChecked(), false)
    await addedCheckbox.check()
    await openDesign()
    const rows = page.locator("[draggable=true]")
    assert.equal(await rows.count(), 2)
    assert((await rows.first().innerText()).includes(originalName))
    await rows.filter({hasText: targetName}).dragTo(rows.filter({hasText: originalName}))
    assert((await rows.first().innerText()).includes(targetName))
    if (!broken) {
        // A real browser transport failure must not report a successful save.
        await page.context().setOffline(true)
        const failedRequest = page.waitForEvent("requestfailed", {
            predicate: (request) => request.url() === endpoint,
        })
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await failedRequest
        await page.evaluate(() => new Promise(requestAnimationFrame))
        assert.equal(
            await page
                .getByRole("alert")
                .filter({hasText: /success|updated|saved/i})
                .count(),
            0
        )
        assert.deepEqual(await read(), reset, "Failed save must leave the database unchanged")
        await page.context().setOffline(false)
        await page.reload()
        await openData()
        await page.getByRole("button", {name: "Language", exact: true}).click()
        await addedCheckbox.check()
        await openDesign()
        await rows.filter({hasText: targetName}).dragTo(rows.filter({hasText: originalName}))
    }
    const eventWrites = []
    page.on("request", (request) => {
        if (request.method() !== "POST" || request.url() !== endpoint) return
        const data = request.postDataJSON()
        if (
            data?.query?.includes("mutation") &&
            data.query.includes("update_sequent_backend_election_event")
        )
            eventWrites.push(data.variables)
    })
    await page.getByRole("button", {name: "Save", exact: true}).click()
    const expectedFirst = broken ? 0 : 1
    const expectedTarget = broken ? 1 : 0
    const expectedLanguages = broken ? [baseLanguage] : [baseLanguage, addedLanguage].sort()
    let persisted
    // The fixed transform awaits sequential election writes and then the event write.
    const deadline = Date.now() + (broken ? 10000 : 30000)
    do {
        await page.waitForTimeout(500)
        persisted = await read()
        if (
            !broken &&
            persisted.elections.find((e) => e.id === targetId).presentation.sort_order ===
                expectedTarget &&
            persisted.elections.find((e) => e.id === firstId).presentation.sort_order ===
                expectedFirst &&
            JSON.stringify(
                [...persisted.event.presentation.language_conf.enabled_language_codes].sort()
            ) === JSON.stringify(expectedLanguages)
        )
            break
    } while (Date.now() < deadline)
    assert.equal(
        persisted.elections.find((e) => e.id === targetId).presentation.sort_order,
        expectedTarget
    )
    assert.equal(
        persisted.elections.find((e) => e.id === firstId).presentation.sort_order,
        expectedFirst
    )
    assert.deepEqual(
        [...persisted.event.presentation.language_conf.enabled_language_codes].sort(),
        expectedLanguages
    )
    if (!broken) {
        assert.equal(eventWrites.length, 1, "Save must submit one normalized event update")
        const payload = eventWrites[0]._set
        assert(payload, "Expected Hasura update payload")
        for (const key of ["electionsOrder", "enabled_languages", "resultsWebsitePolicy"])
            assert.equal(Object.hasOwn(payload, key), false)
        assert.deepEqual(
            [...payload.presentation.language_conf.enabled_language_codes].sort(),
            expectedLanguages
        )
        assert.equal(
            payload.presentation.i18n[baseLanguage].name,
            initial.event.presentation.i18n[baseLanguage].name
        )
    }
    await page.reload()
    await openData()
    await page.getByRole("button", {name: "Language", exact: true}).click()
    assert.equal(await addedCheckbox.isChecked(), !broken)
    await openDesign()
    assert((await rows.first().innerText()).includes(broken ? originalName : targetName))
    console.log(
        JSON.stringify({
            expectedBroken: broken,
            reloadVerified: true,
            normalizedEventWrites: eventWrites.length,
            languages: expectedLanguages,
        })
    )
} finally {
    await browser.close()
}
