// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useState} from "react"
import type {Meta, StoryObj} from "@storybook/react"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, type RaRecord} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {ElectionEventTallyContext} from "@/providers/ElectionEventTallyProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ETallyType} from "@/types/ceremonies"
import {TallyCeremony} from "./TallyCeremony"

const ELECTION_ID = "33333333-3333-4333-8333-333333333333"
const SECOND_ELECTION_ID = "33333333-3333-4333-8333-333333333334"
const KEYS_ID = "44444444-4444-4444-8444-444444444444"
const TALLY_ID = "55555555-5555-4555-8555-555555555555"
const FIXED_TIME = "2026-01-15T12:00:00Z"
const event = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    presentation: {i18n: {en: {name: "Council event"}}},
}
const closed = {
    is_published: true,
    allow_tally: "requires-voting-period-end",
    voting_status: "CLOSED",
}
interface Scenario {
    status: Record<string, unknown>
    automated: boolean
    initialSession: "STARTED" | "CONNECTED" | null
    failure: boolean
    trustee: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>

function Fixture(args: Scenario) {
    const auth = useContext(AuthContext)
    const settings = useContext(SettingsContext)
    const tally = useContext(ElectionEventTallyContext)
    const [tallyId, setTallyId] = useState<string | null>(args.initialSession ? TALLY_ID : null)
    const [isCreatingType, setCreatingFlag] = useState<ETallyType | null>(
        ETallyType.ELECTORAL_RESULTS
    )
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    isAuthorized: (_super, _tenant, permission) =>
                        permission === "trustee-ceremony" ? args.trustee : false,
                }}
            >
                <SettingsContext.Provider
                    value={{
                        ...settings,
                        globalSettings: {
                            ...settings.globalSettings,
                            QUERY_FAST_POLL_INTERVAL_MS: 3_600_000,
                        },
                    }}
                >
                    <ElectionEventTallyContext.Provider
                        value={{...tally, tallyId, setTallyId, isCreatingType, setCreatingFlag}}
                    >
                        <RecordContextProvider value={event}>
                            <TallyCeremony />
                        </RecordContextProvider>
                    </ElectionEventTallyContext.Provider>
                </SettingsContext.Provider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Tally ceremony",
    component: Fixture,
    args: {status: closed, automated: false, initialSession: null, failure: false, trustee: false},
    beforeEach: ({args}) => {
        const keys = {
            id: KEYS_ID,
            created_at: FIXED_TIME,
            last_updated_at: FIXED_TIME,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            trustee_ids: ["alice", "bob"],
            status: {},
            execution_status: "SUCCESS",
            labels: {},
            annotations: {},
            threshold: 2,
            name: "Council key",
            settings: {policy: args.automated ? "automated-ceremonies" : "manual-ceremonies"},
            is_default: true,
            permission_label: null,
        }
        const elections = [ELECTION_ID, SECOND_ELECTION_ID].map((id, index) => ({
            id,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            keys_ceremony_id: KEYS_ID,
            status: args.status,
            presentation: {i18n: {en: {name: index ? "Deputy election" : "Council election"}}},
        }))
        let session = {
            id: TALLY_ID,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            keys_ceremony_id: KEYS_ID,
            election_ids: [ELECTION_ID, SECOND_ELECTION_ID],
            execution_status: args.initialSession ?? "STARTED",
            threshold: 2,
            tally_type: "ELECTORAL_RESULTS",
            annotations: {},
            configuration: {},
            is_execution_completed: false,
        }
        boundary = graphqlBoundary({
            ListKeysCeremony: () => ({
                data: {list_keys_ceremony: {items: [keys], total: {aggregate: {count: 1}}}},
            }),
            CreateTallyCeremony: ({variables}) => {
                if (args.failure) throw new Error("Synthetic tally service unavailable")
                session = {...session, election_ids: variables.election_ids}
                return {data: {create_tally_ceremony: {tally_session_id: TALLY_ID}}}
            },
            UpdateTallyCeremony: () => {
                if (args.failure) throw new Error("Synthetic tally service unavailable")
                return {data: {update_tally_ceremony: {tally_session_id: TALLY_ID}}}
            },
        })
        data = dataBoundary({
            getList: async <RecordType extends RaRecord>(resource: string) => {
                const records: Record<string, RaRecord[]> = {
                    sequent_backend_election: elections,
                    sequent_backend_contest: [],
                    sequent_backend_tally_session: [],
                    sequent_backend_tally_session_execution: [
                        {
                            id: "execution",
                            status: {
                                trustees: [
                                    {name: "Alice", status: "KEY_CHECKED"},
                                    {name: "Bob", status: "KEY_CHECKED"},
                                ],
                                elections_status: [],
                                logs: [],
                            },
                        },
                    ],
                    sequent_backend_results_event: [],
                    sequent_backend_keys_ceremony: [keys],
                    sequent_backend_trustee: [
                        {id: "alice", name: "Alice"},
                        {id: "bob", name: "Bob"},
                    ],
                }
                if (!(resource in records)) {
                    data.unexpected.push(resource)
                    throw new Error(`Unexpected list ${resource}`)
                }
                return {data: records[resource] as RecordType[], total: records[resource].length}
            },
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                if (resource !== "sequent_backend_tally_session" || id !== TALLY_ID) {
                    data.unexpected.push(`${resource}/${id}`)
                    throw new Error("Unexpected record")
                }
                return {data: session as unknown as RecordType}
            },
        })
        const services = boundary
        const records = data
        return () => {
            expect(services.unexpected).toEqual([])
            expect(records.unexpected).toEqual([])
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>
const mutations = () => boundary.calls.filter((call) => call.name !== "ListKeysCeremony")
async function ready(canvasElement: HTMLElement, label = "Start Tally Ceremony") {
    const canvas = within(canvasElement)
    await canvas.findByRole("row", {name: /Council election/})
    const next = canvas.getByRole("button", {name: label})
    await waitFor(() => expect(next).toBeEnabled())
    return {canvas, next}
}
async function confirm(next: HTMLElement, label = "Ok") {
    await userEvent.click(next)
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    expect(mutations()).toEqual([])
    await userEvent.click(within(dialog).getByRole("button", {name: label}))
}
export const SelectAndCreateManualCeremony: Story = {
    play: async ({canvasElement}) => {
        const {canvas, next} = await ready(canvasElement)
        await userEvent.click(
            within(canvas.getByRole("row", {name: /Deputy election/})).getByRole("checkbox")
        )
        await confirm(next)
        await waitFor(() => expect(mutations()).toHaveLength(1))
        expect(mutations()[0]).toEqual({
            name: "CreateTallyCeremony",
            headers: {},
            variables: {
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                keys_ceremony_id: KEYS_ID,
                election_ids: [ELECTION_ID],
                tally_type: "ELECTORAL_RESULTS",
            },
        })
        expect(boundary.calls[0]).toEqual({
            name: "ListKeysCeremony",
            variables: {tenantId: TENANT_ID, electionEventId: EVENT_ID},
            headers: {"x-hasura-role": "admin-ceremony"},
        })
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await expect(await canvas.findByText("Key fragment import status")).toBeVisible()
        expect(canvas.getByRole("button", {name: "Start Tally"})).toBeDisabled()
    },
}
export const AutomatedConfirmation: Story = {
    args: {automated: true},
    play: async ({canvasElement}) => {
        const {next} = await ready(canvasElement, "Start Tally")
        await userEvent.click(next)
        const dialog = await within(document.body).findByRole("dialog")
        await waitFor(() => expect(dialog).toBeVisible())
        await expect(
            within(dialog).getByText(
                "Select Start Tally to run tally process and display results, or Close to cancel."
            )
        ).toBeVisible()
        await userEvent.click(within(dialog).getByRole("button", {name: "Close"}))
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await waitFor(() => expect(next).toBeEnabled())
        expect(mutations()).toEqual([])
    },
}
export const EmptySelection: Story = {
    play: async ({canvasElement}) => {
        const {canvas, next} = await ready(canvasElement)
        for (const checkbox of canvas.getAllByRole("checkbox")) await userEvent.click(checkbox)
        await expect(await canvas.findByText("Select at least one election.")).toBeVisible()
        expect(next).toBeDisabled()
        expect(mutations()).toEqual([])
    },
}
async function blocked(canvasElement: HTMLElement, reason: string) {
    const canvas = within(canvasElement)
    await expect(await canvas.findByText(reason)).toBeVisible()
    expect(canvas.getByRole("button", {name: "Start Tally Ceremony"})).toBeDisabled()
    expect(mutations()).toEqual([])
}
export const UnpublishedElection: Story = {
    args: {status: {...closed, is_published: false}},
    play: ({canvasElement}) =>
        blocked(canvasElement, "Publish each selected election before creating its tally."),
}
export const ActiveOnlineChannel: Story = {
    args: {status: {...closed, voting_status: "OPEN"}},
    play: ({canvasElement}) =>
        blocked(
            canvasElement,
            "End voting in each selected election and stop its active voting channels before creating the tally."
        ),
}
export const ActiveKioskChannel: Story = {
    args: {status: {...closed, kiosk_voting_status: "PAUSED"}},
    play: ActiveOnlineChannel.play,
}
export const TallyForbidden: Story = {
    args: {status: {...closed, allow_tally: "disallowed"}},
    play: ({canvasElement}) =>
        blocked(canvasElement, "Tallying is disabled for a selected election."),
}
export const ServiceFailureAllowsRetry: Story = {
    args: {failure: true},
    play: async ({canvasElement}) => {
        const {next} = await ready(canvasElement)
        await confirm(next)
        await expect(
            await within(document.body).findByText("Synthetic tally service unavailable")
        ).toBeVisible()
        await waitFor(() => expect(next).toBeEnabled())
        expect(mutations()).toHaveLength(1)
    },
}
export const ConnectedTrusteesCanStart: Story = {
    args: {initialSession: "CONNECTED"},
    play: async ({canvasElement}) => {
        const {canvas, next} = await ready(canvasElement, "Start Tally")
        await expect(await canvas.findByRole("row", {name: /Alice/})).toBeVisible()
        expect(
            canvas.getAllByRole("checkbox").every((input) => (input as HTMLInputElement).disabled)
        ).toBe(true)
        await confirm(next, "Start Tally")
        await waitFor(() => expect(mutations()).toHaveLength(1))
        expect(mutations()[0]).toEqual({
            name: "UpdateTallyCeremony",
            headers: {},
            variables: {
                election_event_id: EVENT_ID,
                tally_session_id: TALLY_ID,
                status: "IN_PROGRESS",
            },
        })
    },
}
export const TrusteeUsesRestrictedRole: Story = {
    args: {trustee: true},
    play: async ({canvasElement}) => {
        await ready(canvasElement)
        expect(boundary.calls[0].headers).toEqual({"x-hasura-role": "trustee-ceremony"})
        expect(mutations()).toEqual([])
    },
}
