// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {testDataProvider, type RaRecord} from "react-admin"
import {ConfigureStep} from "./ConfigureStep"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"

const CEREMONY_ID = "44444444-4444-4444-8444-444444444444"
const trustees = ["Alice", "Bob", "Carol"].map((name, index) => ({
    id: `55555555-5555-4555-8555-55555555555${index}`,
    tenant_id: TENANT_ID,
    name,
}))
const electionEvent: Sequent_Backend_Election_Event = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    is_archived: false,
    encryption_protocol: "RistrettoCtx",
    elections: [],
    elections_aggregate: {nodes: []},
    presentation: {ceremonies_policy: "manual-ceremonies"},
}
const ceremony: Sequent_Backend_Keys_Ceremony = {
    id: CEREMONY_ID,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    created_at: "2026-01-01T00:00:00Z",
    last_updated_at: "2026-01-01T00:00:00Z",
    threshold: 2,
    trustee_ids: trustees.map(({id}) => id),
    keys_ceremony_trustee_ids: trustees,
    keys_ceremony_trustee_ids_aggregate: {nodes: trustees},
    execution_status: "STARTED",
    name: "All Elections",
}
let boundary: ReturnType<typeof graphqlBoundary>
let unexpectedReads: string[]
let trusteeRead: ReturnType<typeof fn>
let dataProvider: ReturnType<typeof testDataProvider>
type Props = React.ComponentProps<typeof ConfigureStep> & {outcome: "success" | "permission-error"}

