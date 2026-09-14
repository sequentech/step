/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import CreateEvent from "./CreateScheduledEvent"

const mockSave = jest.fn().mockResolvedValue({data: {manage_election_dates: {}}})
const mockRefetch = jest.fn()
const mockSetValue = jest.fn()
jest.mock("react-hook-form", () => ({useFormContext: () => ({setValue: mockSetValue})}))
const mockT = (key: string) => key
let mockEvent: Record<string, unknown> | undefined
jest.mock("@apollo/client", () => ({
    gql: (s: TemplateStringsArray) => s.join(""),
    useMutation: () => [mockSave],
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: mockT})}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/components/election/SelectElection", () => ({__esModule: true, default: () => null}))
jest.mock("react-admin", () => ({
    Create: ({children}: React.PropsWithChildren) => children,
    SimpleForm: ({children, onSubmit}: React.PropsWithChildren<{onSubmit: () => void}>) =>
        require("react").createElement(
            "form",
            {
                onSubmit: (e: React.FormEvent) => {
                    e.preventDefault()
                    onSubmit()
                },
            },
            children,
            require("react").createElement("button", {type: "submit"}, "Save")
        ),
    DateTimeInput: () => null,
    useGetOne: () => ({data: mockEvent, refetch: mockRefetch}),
    useNotify: () => jest.fn(),
    useRefresh: () => jest.fn(),
}))
const props = {electionEventId: "event", setIsOpenDrawer: jest.fn(), getElectionName: () => "El1"}
beforeEach(() => {
    mockEvent = undefined
    mockSave.mockClear()
})
const checkbox = (channel: string) =>
    screen.getByRole("checkbox", {name: `common.channel.${channel}`}) as HTMLInputElement

it("offers all four channels and submits online + kiosk by default", async () => {
    render(React.createElement(CreateEvent, props))
    expect(screen.getAllByRole("checkbox")).toHaveLength(4)
    expect(checkbox("online").checked).toBe(true)
    expect(checkbox("kiosk").checked).toBe(true)
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
