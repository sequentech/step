/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import CreateEvent, {warningParams} from "./CreateScheduledEvent"
import {adminDateTimeFormat} from "@/components/timezones/timeZoneService"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/VotingChannel"),
    ...jest.requireActual("../../../../ui-core/src/services/timeZones"),
    ...jest.requireActual("../../../../ui-core/src/services/eventTimeZones"),
    ...jest.requireActual("../../../../ui-core/src/types/ScheduledOutcome"),
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
}))
const mockSave = jest.fn().mockResolvedValue({data: {manage_election_dates: {}}})
const mockNotify = jest.fn()
const mockRefetch = jest.fn()
const mockSetValue = jest.fn()
jest.mock("react-hook-form", () => ({useFormContext: () => ({setValue: mockSetValue})}))
const mockT = (key: string) => key
let mockEvent: Record<string, unknown> | undefined
let mockElectionEvent: Record<string, unknown> | undefined
jest.mock("@apollo/client", () => ({
    gql: (s: TemplateStringsArray) => s.join(""),
    useMutation: () => [mockSave],
    useQuery: () => ({data: undefined}),
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockT, i18n: {language: "en"}})}))
jest.mock("@/hooks/useAliasRenderer", () => ({useAliasRenderer: () => () => "El1"}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/components/election/SelectElection", () => ({__esModule: true, default: () => null}))
jest.mock("react-admin", () => ({
    Create: ({children}: React.PropsWithChildren) => children,
    SimpleForm: ({
        children,
        onSubmit,
        toolbar,
    }: React.PropsWithChildren<{onSubmit: () => void; toolbar?: React.ReactNode}>) =>
        require("react").createElement(
            "form",
            {
                onSubmit: (e: React.FormEvent) => {
                    e.preventDefault()
                    onSubmit()
                },
            },
            children,
            toolbar ?? require("react").createElement("button", {type: "submit"}, "Save")
        ),
    Toolbar: ({children}: React.PropsWithChildren) => children,
    SaveButton: ({disabled}: {disabled?: boolean}) =>
        require("react").createElement("button", {type: "submit", disabled}, "Save"),
    useGetOne: (resource: string) => ({
        data:
            resource === "sequent_backend_scheduled_event"
                ? mockEvent
                : resource === "sequent_backend_election_event"
                  ? mockElectionEvent
                  : undefined,
        refetch: mockRefetch,
    }),
    useGetList: () => ({data: []}),
    useNotify: () => mockNotify,
    useRefresh: () => jest.fn(),
}))
const props = {electionEventId: "event", setIsOpenDrawer: jest.fn(), getElectionName: () => "El1"}
beforeEach(() => {
    mockEvent = undefined
    mockElectionEvent = undefined
    mockSave.mockClear()
    mockNotify.mockClear()
})
/** The scheduled time, entered as a wall time in the row's zone (UTC: nothing is configured). */
const enterTime = (local = "2027-01-01T12:00") =>
    fireEvent.change(screen.getByLabelText(/lifecycle.input.scheduledAt/), {target: {value: local}})
const checkbox = (channel: string) =>
    screen.getByRole("checkbox", {name: `common.channel.${channel}`}) as HTMLInputElement

it("offers all four channels and submits online + kiosk by default", async () => {
    render(React.createElement(CreateEvent, props))
    expect(screen.getAllByRole("checkbox")).toHaveLength(4)
    expect(checkbox("online").checked).toBe(true)
    expect(checkbox("kiosk").checked).toBe(true)
    enterTime()
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({votingChannels: ["ONLINE", "KIOSK"]}),
        })
    )
})
it("allows any channel combination but prevents clearing the final channel", async () => {
    render(React.createElement(CreateEvent, props))
    fireEvent.click(checkbox("telephone"))
    fireEvent.click(checkbox("online"))
    fireEvent.click(checkbox("kiosk"))
    expect(checkbox("telephone").disabled).toBe(true)
    expect(mockSetValue).toHaveBeenLastCalledWith("event_payload.voting_channels", ["TELEPHONE"], {
        shouldDirty: true,
    })
    enterTime()
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({votingChannels: ["TELEPHONE"]}),
        })
    )
})
it.each([undefined, null, []])(
    "loads legacy end schedules with default channels (%s)",
    async (channels) => {
        mockEvent = {
            event_processor: "END_VOTING_PERIOD",
            event_payload: {election_id: "election", voting_channels: channels},
            cron_config: {scheduled_date: "2027-01-01T12:00:00Z"},
        }
        render(
            React.createElement(CreateEvent, {
                ...props,
                isEditEvent: true,
                selectedEventId: "schedule",
            })
        )
        fireEvent.click(screen.getByText("Save"))
        await waitFor(() =>
            expect(mockSave).toHaveBeenCalledWith({
                variables: expect.objectContaining({
                    eventProcessor: "END_VOTING_PERIOD",
                    votingChannels: ["ONLINE", "KIOSK"],
                }),
            })
        )
    }
)
it("loads and preserves explicit channels when the edit record arrives asynchronously", async () => {
    const view = render(
        React.createElement(CreateEvent, {...props, isEditEvent: true, selectedEventId: "schedule"})
    )
    mockEvent = {
        event_processor: "END_VOTING_PERIOD",
        event_payload: {voting_channels: ["EARLY_VOTING", "TELEPHONE"]},
        cron_config: {scheduled_date: "2027-01-01T12:00:00Z"},
    }
    view.rerender(
        React.createElement(CreateEvent, {...props, isEditEvent: true, selectedEventId: "schedule"})
    )
    expect(checkbox("online").checked).toBe(false)
    expect(checkbox("early_voting").checked).toBe(true)
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({votingChannels: ["EARLY_VOTING", "TELEPHONE"]}),
        })
    )
})

