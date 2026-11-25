// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../fixtures"
import {EVENT_ID, EVENT_URL, JOSE, MARIA, mockSigningEvent, type Person} from "./data"

const CODE = "7F3A-91C2"
const START = Date.parse("2026-01-15T10:30:00Z") / 1000

/** One step of a close request: its USER entry (who) and SYSTEM entry (what was checked or done). */
interface IStep {
    kind: string
    person: Person | null
    logType: "INFO" | "ERROR"
    description: string
}
const STEPS: IStep[] = [
    {
        kind: "SigningRequestCreated",
        person: MARIA,
        logType: "INFO",
        description: `Started signing request ${CODE} to close voting: 2 signatures needed`,
    },
    {
        kind: "SigningRequestSigned",
        person: MARIA,
        logType: "INFO",
        description: `Signature verified on ${CODE}: 1 of 2`,
    },
    {
        kind: "SigningHandover",
        person: MARIA,
        logType: "INFO",
        description: `Handed signing request ${CODE} over to the next member`,
    },
    {
        kind: "SigningSignatureRefused",
        person: JOSE,
        logType: "ERROR",
        description: `Signature refused on ${CODE}: valid-now`,
    },
    {
        kind: "SigningRequestSigned",
        person: JOSE,
        logType: "INFO",
        description: `Signature verified on ${CODE}: 2 of 2`,
    },
    {
        kind: "SigningRequestCompleted",
        person: JOSE,
        logType: "INFO",
        description: `Signing request ${CODE} has every signature: 2 of 2`,
    },
    {
        kind: "SigningActionExecuted",
        person: JOSE,
        logType: "INFO",
        description: `Ran Close voting for signing request ${CODE}`,
    },
]

/** The board's entries as listElectoralLog answers them: newest first, two per step. */
function entries() {
    const rows = STEPS.flatMap((step, index) =>
        ["USER", "SYSTEM"].map((eventType) => ({
            step,
            index,
            eventType,
        }))
    ).map(({step, index, eventType}, id) => {
        const user = eventType === "USER" ? step.person : null
        const timestamp = START + index * 60
        return {
            id: id + 1,
            created: timestamp + 2,
            statement_timestamp: timestamp,
            statement_kind: step.kind,
            // The client schema makes the row's user_id non-null (String!), though the board
            // has none for SYSTEM entries; the list reads the message's own user_id.
            user_id: user?.userId ?? "",
            message: JSON.stringify({
                user_id: user?.userId ?? null,
                username: user?.username ?? null,
                statement: {
                    head: {
                        event: EVENT_ID,
                        kind: step.kind,
                        timestamp,
                        event_type: eventType,
                        // A refused attempt: the person's entry is INFO, the system's ERROR.
                        log_type: eventType === "SYSTEM" ? step.logType : "INFO",
                        description: step.description,
                    },
                    body: {Signing: {kind: step.kind, details_json: "{}"}},
                },
            }),
        }
    })
    return rows.reverse()
}

test.use({
    roles: ["election-event-read", "election-read", "election-event-logs-tab", "logs-read"],
    // Wide enough for the table: a list under 1,000 px shows two lines per entry.
    viewport: {width: 1920, height: 1080},
})

test("Election Event > Logs lists each signing step as a USER and a SYSTEM entry", async ({
    page,
    portal,
}) => {
    mockSigningEvent(portal)
    const items = entries()
    portal.graphql.on("listElectoralLog", () => ({
        data: {listElectoralLog: {items, total: {aggregate: {count: items.length}}}},
    }))
    await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
    await page.getByRole("tab", {name: "Logs", exact: true}).click()

    const headers = page.getByRole("columnheader")
    await expect(headers.filter({hasText: "Event Type"})).toHaveCount(1)
    await expect(headers.filter({hasText: "Statement kind"})).toHaveCount(1)
    const rows = page.getByRole("row").filter({has: page.getByRole("cell")})
    await expect(rows).toHaveCount(items.length)
    for (const [index, item] of items.entries()) {
        const head = JSON.parse(item.message).statement.head
        const row = rows.nth(index)
        await expect(row).toContainText(item.statement_kind)
        await expect(row).toContainText(`${head.event_type}${head.log_type}`)
        // The description stays on one line; the entry's popup shows it whole.
        await expect(row).toContainText(head.description)
        const username = JSON.parse(item.message).username
        await expect(row.getByRole("cell").nth(1)).toHaveText(username ?? "-")
    }
    // The refused attempt is an ERROR from the system and names Jose in its USER entry.
    const refused = rows.filter({hasText: "SigningSignatureRefused"})
    await expect(refused).toHaveCount(2)
    await expect(refused.filter({hasText: "SYSTEMERROR"})).toHaveCount(1)
    await expect(refused.filter({hasText: JOSE.username})).toHaveCount(1)
    expect(portal.graphql.callsTo("listElectoralLog")[0].variables).toMatchObject({
        election_event_id: EVENT_ID,
        order_by: {id: "desc"},
    })
})

