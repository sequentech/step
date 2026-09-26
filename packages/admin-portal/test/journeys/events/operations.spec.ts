// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {EVENT_ID, EVENT_ROLES, TENANT_ID, electionEvent, mockEvent, openEvent} from "./data"
import {mockTask, taskExecution} from "../settings/data"

const sidebarRoles = [
    ...EVENT_ROLES,
    "election-event-create",
    "election-event-delete",
    "election-event-archive",
]

async function sidebarAction(page: Page, name: string) {
    await page.getByRole("link", {name: "Council election", exact: true}).hover()
    // The sidebar's More icon has no accessible button or name.
    await page.locator(".menu-actions-sequent_backend_election_event #MoreHorizIcon").click()
    await page.getByRole("menuitem", {name, exact: true}).click()
    return page.getByRole("dialog", {name: "Warning", exact: true})
}

test.describe("event sidebar actions", () => {
    test.use({roles: sidebarRoles})

    test("cancels deletion, then deletes precisely the selected event and reloads the tree", async ({
        page,
        portal,
    }) => {
        mockEvent(portal)
        let deleted = false
        portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
        portal.graphql.on("election_events_tree", () => ({
            data: {sequent_backend_election_event: deleted ? [] : [electionEvent()]},
        }))
        const task = () =>
            taskExecution("DELETE_ELECTION_EVENT", {
                execution_status: deleted ? "SUCCESS" : "IN_PROGRESS",
            })
        mockTask(portal, task)
        portal.graphql.on("DeleteElectionEvent", () => ({
            data: {delete_election_event: {id: EVENT_ID, error_msg: null, task_execution: task()}},
        }))
        await openEvent(page, portal)
        const canceled = await sidebarAction(page, "Remove this Election Event")
        await canceled.getByRole("button", {name: "Cancel", exact: true}).click()
        expect(portal.graphql.callsTo("DeleteElectionEvent")).toHaveLength(0)
        const confirmed = await sidebarAction(page, "Remove this Election Event")
        await confirmed.getByRole("button", {name: "Delete", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
        const calls = portal.graphql.callsTo("DeleteElectionEvent")
        expect(calls).toHaveLength(1)
        expect(calls[0].variables).toEqual({electionEventId: EVENT_ID})
        expect(calls[0].headers["x-hasura-role"]).toBe("election-event-delete")
        expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: task().id})
        deleted = true
        await page.clock.runFor(250)
        await expect(page.getByText("SUCCESS", {exact: true})).toBeVisible()
        await expect(page.getByRole("link", {name: "Council election", exact: true})).toHaveCount(0)
    })

    for (const archived of [false, true]) {
        test(`${archived ? "unarchives" : "archives"} the selected event only after confirmation`, async ({
            page,
            portal,
        }) => {
            let current = electionEvent({is_archived: archived})
            mockEvent(portal, () => current)
            portal.graphql.on("election_events_tree", ({variables}) => ({
                data: {
                    sequent_backend_election_event:
                        variables.isArchived === current.is_archived ? [current] : [],
                },
            }))
            portal.graphql.on("update_sequent_backend_election_event", ({variables}) => {
                current = {...current, ...(variables._set as object)}
                return {
                    data: {
                        update_sequent_backend_election_event: {
                            affected_rows: 1,
                            returning: [current],
                        },
                    },
                }
            })
            await openEvent(page, portal)
            if (archived) await page.getByText("Archived", {exact: true}).click()
            const action = archived ? "Unarchive" : "Archive"
            const canceled = await sidebarAction(page, `${action} this Election Event`)
            await canceled.getByRole("button", {name: "Cancel", exact: true}).click()
            expect(portal.graphql.callsTo("update_sequent_backend_election_event")).toHaveLength(0)
            const confirmed = await sidebarAction(page, `${action} this Election Event`)
            await confirmed.getByRole("button", {name: action, exact: true}).click()
            await expect(
                page.getByText(`The item has been ${archived ? "unarchived" : "archived"}`, {
                    exact: true,
                })
            ).toBeVisible()
            const calls = portal.graphql.callsTo("update_sequent_backend_election_event")
            expect(calls).toHaveLength(1)
            expect(calls[0].variables).toEqual({
                where: {id: {_eq: EVENT_ID}},
                _set: {is_archived: !archived},
            })
            expect(calls[0].headers["x-hasura-role"]).toBe("election-event-write")
        })
    }
})