it("blocks a start schedule that opens online and early voting together", async () => {
    render(React.createElement(CreateEvent, props))
    enterTime()
    fireEvent.click(checkbox("early_voting"))
    expect(screen.getByText("eventsScreen.messages.onlineWithEarlyVoting")).toBeTruthy()
    const save = screen.getByText("Save") as HTMLButtonElement
    expect(save.disabled).toBe(true)
    fireEvent.submit(save.closest("form") as HTMLFormElement)
    await waitFor(() => expect(mockSave).not.toHaveBeenCalled())

    fireEvent.click(checkbox("online"))
    expect(screen.queryByText("eventsScreen.messages.onlineWithEarlyVoting")).toBeNull()
    expect(save.disabled).toBe(false)
})
it("lets an end schedule close online and early voting together", async () => {
    mockEvent = {
        event_processor: "END_VOTING_PERIOD",
        event_payload: {election_id: "election", voting_channels: ["ONLINE", "EARLY_VOTING"]},
        cron_config: {scheduled_date: "2027-01-01T12:00:00Z"},
    }
    render(
        React.createElement(CreateEvent, {...props, isEditEvent: true, selectedEventId: "schedule"})
    )
    expect(screen.queryByText("eventsScreen.messages.onlineWithEarlyVoting")).toBeNull()
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({votingChannels: ["ONLINE", "EARLY_VOTING"]}),
        })
    )
})
it("shows the reason when the API rejects a schedule and lets the admin retry", async () => {
    const reason = "A start voting period schedule cannot open ONLINE and EARLY_VOTING together."
    mockSave.mockRejectedValueOnce({graphQLErrors: [{message: reason}]})
    render(React.createElement(CreateEvent, props))
    enterTime()
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() => expect(mockNotify).toHaveBeenCalledWith(reason, {type: "error"}))
    expect(checkbox("kiosk").disabled).toBe(false)
})

