// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, areaRecords, electionPresentation, electionRecord} from "@/__stories__/fixtures"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IvrApiStatus, IvrEmulatorContext} from "@/providers/IvrEmulatorContextProvider"
import type {IvrEmulatorApi} from "@/services/IvrEmulator"
import {IvrEmulator} from "./IvrEmulator"
import {
    BALLOT_EML,
    IVR_SCRIPT,
    ballotStyleRecord,
    ivrEmulatorApi,
    ivrEvent,
    type IvrSession,
} from "./__stories__/IvrFixture"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** What loading the emulator's WASM module did. */
    apiStatus: IvrApiStatus
    /** Makes the emulated call fail with this message. */
    failure?: string
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let emulator: {api: IvrEmulatorApi; session: IvrSession}

function Fixture({apiStatus}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const api = apiStatus === IvrApiStatus.READY ? emulator.api : undefined
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <IvrEmulatorContext.Provider value={{status: apiStatus, api}}>
                <RecordContextProvider value={ivrEvent()}>
                    <IvrEmulator />
                </RecordContextProvider>
            </IvrEmulatorContext.Provider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/IvrEmulator",
    component: IvrEmulator,
    args: {apiStatus: IvrApiStatus.READY},
    argTypes: {apiStatus: {control: "inline-radio", options: Object.values(IvrApiStatus)}},
    parameters: {widgets: ["ConfigForm", "ConfigFormBody"]},
    beforeEach: async ({args}) => {
        emulator = ivrEmulatorApi({...IVR_SCRIPT, failure: args.failure})
        data = resourceBoundary({
            sequent_backend_area: areaRecords(),
            sequent_backend_election: [
                electionRecord(),
                electionRecord(undefined, {
                    id: STORY_IDS.secondElection,
                    presentation: electionPresentation("Budget election"),
                }),
            ],
            sequent_backend_ballot_style: [ballotStyleRecord],
        })
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const reads = (resource: string) =>
    data.calls.filter(({args}) => args[0] === resource).map(({args}) => args[1])

/** Chooses the area and the election with a ballot style, then starts a call. */
async function startCall(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    const area = await canvas.findByRole("combobox", {name: "Area"})
    await userEvent.type(area, "North district")
    await userEvent.click(
        await within(document.body).findByRole("option", {name: "North district"})
    )
    await userEvent.click(canvas.getByRole("combobox", {name: "Elections"}))
    await userEvent.click(await within(document.body).findByRole("option", {name: "Council"}))
    await userEvent.click(canvas.getByRole("button", {name: "Start new session"}))
}

const transcript = (canvasElement: HTMLElement) =>
    within(canvasElement).findByText("Welcome to the council vote")

export const ConfigurationForm: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Hints")).toBeVisible()
        await expect(
            canvas.getByText("No published ballot styles found matching your selections")
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Start new session"})).toBeDisabled()
        await waitFor(() =>
            expect(reads("sequent_backend_election")).toEqual([
                expect.objectContaining({
                    filter: {tenant_id: TENANT_ID, election_event_id: EVENT_ID},
                    sort: {field: "external_id", order: "DESC"},
                }),
            ])
        )
        // Ballot styles are only read once an area is chosen.
        expect(reads("sequent_backend_ballot_style")).toEqual([])
        expect(emulator.session.configs).toEqual([])
    },
}

export const OnlyElectionsWithBallotStylesAreOffered: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.type(await canvas.findByRole("combobox", {name: "Area"}), "North district")
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "North district"})
        )
        await waitFor(() =>
            expect(reads("sequent_backend_ballot_style").at(-1)).toMatchObject({
                filter: {
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    area_id: STORY_IDS.area,
                },
            })
        )
        await userEvent.click(canvas.getByRole("combobox", {name: "Elections"}))
        const options = await within(document.body).findAllByRole("option")
        expect(options.map((option) => option.textContent)).toEqual(["Council"])
        await userEvent.keyboard("{Escape}")
        await expect(canvas.getByRole("button", {name: "Start new session"})).toBeDisabled()
    },
}

