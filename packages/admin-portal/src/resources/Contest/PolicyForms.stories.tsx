// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react"
import {initCore} from "@sequentech/ui-core"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {
    RecordContextProvider,
    ResourceContextProvider,
    SaveContextProvider,
    type RaRecord,
} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ContestDataForm} from "./EditContestDataForm"
import {ElectionDataForm} from "../Election/ElectionDataForm"

const ELECTION_ID = "33333333-3333-4333-8333-333333333333"
const CONTEST_ID = "44444444-4444-4444-8444-444444444444"
const language = {enabled_language_codes: ["en", "es"], default_language_code: "en"}
const event = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    presentation: {language_conf: language, contest_encryption_policy: "multiple-contests"},
}
const election = {
    id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    status: {allow_tally: "allowed"},
    voting_channels: ["ONLINE"],
    presentation: {
        language_conf: language,
        i18n: {en: {name: "Council election"}, es: {name: "Elección municipal"}},
        contests_order: "alphabetical",
    },
}
const contest = {
    id: CONTEST_ID,
    election_id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    min_votes: 0,
    max_votes: 2,
    winning_candidates_num: 1,
    counting_algorithm: "plurality-at-large",
    presentation: {
        columns: 1,
        i18n: {en: {name: "Council members"}, es: {name: "Miembros del consejo"}},
    },
}
interface Scenario {
    kind: "contest" | "election"
    canEdit: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
const save = fn(async (_values: Record<string, unknown>) => undefined)
function Fixture({kind, canEdit}: Scenario) {
    const auth = useContext(AuthContext)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    tenantId: TENANT_ID,
                    isAuthorized: (_super, _tenant, permission) =>
                        canEdit && ["election-write", "contest-write"].includes(String(permission)),
                }}
            >
                <ResourceContextProvider value={`sequent_backend_${kind}`}>
                    <SaveContextProvider value={{save, saving: false, mutationMode: "pessimistic"}}>
                        <RecordContextProvider value={kind === "contest" ? contest : election}>
                            {kind === "contest" ? <ContestDataForm /> : <ElectionDataForm />}
                        </RecordContextProvider>
                    </SaveContextProvider>
                </ResourceContextProvider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Policy forms",
    component: Fixture,
    args: {kind: "contest", canEdit: true},
    beforeEach: async () => {
        await initCore()
        save.mockClear()
        boundary = graphqlBoundary({})
        data = dataBoundary({
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                const records: Record<string, RaRecord> = {
                    sequent_backend_election_event: event,
                    sequent_backend_election: election,
                    sequent_backend_tenant: {id: TENANT_ID, settings: {languages: ["en", "es"]}},
                    sequent_backend_document: {id: TENANT_ID, name: null},
                }
                const record = records[resource]
                if (!record || record.id !== id) {
                    data.unexpected.push(`${resource}/${id}`)
                    throw new Error("Unexpected record")
                }
                return {data: record as RecordType}
            },
            getList: async <RecordType extends RaRecord>(resource: string) => {
                if (!["sequent_backend_contest", "sequent_backend_candidate"].includes(resource)) {
                    data.unexpected.push(resource)
                    throw new Error("Unexpected list")
                }
                return {data: [] as RecordType[], total: 0}
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
async function choose(canvasElement: HTMLElement, label: string, value: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: label}))
    await userEvent.click(await within(document.body).findByRole("option", {name: value}))
}
export const ContestPoliciesSaveWireValues: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Ballot Design"}))
        await waitFor(() => expect(canvas.getByDisplayValue("Council members")).not.toBeVisible())
        await choose(canvasElement, "Under Vote Policy", "Warn and Alert")
        await choose(canvasElement, "Invalid Vote Policy", "Not Allowed")
        await choose(canvasElement, "Blank Vote Policy", "Not Allowed")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toEqual(
            expect.objectContaining({
                id: CONTEST_ID,
                tenant_id: TENANT_ID,
                election_id: ELECTION_ID,
                presentation: expect.objectContaining({
                    under_vote_policy: "warn-and-alert",
                    invalid_vote_policy: "not-allowed",
                    blank_vote_policy: "not-allowed",
                }),
            })
        )
        expect(boundary.calls).toEqual([])
    },
}
export const ContestLanguageTabs: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByDisplayValue("Council members")).toBeVisible()
        await userEvent.click(canvas.getByRole("tab", {name: "Spanish"}))
        await expect(await canvas.findByDisplayValue("Miembros del consejo")).toBeVisible()
        expect(save).not.toHaveBeenCalled()
    },
}
export const ContestReadOnlyHidesSave: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByDisplayValue("Council members")
        expect(canvas.queryByRole("button", {name: "Save"})).not.toBeInTheDocument()
        expect(save).not.toHaveBeenCalled()
    },
}
export const ElectionPolicySave: Story = {
    parameters: {
        expectedFailure: {
            reason: "The existing JSON configuration editors have low-contrast counts and an unlabeled file input.",
            a11y: ["color-contrast", "label"],
        },
    },
    args: {kind: "election"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByDisplayValue("Council election")
        await userEvent.click(canvas.getByRole("button", {name: "Advanced Configuration"}))
        await waitFor(() => expect(canvas.getByDisplayValue("Council election")).not.toBeVisible())
        await choose(canvasElement, "Allow Tally", "Requires Voting Period End")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toEqual(
            expect.objectContaining({
                id: ELECTION_ID,
                status: {allow_tally: "requires-voting-period-end"},
            })
        )
    },
}
