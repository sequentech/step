/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {EMessageChannel} from "@/types/messaging"
import {SendTemplateAudience} from "./SendTemplateAudience"

let mockAnswer: {data?: unknown; loading?: boolean; error?: Error} = {}
const mockUseQuery = jest.fn()
jest.mock("@apollo/client", () => ({
    gql: jest.fn(() => ({})),
    useQuery: (...args: unknown[]) => {
        mockUseQuery(...args)
        const options = args[1] as {skip?: boolean}
        const polling = {refetch: jest.fn(), startPolling: jest.fn(), stopPolling: jest.fn()}
        return options.skip
            ? {data: undefined, loading: false, ...polling}
            : {...mockAnswer, ...polling}
    },
}))
const mockRecount = jest.fn()
jest.mock("@/hooks/useMessagingVoterCounts", () => ({
    RECOUNT_POLL_MS: 15000,
    useRecountMessagingVoters: () => ({recount: mockRecount, recounting: false}),
}))
jest.mock("@/hooks/useZonedTime", () => ({
    useZonedTime: () => (value?: string | null) => (value ? `zoned ${value}` : ""),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({ChannelLabel: ({label}: {label: string}) => <span>{label}</span>}),
    {virtual: true}
)
let mockRole: string | null = "messaging-config-read"
jest.mock("@/hooks/useMessagingReadRole", () => ({
    useMessagingReadRole: () => mockRole,
    roleContext: (role: string | null) => (role ? {headers: {"x-hasura-role": role}} : undefined),
}))

const renderTable = (props: Partial<React.ComponentProps<typeof SendTemplateAudience>> = {}) =>
    render(
        <SendTemplateAudience
            electionEventId="event-2"
            audienceSelection="ALL_USERS"
            channels={[EMessageChannel.SMS, EMessageChannel.EMAIL]}
            sendsFrom={(channel) => `configured ${channel}`}
            {...props}
        />
    )

const rows = () =>
    screen
        .getAllByRole("row")
        .slice(1)
        .map((row) => row.getAttribute("data-channel"))

beforeEach(() => {
    mockRole = "messaging-config-read"
    mockAnswer = {}
    mockUseQuery.mockReset()
})

describe("Send Notification's each voter's channel", () => {
    it("shows each channel's voters and sender from the preview, and voters with no channel", () => {
        mockAnswer = {
            data: {
                messaging_audience_preview: {
                    total: 12507,
                    will_send: 10003,
                    status: {
                        computed_at: "2026-10-09T08:00:00Z",
                        pending: false,
                        outcome: "COUNTED",
                    },
                    channels: [
                        {
                            channel: "WHATSAPP",
                            voters: 12500,
                            not_offered: 2500,
                            sends_from: "ACCOUNT",
                            account_name: "Staff Association WhatsApp",
                        },
                        {channel: "VIBER", voters: 1, not_offered: 0, sends_from: "NONE"},
                        {channel: "SMS", voters: 2, not_offered: 0, sends_from: "DEPLOYMENT"},
                        {channel: null, voters: 1, not_offered: 0},
                    ],
                },
            },
        }
        renderTable()
        expect(rows()).toEqual(["WHATSAPP", "VIBER", "SMS", "NONE"])
        const whatsapp = within(screen.getAllByRole("row")[1])
        expect(whatsapp.getByText("Staff Association WhatsApp")).toBeInTheDocument()
        expect(whatsapp.getByText("12,500")).toBeInTheDocument()
        expect(
            whatsapp.getByText(/messaging.audience.notOffered .*"notOffered":"2,500"/)
        ).toBeInTheDocument()
        const viber = within(screen.getAllByRole("row")[2])
        expect(viber.getByText("messaging.send.noAccount")).toBeInTheDocument()
        expect(viber.getByText(/messaging.audience.noAccountHelp/)).toBeInTheDocument()
        const sms = within(screen.getAllByRole("row")[3])
        expect(sms.getByText("messaging.send.environment")).toBeInTheDocument()
        const none = within(screen.getAllByRole("row")[4])
        expect(none.getByText("messaging.reason.NO_CONTACT_CHANNEL")).toBeInTheDocument()
        expect(none.getByText("messaging.audience.notSent")).toBeInTheDocument()
        expect(screen.getByText('messaging.audience.total {"total":"12,507"}')).toBeInTheDocument()
        expect(
            screen.getByText('messaging.audience.willSend {"willSend":"10,003"}')
        ).toBeInTheDocument()
        expect(
            screen.getByText('messaging.voterCount.countedAt {"time":"zoned 2026-10-09T08:00:00Z"}')
        ).toBeInTheDocument()
        screen.getByRole("button", {name: "messaging.voterCount.recount"}).click()
        expect(mockRecount).toHaveBeenCalled()
    })

    it("says plainly that nothing is sent on a channel no voter of the send may get (R8-08)", () => {
        mockAnswer = {
            data: {
                messaging_audience_preview: {
                    total: 3,
                    will_send: 0,
                    status: {
                        computed_at: "2026-10-09T08:00:00Z",
                        pending: false,
                        outcome: "COUNTED",
                    },
                    channels: [{channel: "EMAIL", voters: 3, not_offered: 3}],
                },
            },
        }
        renderTable()
        const email = within(screen.getAllByRole("row")[1])
        expect(email.getByText("messaging.audience.notSent")).toBeInTheDocument()
        expect(email.getByText(/messaging.audience.notOfferedRow/)).toBeInTheDocument()
        expect(email.queryByText(/messaging.audience.notOffered /)).toBeNull()
        expect(email.queryByText("messaging.send.environment")).toBeNull()
        expect(email.queryByText("messaging.send.noAccount")).toBeNull()
    })

    it("shows a deleted configured account as deleted, never as the deployment's sender (R8-05)", () => {
        mockAnswer = {
            data: {
                messaging_audience_preview: {
                    total: 2,
                    will_send: 0,
                    status: {
                        computed_at: "2026-10-09T08:00:00Z",
                        pending: false,
                        outcome: "COUNTED",
                    },
                    channels: [
                        {
                            channel: "SMS",
                            voters: 2,
                            not_offered: 0,
                            sends_from: "ACCOUNT_MISSING",
                            account_id: "deleted-1",
                        },
                    ],
                },
            },
        }
        renderTable()
        const sms = within(screen.getAllByRole("row")[1])
        expect(sms.getByText("messaging.audience.deletedAccount")).toBeInTheDocument()
        expect(sms.getByText(/messaging.audience.noAccountHelp/)).toBeInTheDocument()
        expect(sms.queryByText("messaging.send.environment")).toBeNull()
    })

    it("says when the selected voters took too long to count, and offers no recount", () => {
        mockAnswer = {
            data: {
                messaging_audience_preview: {
                    total: 0,
                    will_send: 0,
                    status: {computed_at: null, pending: false, outcome: "TIMED_OUT"},
                    channels: [],
                },
            },
        }
        renderTable({audienceSelection: "SELECTED", voterIds: ["v1"]})
        expect(screen.getByText("messaging.audience.selectedTimedOut")).toBeInTheDocument()
        expect(rows()).toEqual(["SMS", "EMAIL"])
        expect(screen.queryByRole("button", {name: "messaging.voterCount.recount"})).toBeNull()
    })

    it("keeps the channels and their sender, with no counts, when the preview fails", () => {
        mockAnswer = {error: new Error("refused"), loading: false}
        renderTable()
        expect(rows()).toEqual(["SMS", "EMAIL"])
        expect(screen.getAllByRole("cell", {name: "–"})).toHaveLength(2)
        expect(screen.getByText("configured SMS")).toBeInTheDocument()
        expect(screen.getByText("messaging.audience.loadError")).toBeInTheDocument()
    })

    it("asks for the selected voters only, and nothing without them or the read role", () => {
        renderTable({audienceSelection: "SELECTED", voterIds: ["v1", 2]})
        expect(mockUseQuery.mock.calls[0][1]).toMatchObject({
            variables: {
                electionEventId: "event-2",
                audienceSelection: "SELECTED",
                voterIds: ["v1", "2"],
            },
            skip: false,
            context: {headers: {"x-hasura-role": "messaging-config-read"}},
        })
        mockUseQuery.mockReset()
        renderTable({audienceSelection: "SELECTED", voterIds: []})
        expect(mockUseQuery.mock.calls[0][1]).toMatchObject({skip: true})
        mockUseQuery.mockReset()
        mockRole = null
        renderTable()
        expect(mockUseQuery.mock.calls[0][1]).toMatchObject({skip: true})
    })
})
