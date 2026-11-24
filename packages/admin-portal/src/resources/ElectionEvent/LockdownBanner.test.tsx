/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {LockdownBanner} from "./LockdownBanner"

const prefix = "electionEventScreen.field.lockdownState."
const button = (action: string) => screen.getByRole("button", {name: prefix + action})
const missing = (action: string) => screen.queryByRole("button", {name: prefix + action})
const validEnd = {
    task_id: "tenant_tenant_event_event_END_LOCKDOWN_PERIOD",
    cron_config: {scheduled_date: "2028-05-08T08:00:00+08:00"},
    event_payload: {election_id: null},
}
let mockEnds: {
    data?: Array<{
        task_id?: string
        cron_config?: {scheduled_date: string}
        event_payload?: unknown
    }>
    total?: number
    isSuccess: boolean
}
let mockLogs: {statement_timestamp?: number; details: {locked_down?: boolean; previous?: boolean}}[]
let mockRead: boolean
let mockWrite: boolean
let mockLabels: string[]
let mockCancelling: boolean
const mockReads = jest.fn()
const mockCancel = jest.fn()
const mockNotify = jest.fn()
const mockRefresh = jest.fn()
jest.mock("react-admin", () => ({
    useGetList: (resource: string, ...args: unknown[]) => {
        mockReads(resource, ...args)
        return resource === "electoral_log" ? {data: mockLogs} : mockEnds
    },
    useNotify: () => mockNotify,
    useRefresh: () => mockRefresh,
}))
jest.mock("@apollo/client", () => ({
    gql: (s: TemplateStringsArray) => s.join(""),
    useMutation: () => [mockCancel, {loading: mockCancelling}],
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        tenantId: "tenant",
        get permissionLabels() {
            return mockLabels
        },
        isAuthorized: (_strict: boolean, tenant: string, roles: string | string[]) =>
            tenant === "tenant" && (Array.isArray(roles) ? mockRead : mockWrite),
    }),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {QUERY_POLL_INTERVAL_MS: 0}}),
}))
jest.mock("./useLogsPermissions", () => ({useLogsPermissions: () => ({canReadLogs: mockRead})}))
jest.mock("@sequentech/ui-core", () => ({effectiveTimeZone: () => "Asia/Manila"}))
jest.mock("@/components/timezones/timeZoneService", () => ({
    useTimeZoneService: () => ({formatDateTimeZone: (date: Date) => date.toISOString()}),
}))
jest.mock("@/components/logs/logMessage", () => ({
    logMessage: (log: {details: unknown}) => log.details,
    logDetails: (value: unknown) => value,
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: {date?: string}) =>
            options?.date ? key + " " + options.date : key,
    }),
}))
jest.mock("../ScheduledEvents/CreateScheduledEvent", () => ({
    __esModule: true,
    EventProcessors: {END_LOCKDOWN_PERIOD: "END_LOCKDOWN_PERIOD"},
    default: (props: {
        electionEventId: string
        fixedProcessor: string
        getElectionName: () => string
    }) =>
        require("react").createElement(
            "output",
            {},
            [props.electionEventId, props.fixedProcessor, props.getElectionName()].join(" ")
        ),
}))

const event = {
    id: "event",
    tenant_id: "tenant",
    is_archived: false,
    elections: [],
} as unknown as Sequent_Backend_Election_Event
beforeEach(() => {
    jest.clearAllMocks()
    mockEnds = {data: [], isSuccess: true}
    mockLogs = []
    mockRead = true
    mockWrite = true
    mockLabels = []
    mockCancelling = false
    mockCancel.mockReset().mockResolvedValue({data: {manage_election_dates: {}}})
})

it("only opens the event-wide end form and shows the current audited period", () => {
    mockLogs = [
        {details: {locked_down: true, previous: true}},
        {statement_timestamp: 1720000000, details: {locked_down: true, previous: false}},
    ]
    render(<LockdownBanner event={event} />)
    expect(screen.getByRole("status").textContent).toContain("since 2024-07-03T09:46:40.000Z")
    expect(screen.getByText(prefix + "noScheduledEnd")).toBeTruthy()
    expect(missing("cancelEnd")).toBeNull()
    fireEvent.click(button("scheduleEnd"))
    expect(
        screen.getByText("event END_LOCKDOWN_PERIOD lifecycle.schedule.allElections")
    ).toBeTruthy()
    expect(mockCancel).not.toHaveBeenCalled()
})