test("event log details show the complete HSM approver fingerprint, separate from the log signature", async ({
    page,
    portal,
}) => {
    mockSigningEvent(portal)
    // Synthetic rows with the actual two HSM proof shapes, using the entry numbers under review.
    const [system, user] = entries().filter(
        (entry) => entry.statement_kind === "SigningRequestSigned"
    )
    const systemSignature = "f0".repeat(32)
    const fixtures = [
        {
            id: 106,
            entry: system,
            proof: "direct-action-demo-pkcs11-v2",
            fingerprint: "1e".repeat(32),
        },
        {id: 59, entry: user, proof: "initialize-voting-demo-pkcs11", fingerprint: "2f".repeat(32)},
        {id: 58, entry: system, proof: "direct-action-demo-pkcs11-v2", fingerprint: "invalid"},
        {id: 57, entry: user, proof: undefined, fingerprint: undefined},
    ]
    const items = fixtures.map(({id, entry, proof, fingerprint}) => {
        const message = JSON.parse(entry.message)
        message.system_signature = systemSignature
        message.statement.body.Signing.details_json = JSON.stringify(
            proof
                ? {proof_kind: proof, spki_sha256: fingerprint}
                : {certificate: {spki_sha256: "00".repeat(32)}}
        )
        return {...entry, id, message: JSON.stringify(message)}
    })
    portal.graphql.on("listElectoralLog", () => ({
        data: {listElectoralLog: {items, total: {aggregate: {count: items.length}}}},
    }))
    await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
    await page.getByRole("tab", {name: "Logs", exact: true}).click()
    await expect(page.getByText("Approver key fingerprint", {exact: true})).toHaveCount(0)

    for (const {id, fingerprint} of fixtures) {
        await page.getByRole("cell", {name: String(id), exact: true}).click()
        const dialog = page.getByRole("dialog", {name: `Log entry ${id}`, exact: true})
        await expect(dialog.getByText("System signature", {exact: true})).toBeVisible()
        await expect(dialog.getByTitle(systemSignature, {exact: true})).toBeVisible()
        const field = dialog.getByText("Approver key fingerprint", {exact: true})
        if (fingerprint?.length === 64) {
            await expect(field).toBeVisible()
            const value = dialog.getByText(fingerprint, {exact: true})
            await expect(value).toBeVisible()
            await value.dblclick()
            await expect
                .poll(() => page.evaluate(() => window.getSelection()?.toString()))
                .toBe(fingerprint)
        } else {
            await expect(field).toHaveCount(0)
        }
        await dialog.getByRole("button", {name: "Close", exact: true}).click()
        await expect(dialog).toBeHidden()
    }
})

test("Election Event > Logs filters by any statement kind under Add filter", async ({
    page,
    portal,
}) => {
    mockSigningEvent(portal)
    const items = entries()
    portal.graphql.on("listElectoralLog", ({variables}) => {
        const kind = (variables.filter as {statement_kind?: string} | undefined)?.statement_kind
        const listed = kind ? items.filter((item) => item.statement_kind === kind) : items
        return {
            data: {listElectoralLog: {items: listed, total: {aggregate: {count: listed.length}}}},
        }
    })
    await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
    await page.getByRole("tab", {name: "Logs", exact: true}).click()
    const rows = page.getByRole("row").filter({has: page.getByRole("cell")})
    await expect(rows).toHaveCount(items.length)

    // No shortcut for one family of kinds: every kind is under Add filter.
    await expect(page.getByRole("button", {name: "Signing activity"})).toHaveCount(0)
    await page.getByRole("button", {name: "Add filter", exact: true}).click()
    await page.getByRole("menuitemcheckbox", {name: "Statement kind", exact: true}).click()
    await page.getByRole("combobox", {name: "Statement kind", exact: true}).click()
    const options = page.getByRole("option").filter({hasText: /\w/})
    // The backend's 76 kinds, by name.
    await expect(options).toHaveCount(76)
    await expect(page.getByRole("option", {name: "Lockdown changed", exact: true})).toHaveCount(1)
    await expect(page.getByRole("option", {name: "SigningActionExecuted"})).toHaveCount(0)
    await page.getByRole("option", {name: "Approval threshold reached", exact: true}).click()

    // The server filters by the exact kind; the page shows what it answers.
    await expect(rows).toHaveCount(2)
    await expect(rows.filter({hasText: "SigningRequestCompleted"})).toHaveCount(2)
    expect(portal.graphql.callsTo("listElectoralLog").at(-1)?.variables).toMatchObject({
        election_event_id: EVENT_ID,
        filter: {statement_kind: "SigningRequestCompleted"},
    })
})