const logRoles = [...EVENT_ROLES, "election-event-logs-tab", "logs-read", "logs-export"]
function mockLogs(portal: AdminPortal) {
    mockEvent(portal)
    portal.graphql.on("listElectoralLog", () => ({
        data: {
            listElectoralLog: {
                items: [
                    {
                        id: 1,
                        created: 1768478400,
                        statement_timestamp: 1768478400,
                        statement_kind: "info",
                        user_id: "synthetic-admin",
                        message: JSON.stringify({
                            username: "synthetic-admin",
                            statement: {
                                head: {
                                    event_type: "Election created",
                                    log_type: "info",
                                    description: "Synthetic event created",
                                },
                            },
                        }),
                    },
                ],
                total: {aggregate: {count: 1}},
            },
        },
    }))
}

test.describe("event activity log exports", () => {
    test.use({roles: logRoles})
    for (const format of ["CSV", "PDF"]) {
        test(`confirms the exact ${format} export and tracks its task`, async ({page, portal}) => {
            mockLogs(portal)
            let done = false
            portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
            const task = () =>
                taskExecution("EXPORT_ACTIVITY_LOGS_REPORT", {
                    execution_status: done ? "SUCCESS" : "IN_PROGRESS",
                })
            mockTask(portal, task)
            portal.graphql.on("ExportElectionEventLogs", () => ({
                data: {
                    export_election_event_logs: {
                        document_id: "logs-document",
                        task_execution: task(),
                    },
                },
            }))
            await openEvent(page, portal, "Logs")
            await expect(
                page.getByRole("cell", {name: "synthetic-admin", exact: true}).first()
            ).toBeVisible()
            for (const confirm of [false, true]) {
                await page.getByRole("button", {name: "Export", exact: true}).click()
                await page.getByRole("menuitem", {name: `Export in ${format}`, exact: true}).click()
                expect(portal.graphql.callsTo("ExportElectionEventLogs")).toHaveLength(0)
                await page
                    .getByRole("dialog")
                    .getByRole("button", {name: confirm ? "Export" : "Cancel", exact: true})
                    .click()
                await expect(page.getByRole("dialog")).toHaveCount(0)
            }
            await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
            const calls = portal.graphql.callsTo("ExportElectionEventLogs")
            expect(calls).toHaveLength(1)
            expect(calls[0].variables).toEqual({electionEventId: EVENT_ID, format})
            expect(calls[0].headers["x-hasura-role"]).toBe("logs-export")
            expect(
                portal.oidc.verifyAccessToken(
                    calls[0].headers.authorization.replace(/^Bearer /, "")
                )
            ).toMatchObject({"https://hasura.io/jwt/claims": {"x-hasura-tenant-id": TENANT_ID}})
            done = true
            await page.clock.runFor(250)
            await expect(page.getByText("SUCCESS", {exact: true})).toBeVisible()
        })
    }
})

test.describe("action permissions", () => {
    test.use({roles: logRoles.filter((role) => role !== "logs-export")})
    test("shows log rows but no export or destructive sidebar action without those permissions", async ({
        page,
        portal,
    }) => {
        mockLogs(portal)
        await openEvent(page, portal, "Logs")
        await expect(
            page.getByRole("cell", {name: "synthetic-admin", exact: true}).first()
        ).toBeVisible()
        await expect(page.getByRole("button", {name: "Export", exact: true})).toHaveCount(0)
        await expect(
            page.locator(".menu-actions-sequent_backend_election_event #MoreHorizIcon")
        ).toHaveCount(0)
        expect(portal.graphql.calls.some((call) => call.query.trim().startsWith("mutation"))).toBe(
            false
        )
    })
})
