// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {VOTER_TAB_ROLES, expectRole, mockVoters, openVoters} from "./data"

const EMAIL = {
    subject: "Council reminder",
    plaintext_body: "Please vote today",
    html_body: "<p>Please vote today</p>",
}
test.use({roles: [...VOTER_TAB_ROLES, "notification-send", "communication-template-read"]})

for (const method of ["EMAIL", "SMS"] as const) {
    test(`schedules the saved ${method} template for non-voters and retries a refused send`, async ({
        page,
        portal,
    }) => {
        mockVoters(portal)
        portal.graphql.on("sequent_backend_template", () => ({
            data: {
                sequent_backend_template: [
                    {
                        id: "77777777-7777-4777-8777-777777777702",
                        tenant_id: TENANT_ID,
                        election_event_id: null,
                        name: "Reminder",
                        communication_method: method,
                        type: "CREDENTIALS",
                        template: {
                            alias: "Council reminder",
                            email: EMAIL,
                            sms: {message: "Please vote today"},
                        },
                        created_at: FIXED_TIME,
                        updated_at: FIXED_TIME,
                        created_by: "synthetic-admin",
                        annotations: {},
                        labels: {},
                    },
                ],
                sequent_backend_template_aggregate: {aggregate: {count: 1}},
            },
        }))
        portal.graphql.once("CreateScheduledEvent", () => ({
            errors: [{message: "Synthetic notification refused", extensions: {code: "Conflict"}}],
        }))
        portal.graphql.on("CreateScheduledEvent", () => ({
            data: {createScheduledEvent: {id: "77777777-7777-4777-8777-777777777703"}},
        }))
        await openVoters(page, portal)
        await page.getByRole("button", {name: "Send", exact: true}).first().click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Everyone", {exact: true})).toBeVisible()
        // These three selectors have visual captions but no accessible names.
        await drawer.getByRole("combobox").nth(0).click()
        await page.getByRole("option", {name: "Those who didn't vote yet", exact: true}).click()
        if (method === "SMS") {
            await drawer.getByRole("combobox").nth(1).click()
            await page.getByRole("option", {name: "SMS", exact: true}).click()
        }
        await drawer.getByRole("combobox").nth(2).click()
        await page.getByRole("option", {name: "Council reminder", exact: true}).click()
        if (method === "SMS") {
            await expect(drawer.getByRole("textbox", {name: "SMS Message"})).toHaveValue(
                "Please vote today"
            )
            await drawer
                .getByRole("textbox", {name: "SMS Message"})
                .fill("Council polls close at 18:00")
        } else {
            await expect(drawer.getByRole("textbox", {name: "Email Subject"})).toHaveValue(
                "Council reminder"
            )
        }
        await drawer.getByRole("switch", {name: "Send now"}).uncheck()
        await drawer.getByRole("button", {name: "Send Notification", exact: true}).click()
        await expect(drawer.getByText("Please choose a date", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("CreateScheduledEvent")).toHaveLength(0)
        await drawer
            .getByLabel("Date and time to start sending notifications")
            .fill("2026-01-16T10:30")
        await drawer.getByRole("button", {name: "Send Notification", exact: true}).click()
        await expect(
            drawer.getByText(
                "Error sending the notification: ApolloError: Synthetic notification refused",
                {exact: true}
            )
        ).toBeVisible()
        await drawer.getByRole("button", {name: "Send Notification", exact: true}).click()
        await expect(
            page.getByText("Notification programmed/sent successfully", {exact: true})
        ).toBeVisible()
        await expect(drawer).toHaveCount(0)
        const expected = {
            tenantId: TENANT_ID,
            electionEventId: IDS.event,
            eventProcessor: "SEND_TEMPLATE",
            eventPayload: {
                audience_selection: "NOT_VOTED",
                audience_voter_ids: [],
                communication_method: method,
                schedule_now: false,
                schedule_date: "2026-01-16T10:30:00.000Z",
                email:
                    method === "EMAIL"
                        ? EMAIL
                        : {
                              subject: "Participate in {{election_event.name}}",
                              plaintext_body:
                                  "Hello {{user.first_name}},\n\nEnter in {{vote_url}} to vote",
                              html_body:
                                  "<p>Hello {{user.first_name}},<br><br>Enter in {{vote_url}} to vote</p>",
                          },
                sms: {
                    message:
                        method === "SMS"
                            ? "Council polls close at 18:00"
                            : "Enter in {{vote_url}} to vote",
                },
                secret_attribute_names: [],
            },
        }
        expect(
            portal.graphql.callsTo("CreateScheduledEvent").map(({variables}) => variables)
        ).toEqual([expected, expected])
        expectRole(portal, "CreateScheduledEvent", "admin-user")
        expectRole(portal, "sequent_backend_template", "communication-template-read")
    })
}
