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
        // Long descriptions are shortened in the list; their start is shown.
        await expect(row).toContainText(
            `${head.event_type}${head.log_type}${head.description.slice(0, 40)}`
        )
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