export const CallSession: Story = {
    parameters: {widgets: ["ConfigForm", "EmulatorInterface", "PromptLine"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await startCall(canvasElement)
        await expect(await transcript(canvasElement)).toBeVisible()
        const input = await canvas.findByPlaceholderText(
            "Enter your input (max digits=3, valid inputs=0-9, timeout=10s)"
        )
        await waitFor(() => expect(input).toBeEnabled())
        expect(emulator.session.configs).toEqual([
            {
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                caller_number: "+1234567890",
                contact_id: expect.stringMatching(/^[0-9a-f-]{36}$/),
                blacklisted_numbers: [],
                open_elections: [STORY_IDS.election],
                election_event: JSON.stringify(ivrEvent()),
                ballot_styles: [BALLOT_EML],
            },
        ])
        // Only digits, star and hash are dialled.
        await userEvent.type(input, "1a2*")
        await expect(input).toHaveValue("12*")
        await userEvent.clear(input)
        await userEvent.type(input, "123")
        await userEvent.click(canvas.getByRole("button", {name: "Send DTMF input"}))
        await expect(await canvas.findByText("Voter 123 accepted")).toBeVisible()
        await expect(await canvas.findByText("Disconnected")).toBeVisible()
        await expect(canvas.getByText("Adiós")).toBeVisible()
        await expect(canvas.getByTitle("es-ES, story-voice")).toHaveTextContent("ES")
        expect(emulator.session.inputs).toEqual(["123"])
        await waitFor(() => expect(emulator.session.freed).toBe(1))
    },
}

export const TimeoutRepeatsThePrompt: Story = {
    parameters: {widgets: ["EmulatorInterface", "PromptLine"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await startCall(canvasElement)
        await transcript(canvasElement)
        const timeout = canvas.getByRole("button", {name: "Send timeout"})
        await waitFor(() => expect(timeout).toBeEnabled())
        await userEvent.click(timeout)
        await expect(
            await canvas.findByText("No input received, enter your voter ID")
        ).toBeVisible()
        expect(emulator.session.inputs).toEqual([])
    },
}

export const EndSessionReturnsToTheForm: Story = {
    parameters: {widgets: ["ConfigForm", "EmulatorInterface"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await startCall(canvasElement)
        await transcript(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "End the session"}))
        await expect(await canvas.findByRole("combobox", {name: "Area"})).toBeVisible()
        expect(canvas.queryByText("Welcome to the council vote")).toBeNull()
        // Unmounting the call releases its driver.
        await waitFor(() => expect(emulator.session.freed).toBe(1))
    },
}

export const CallFailure: Story = {
    args: {failure: "Synthetic emulator crash"},
    parameters: {widgets: ["EmulatorInterface"]},
    play: async ({canvasElement}) => {
        await startCall(canvasElement)
        await expect(
            await within(canvasElement).findByText("Error: Synthetic emulator crash")
        ).toBeVisible()
        // The failed call accepts no further input.
        await expect(
            within(canvasElement).getByRole("button", {name: "Send timeout"})
        ).toBeDisabled()
        await expect(
            within(canvasElement).getByRole("button", {name: "Send DTMF input"})
        ).toBeDisabled()
        expect(emulator.session.configs).toHaveLength(1)
        await waitFor(() => expect(emulator.session.freed).toBe(1))
    },
}

export const EmulatorLoading: Story = {
    args: {apiStatus: IvrApiStatus.LOADING},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Loading the emulator system")).toBeVisible()
        expect(canvas.queryByRole("combobox", {name: "Area"})).toBeNull()
        expect(data.calls).toEqual([])
    },
}

export const EmulatorUnavailable: Story = {
    args: {apiStatus: IvrApiStatus.UNAVAILABLE},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                "The emulator system is not available in your environment"
            )
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}

export const EmulatorLoadError: Story = {
    args: {apiStatus: IvrApiStatus.ERROR},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("Error loading the emulator system")
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}