it("saves the wall time and zone with the instant", async () => {
    render(React.createElement(CreateEvent, props))
    enterTime("2028-04-09T00:00")
    fireEvent.click(screen.getByText("Save"))
    // Nothing configured: the row's zone is UTC (design §2 migration default).
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({
                scheduledDate: "2028-04-09T00:00:00Z",
                localDateTime: "2028-04-09T00:00",
                timeZone: "UTC",
            }),
        })
    )
})
it("doesn't save without a scheduled time", async () => {
    render(React.createElement(CreateEvent, props))
    const save = screen.getByText("Save") as HTMLButtonElement
    expect(save.disabled).toBe(true)
    fireEvent.submit(save.closest("form") as HTMLFormElement)
    await waitFor(() => expect(mockSave).not.toHaveBeenCalled())
})
it("keeps an edited row's time as entered, in its own zone", async () => {
    mockElectionEvent = {
        presentation: {timezones: {configured: ["UTC", "Asia/Dubai"], primary: "UTC"}},
    }
    mockEvent = {
        event_processor: "START_VOTING_PERIOD",
        event_payload: {election_id: "election", voting_channels: ["ONLINE"]},
        cron_config: {
            scheduled_date: "2028-04-08T20:00:00Z",
            local: "2028-04-09T00:00",
            timezone: "Asia/Dubai",
        },
    }
    render(
        React.createElement(CreateEvent, {...props, isEditEvent: true, selectedEventId: "schedule"})
    )
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({
                scheduledDate: "2028-04-08T20:00:00Z",
                localDateTime: "2028-04-09T00:00",
                timeZone: "Asia/Dubai",
            }),
        })
    )
})
it("reports the save's warnings without blocking it", async () => {
    mockSave.mockResolvedValueOnce({
        data: {
            manage_election_dates: {
                warnings: [{code: "thirty-day", message_key: "lifecycle.warning.thirtyDay"}],
            },
        },
    })
    render(React.createElement(CreateEvent, props))
    enterTime()
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith(
            "eventsScreen.messages.createSuccess lifecycle.warning.thirtyDay",
            {type: "warning", multiLine: true}
        )
    )
})

