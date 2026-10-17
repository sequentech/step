// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {eventRecord, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {ReadinessCheckButton, ReadinessDialog} from "./ReadinessCheckButton"

interface Scenario {
    outcome: "passed" | "failed" | "error" | "unsigned" | "pending"
    eventList: "records" | "error" | "empty"
    authorized: boolean
}
const TESTED_ID = storyId(2, 3)
const tested = eventRecord(undefined, {
    id: TESTED_ID,
    is_archived: true,
    presentation: {i18n: {en: {name: "Archived rehearsal"}}},
})
const report = (passed: boolean) => ({
    passed,
    checked_at: "2026-10-16T09:00:00Z",
    log_entry_id: passed ? "signed-readiness-entry" : null,
    checks: [
        "configuration",
        "certified_voters",
        "official_zero",
        "publication",
        "tested_finished",
    ].map((kind) => ({
        kind,
        passed: passed || kind !== "certified_voters",
        message: "Authoritative server check",
    })),
    evidence: {official: {event_id: EVENT_ID}, tested: {event_id: TESTED_ID}},
})
let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Election event/ReadinessDialog",
    component: ReadinessDialog,
    args: {outcome: "passed", eventList: "records", authorized: true},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary(
            {
                GetReadinessEvents: () =>
                    args.eventList === "error"
                        ? {errors: [new GraphQLError("Event access denied")]}
                        : {
                              data: {
                                  sequent_backend_election_event:
                                      args.eventList === "empty"
                                          ? []
                                          : [
                                                eventRecord(undefined, {
                                                    id: storyId(2, 5),
                                                    presentation: {
                                                        i18n: {en: {name: "Zulu rehearsal"}},
                                                    },
                                                }),
                                                tested,
                                                eventRecord(undefined, {
                                                    id: storyId(2, 4),
                                                    presentation: {
                                                        i18n: {en: {name: "Alpha rehearsal"}},
                                                    },
                                                }),
                                            ],
                              },
                          },
                ReadinessCheck: () =>
                    args.outcome === "error"
                        ? {errors: [new GraphQLError("Readiness denied")]}
                        : args.outcome === "pending"
                          ? new Promise(() => {})
                          : {
                                data: {
                                    readiness_check:
                                        args.outcome === "unsigned"
                                            ? {...report(true), log_entry_id: null}
                                            : report(args.outcome !== "failed"),
                                },
                            },
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: (args) => (
        <AdminStoryProvider
            boundary={boundary}
            roles={
                args.authorized
                    ? [IPermissions.READINESS_CHECK, IPermissions.ELECTION_EVENT_READ]
                    : [IPermissions.ELECTION_EVENT_READ]
            }
        >
            <ReadinessCheckButton event={eventRecord()} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
const text = (key: string) => i18n.t(`readiness.${key}`)
async function open(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: text("action")}))
    const dialog = within(await within(document.body).findByRole("dialog", {name: text("title")}))
    await waitFor(() => expect(dialog.getByRole("combobox")).toBeEnabled())
    expect(dialog.getByRole("button", {name: text("run")})).toBeDisabled()
    return dialog
}
async function run(canvasElement: HTMLElement) {
    const dialog = await open(canvasElement)
    await userEvent.click(dialog.getByRole("combobox"))
    const options = await within(document.body).findAllByRole("option")
    expect(options.map((option) => option.textContent?.split(" · ")[0])).toEqual([
        "Alpha rehearsal",
        "Archived rehearsal",
        "Zulu rehearsal",
    ])
    const option = within(document.body).getByRole("option", {name: /Archived rehearsal/})
    expect(option).toHaveTextContent(text("archived"))
    await userEvent.click(option)
    await userEvent.click(dialog.getByRole("button", {name: text("run")}))
    return dialog
}
function expectChecklistAlignment(dialog: ReturnType<typeof within>) {
    const list = dialog.getByRole("list", {name: text("results")})
    for (const row of within(list).getAllByRole("listitem")) {
        const pill = row.querySelector(".MuiChip-root") as HTMLElement
        const label = row.querySelector("p") as HTMLElement
        expect(
            Math.abs(pill.getBoundingClientRect().top - label.getBoundingClientRect().top)
        ).toBeLessThan(1)
    }
}
export const Passed: Story = {
    play: async ({canvasElement}) => {
        const dialog = await run(canvasElement)
        await expect(await dialog.findByText(text("success"))).toBeVisible()
        await expect(dialog.getByText("signed-readiness-entry")).toBeVisible()
        expect(dialog.getByRole("button", {name: text("download")})).toBeEnabled()
        expect(dialog.getAllByText(i18n.t("logsScreen.readiness.passed"))).toHaveLength(5)
        expectChecklistAlignment(dialog)
        expect(boundary.calls).toEqual([
            {
                name: "GetReadinessEvents",
                variables: {tenantId: TENANT_ID, officialEventId: EVENT_ID},
                headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ},
            },
            {
                name: "ReadinessCheck",
                variables: {official_event_id: EVENT_ID, tested_event_id: TESTED_ID},
                headers: {"x-hasura-role": IPermissions.READINESS_CHECK},
            },
        ])
    },
}
export const Failed: Story = {
    args: {outcome: "failed"},
    play: async ({canvasElement}) => {
        const dialog = await run(canvasElement)
        await expect(await dialog.findByText(text("failed"))).toBeVisible()
        expect(dialog.getAllByText(i18n.t("logsScreen.readiness.failed"))).toHaveLength(1)
        expect(dialog.queryByText("signed-readiness-entry")).toBeNull()
        expectChecklistAlignment(dialog)
    },
}
export const ServerError: Story = {
    args: {outcome: "error"},
    play: async ({canvasElement}) => {
        const dialog = await run(canvasElement)
        await expect(await dialog.findByText(text("error"))).toBeVisible()
        expect(dialog.queryByText(text("success"))).toBeNull()
    },
}
export const UnsignedPass: Story = {
    args: {outcome: "unsigned"},
    play: ServerError.play,
}
export const Pending: Story = {
    args: {outcome: "pending"},
    play: async ({canvasElement}) => {
        const dialog = await run(canvasElement)
        await expect(dialog.getByRole("status")).toBeVisible()
        expect(dialog.getByRole("combobox")).toBeDisabled()
        expect(dialog.getByRole("button", {name: text("running")})).toBeDisabled()
        expect(dialog.getByRole("button", {name: text("close")})).toBeDisabled()
    },
}
export const EventListError: Story = {
    args: {eventList: "error"},
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: text("action")}))
        await waitFor(() =>
            expect(within(document.body).getByText(text("eventsError"))).toBeVisible()
        )
        expect(within(document.body).getByRole("button", {name: text("run")})).toBeDisabled()
    },
}
export const NoOtherEvents: Story = {
    args: {eventList: "empty"},
    play: async ({canvasElement}) => {
        const dialog = await open(canvasElement)
        await userEvent.click(dialog.getByRole("combobox"))
        await expect(await within(document.body).findByText(text("noEvents"))).toBeVisible()
    },
}