const meta = {
    title: "Admin/Keys ceremony configuration",
    component: ConfigureStep,
    args: {
        currentCeremony: null,
        electionEvent,
        setCurrentCeremony: fn(),
        openCeremonyStep: fn(),
        goBack: fn(),
        outcome: "success",
    },
    beforeEach: ({args}) => {
        unexpectedReads = []
        trusteeRead = fn()
        boundary = graphqlBoundary({
            CreateKeysCeremony: () => ({
                data: {
                    create_keys_ceremony: {
                        keys_ceremony_id: args.outcome === "success" ? CEREMONY_ID : null,
                        error_message:
                            args.outcome === "permission-error" ? "permission-labels" : null,
                    },
                },
            }),
        })
        dataProvider = testDataProvider({
            getList: async <RecordType extends RaRecord>(resource: string, params: unknown) => {
                if (resource === "sequent_backend_trustee") {
                    trusteeRead(params)
                    return {data: trustees as unknown as RecordType[], total: 3}
                }
                if (resource === "sequent_backend_election")
                    return {data: [] as RecordType[], total: 0}
                unexpectedReads.push(resource)
                throw new Error(`Unexpected list: ${resource}`)
            },
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                if (resource === "sequent_backend_keys_ceremony" && id === CEREMONY_ID) {
                    return {data: ceremony as unknown as RecordType}
                }
                unexpectedReads.push(`${resource}/${id}`)
                throw new Error(`Unexpected record: ${resource}/${id}`)
            },
        })
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary} dataProvider={dataProvider}>
            <ConfigureStep {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

async function chooseTrustees(canvasElement: HTMLElement, names = ["Alice", "Bob"]) {
    const canvas = within(canvasElement)
    for (const name of names) await userEvent.click(await canvas.findByRole("checkbox", {name}))
}
async function openConfirmation(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Create Key Ceremony"}))
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    return within(dialog)
}
function assertNoRequests() {
    expect(boundary.calls).toEqual([])
    expect(boundary.unexpected).toEqual([])
    expect(unexpectedReads).toEqual([])
}

export const ManualCeremony: Story = {
    play: async ({canvasElement, args}) => {
        await chooseTrustees(canvasElement)
        const dialog = await openConfirmation(canvasElement)
        expect(boundary.calls).toEqual([])
        await expect(
            dialog.getByText("Are you sure you want to Create Key Ceremony?")
        ).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Yes, Create Key Ceremony"}))
        await waitFor(() => expect(args.openCeremonyStep).toHaveBeenCalledTimes(1))
        expect(args.setCurrentCeremony).toHaveBeenCalledWith(ceremony)
        expect(boundary.calls).toEqual([
            {
                name: "CreateKeysCeremony",
                variables: {
                    electionEventId: EVENT_ID,
                    threshold: 2,
                    trusteeNames: ["Alice", "Bob"],
                    electionId: null,
                    name: "All Elections",
                    isAutomaticCeremony: false,
                },
                headers: {"x-hasura-role": "admin-ceremony"},
            },
        ])
        expect(trusteeRead).toHaveBeenCalledWith(
            expect.objectContaining({
                filter: {tenant_id: TENANT_ID},
                sort: {field: "last_updated_at", order: "DESC"},
                pagination: {page: 1, perPage: 200},
            })
        )
        expect(boundary.unexpected).toEqual([])
        expect(unexpectedReads).toEqual([])
    },
}
export const AutomaticCeremony: Story = {
    args: {
        electionEvent: {
            ...electionEvent,
            presentation: {ceremonies_policy: "automated-ceremonies"},
        },
    },
    play: async ({canvasElement, args}) => {
        await chooseTrustees(canvasElement)
        await userEvent.click(
            within(canvasElement).getByRole("switch", {name: "Automatic Ceremony"})
        )
        const dialog = await openConfirmation(canvasElement)
        await expect(
            dialog.getByText("Are you sure you want to Create Automatic Key Ceremony?")
        ).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Yes, Create Key Ceremony"}))
        await waitFor(() => expect(args.openCeremonyStep).toHaveBeenCalledTimes(1))
        expect(boundary.calls[0].variables).toEqual({
            electionEventId: EVENT_ID,
            threshold: 2,
            trusteeNames: ["Alice", "Bob"],
            electionId: null,
            name: "All Elections",
            isAutomaticCeremony: true,
        })
        expect(boundary.unexpected).toEqual([])
        expect(unexpectedReads).toEqual([])
    },
}

async function assertThresholdError(canvasElement: HTMLElement, threshold: string) {
    const canvas = within(canvasElement)
    await chooseTrustees(canvasElement)
    const input = canvas.getByRole("spinbutton", {name: "Threshold"})
    await userEvent.clear(input)
    await userEvent.type(input, threshold)
    await userEvent.click(canvas.getByRole("button", {name: "Create Key Ceremony"}))
    await expect(
        await canvas.findByText(
            `You selected threshold ${threshold} but it must be between 2 and 3.`
        )
    ).toBeVisible()
    expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
    assertNoRequests()
}
export const ThresholdBelowMinimum: Story = {
    play: async ({canvasElement}) => assertThresholdError(canvasElement, "1"),
}
export const ThresholdAboveTrusteeCount: Story = {
    play: async ({canvasElement}) => assertThresholdError(canvasElement, "4"),
}
export const TooFewSelectedTrustees: Story = {
    play: async ({canvasElement}) => {
        await chooseTrustees(canvasElement, ["Alice"])
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Create Key Ceremony"}))
        await expect(
            await canvas.findByText("You selected only 1 trustee, but you must select at least 2.")
        ).toBeVisible()
        assertNoRequests()
    },
}
export const RaisedThresholdRequiresMoreTrustees: Story = {
    play: async ({canvasElement}) => {
        await chooseTrustees(canvasElement)
        const canvas = within(canvasElement)
        const threshold = canvas.getByRole("spinbutton", {name: "Threshold"})
        await userEvent.clear(threshold)
        await userEvent.type(threshold, "3")
        await userEvent.click(canvas.getByRole("button", {name: "Create Key Ceremony"}))
        await expect(
            await canvas.findByText("You selected only 2 trustees, but you must select at least 3.")
        ).toBeVisible()
        expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        assertNoRequests()
    },
}
export const PermissionLabelsFailure: Story = {
    args: {outcome: "permission-error"},
    play: async ({canvasElement, args}) => {
        await chooseTrustees(canvasElement)
        const dialog = await openConfirmation(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Yes, Create Key Ceremony"}))
        await expect(
            await within(canvasElement).findByText(
                "Cannot create Key Ceremony: one or more permission labels are missing."
            )
        ).toBeVisible()
        expect(args.openCeremonyStep).not.toHaveBeenCalled()
        expect(boundary.calls).toHaveLength(1)
        expect(boundary.unexpected).toEqual([])
        expect(unexpectedReads).toEqual([])
    },
}
export const CancelConfirmation: Story = {
    play: async ({canvasElement, args}) => {
        await chooseTrustees(canvasElement)
        const dialog = await openConfirmation(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await expect(
            within(canvasElement).getByRole("heading", {name: "Create Election Event Key Ceremony"})
        ).toBeVisible()
        expect(args.openCeremonyStep).not.toHaveBeenCalled()
        assertNoRequests()
    },
}

export const ThresholdAtTrusteeCount: Story = {
    play: async ({canvasElement, args}) => {
        await chooseTrustees(canvasElement, ["Alice", "Bob", "Carol"])
        const threshold = within(canvasElement).getByRole("spinbutton", {name: "Threshold"})
        await userEvent.clear(threshold)
        await userEvent.type(threshold, "3")
        const dialog = await openConfirmation(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Yes, Create Key Ceremony"}))
        await waitFor(() => expect(args.openCeremonyStep).toHaveBeenCalledTimes(1))
        expect(boundary.calls[0].variables).toEqual({
            electionEventId: EVENT_ID,
            threshold: 3,
            trusteeNames: ["Alice", "Bob", "Carol"],
            electionId: null,
            name: "All Elections",
            isAutomaticCeremony: false,
        })
        expect(boundary.unexpected).toEqual([])
        expect(unexpectedReads).toEqual([])
    },
}

export const FilteringKeepsSelectedTrustees: Story = {
    play: async ({canvasElement}) => {
        await chooseTrustees(canvasElement)
        const canvas = within(canvasElement)
        const filter = canvas.getByRole("textbox", {name: "Filter Trustees"})
        await userEvent.type(filter, "car")
        await expect(canvas.getByText("Carol")).toBeVisible()
        expect(canvas.queryByRole("checkbox", {name: "Alice"})).not.toBeInTheDocument()
        await userEvent.click(canvas.getByRole("button", {name: "Clear value"}))
        await expect(canvas.getByRole("checkbox", {name: "Alice"})).toBeChecked()
        await expect(canvas.getByRole("checkbox", {name: "Bob"})).toBeChecked()
        assertNoRequests()
    },
}