it("saves an untouched row's time exactly as stored, seconds included", async () => {
    mockEvent = {
        event_processor: "END_VOTING_PERIOD",
        event_payload: {election_id: null, voting_channels: ["ONLINE"]},
        cron_config: {scheduled_date: "2027-01-01T12:00:30Z"},
    }
    render(
        React.createElement(CreateEvent, {...props, isEditEvent: true, selectedEventId: "schedule"})
    )
    fireEvent.click(checkbox("kiosk"))
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({
                scheduledDate: "2027-01-01T12:00:30Z",
                localDateTime: null,
                timeZone: null,
            }),
        })
    )
})
it("refuses a zone the event doesn't configure", async () => {
    mockEvent = {
        event_processor: "START_VOTING_PERIOD",
        event_payload: {election_id: "election", voting_channels: ["ONLINE"]},
        cron_config: {
            scheduled_date: "2028-04-08T20:00:00Z",
            local: "2028-04-09T00:00",
            timezone: "Asia/Dubai",
        },
    }
    render(
        React.createElement(CreateEvent, {...props, isEditEvent: true, selectedEventId: "schedule"})
    )
    expect(screen.getByText("lifecycle.input.unconfiguredZone")).toBeTruthy()
    const save = screen.getByText("Save") as HTMLButtonElement
    expect(save.disabled).toBe(true)
    fireEvent.submit(save.closest("form") as HTMLFormElement)
    await waitFor(() => expect(mockSave).not.toHaveBeenCalled())
})
it("words a warning's wall times, zone and election", () => {
    const zones = jest.requireActual("../../../../ui-core/src/services/timeZones")
    const text = {
        t: ((key: string, options?: {defaultValue?: string}) =>
            options?.defaultValue ?? key) as never,
        lang: "en",
        formatDateTime: adminDateTimeFormat("en"),
    }
    expect(
        warningParams(
            {
                code: "voting-window-days",
                election_id: "dubai",
                message_key: "eventsScreen.warning.votingWindowDays",
                params: {
                    days: 29,
                    expected: 30,
                    start_local: "2028-04-09T00:00",
                    end_local: "2028-05-07T23:00:30",
                    time_zone: "Asia/Dubai",
                },
            },
            {text, zoneLabel: zones.zoneLabel},
            (id) => (id === "dubai" ? "Dubai PCG" : id)
        )
    ).toEqual({
        election: "Dubai PCG",
        days: 29,
        expected: 30,
        start_local: "Apr 09, 2028, 00:00",
        end_local: "May 07, 2028, 23:00",
        time_zone: "GMT+4",
    })
})
describe("new save warnings", () => {
    const zones = jest.requireActual("../../../../ui-core/src/services/timeZones")
    const text = {
        t: ((key: string, options?: {defaultValue?: string}) =>
            options?.defaultValue ?? key) as never,
        lang: "en",
        formatDateTime: adminDateTimeFormat("en"),
    }
    const service = {text, zoneLabel: zones.zoneLabel}
    const name = (id: string) => (id === "honolulu" ? "Honolulu PCG" : id)

    it("words a close at or before the opening", () => {
        expect(
            warningParams(
                {
                    code: "close-before-open",
                    election_id: "honolulu",
                    message_key: "eventsScreen.warning.closeBeforeOpen",
                    params: {
                        start_local: "2028-05-08T01:00",
                        end_local: "2028-05-08T01:00",
                        time_zone: "Pacific/Honolulu",
                    },
                },
                service,
                name
            )
        ).toEqual({
            election: "Honolulu PCG",
            start_local: "May 08, 2028, 01:00",
            end_local: "May 08, 2028, 01:00",
            time_zone: "HST",
        })
    })

    it("words a short last voting day, hours as numbers", () => {
        expect(
            warningParams(
                {
                    code: "short-last-day",
                    election_id: "honolulu",
                    message_key: "eventsScreen.warning.shortLastDay",
                    params: {
                        hours: 1,
                        minimum_hours: 12,
                        end_local: "2028-05-08T01:00",
                        time_zone: "Pacific/Honolulu",
                    },
                },
                service,
                name
            )
        ).toEqual({
            election: "Honolulu PCG",
            hours: 1,
            minimum_hours: 12,
            end_local: "May 08, 2028, 01:00",
            time_zone: "HST",
        })
    })
})

it("edits the selected row even when its task name is custom", async () => {
    mockEvent = {
        id: "manual-kiosk-stop",
        task_id: "custom-kiosk-stop",
        event_processor: "END_VOTING_PERIOD",
        event_payload: {voting_channels: ["KIOSK"]},
        cron_config: {scheduled_date: "2027-01-01T12:00:00Z"},
    }
    render(
        React.createElement(CreateEvent, {
            ...props,
            isEditEvent: true,
            selectedEventId: "manual-kiosk-stop",
        })
    )
    enterTime()
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() =>
        expect(mockSave).toHaveBeenCalledWith({
            variables: expect.objectContaining({
                scheduledEventId: "manual-kiosk-stop",
                votingChannels: ["KIOSK"],
            }),
        })
    )
})

it("keeps the editor open and displays a schedule conflict returned by Harvest", async () => {
    const close = jest.fn()
    const reason = "Another schedule already targets KIOSK for this action."
    mockSave.mockResolvedValueOnce({data: {manage_election_dates: {error_msg: reason}}})
    render(React.createElement(CreateEvent, {...props, setIsOpenDrawer: close}))
    enterTime()
    fireEvent.click(screen.getByText("Save"))
    await waitFor(() => expect(mockNotify).toHaveBeenCalledWith(reason, {type: "error"}))
    expect(close).not.toHaveBeenCalled()
})