it("cancels a valid pending end through the audited scheduling action and refreshes", async () => {
    mockEnds = {data: [validEnd], total: 1, isSuccess: true}
    render(<LockdownBanner event={event} />)
    expect(screen.getByText(prefix + "scheduledEnd 2028-05-08T00:00:00.000Z")).toBeTruthy()
    expect(button("replaceEnd")).toBeTruthy()
    expect(mockReads).toHaveBeenCalledWith(
        "sequent_backend_scheduled_event",
        expect.objectContaining({
            filter: expect.objectContaining({
                tenant_id: "tenant",
                election_event_id: "event",
                event_processor: "END_LOCKDOWN_PERIOD",
            }),
        }),
        expect.objectContaining({enabled: true})
    )
    fireEvent.click(button("cancelEnd"))
    await waitFor(() => expect(mockRefresh).toHaveBeenCalledTimes(1))
    expect(mockCancel).toHaveBeenCalledWith({
        variables: {
            electionEventId: "event",
            electionId: null,
            eventProcessor: "END_LOCKDOWN_PERIOD",
            scheduledDate: null,
        },
    })
    expect(mockNotify).not.toHaveBeenCalled()
})

it.each([
    ["duplicate rows", {data: [validEnd], total: 3}],
    ["wrong task ID", {data: [{...validEnd, task_id: "wrong-task"}]}],
    ["Post payload", {data: [{...validEnd, event_payload: {election_id: "post"}}]}],
    ["non-object payload", {data: [{...validEnd, event_payload: "not-a-payload"}]}],
    ["non-record payload", {data: [{...validEnd, event_payload: []}]}],
    ["instant with no offset", {data: [{cron_config: {scheduled_date: "2028-05-08T08:00:00"}}]}],
    [
        "invalid instant with offset",
        {data: [{cron_config: {scheduled_date: "2028-13-08T08:00:00+08:00"}}]},
    ],
] as const)("offers only audited cancellation for %s", (_reason, fixture) => {
    mockEnds = {...fixture, data: [...fixture.data], isSuccess: true}
    render(<LockdownBanner event={event} />)
    expect(screen.getByText(prefix + "ambiguousEnd")).toBeTruthy()
    expect(missing("scheduleEnd")).toBeNull()
    expect(missing("replaceEnd")).toBeNull()
    expect(button("cancelInvalidEnds")).toBeTruthy()
})

it("accepts the legacy row without task ID or payload and disables double submits", () => {
    mockEnds = {data: [{cron_config: validEnd.cron_config}], isSuccess: true}
    mockCancelling = true
    render(<LockdownBanner event={event} />)
    expect((button("replaceEnd") as HTMLButtonElement).disabled).toBe(true)
    expect((button("cancelEnd") as HTMLButtonElement).disabled).toBe(true)
})

it.each([
    ["not loaded", {isSuccess: false}, false, true, [], []],
    ["not permitted to edit", {isSuccess: true}, false, true, [], []],
    ["archived", {isSuccess: true}, true, true, [], []],
    [
        "missing a Post label",
        {isSuccess: true},
        false,
        true,
        [],
        [{permission_label: "restricted"}],
    ],
    ["no schedule read access", {isSuccess: false}, false, false, [], []],
] as const)("never offers editing when %s", (reason, schedule, archived, read, labels, posts) => {
    mockEnds = schedule
    mockRead = read
    mockWrite = reason !== "not permitted to edit"
    mockLabels = [...labels]
    render(
        <LockdownBanner
            event={{
                ...event,
                is_archived: archived,
                elections: [...posts] as typeof event.elections,
            }}
        />
    )
    expect(missing("scheduleEnd")).toBeNull()
    expect(missing("cancelEnd")).toBeNull()
    expect(mockCancel).not.toHaveBeenCalled()
})

it.each([
    ["application refusal", false, "Cannot cancel this pending end."],
    ["GraphQL refusal", true, "Cannot cancel this pending end."],
    ["missing error detail", true, undefined],
] as const)("surfaces a failed cancel: %s", async (_label, throws, detail) => {
    mockEnds = {data: [validEnd], isSuccess: true}
    if (throws) {
        mockCancel.mockRejectedValueOnce(detail ? {graphQLErrors: [{message: detail}]} : {})
    } else {
        mockCancel.mockResolvedValueOnce({data: {manage_election_dates: {error_msg: detail}}})
    }
    render(<LockdownBanner event={event} />)
    fireEvent.click(button("cancelEnd"))
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith(detail ?? "eventsScreen.messages.editError", {
            type: "error",
        })
    )
})
