// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
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
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Publish} from "./Publish"
import {EPublishType} from "./EPublishType"

const ELECTION_ID = "33333333-3333-4333-8333-333333333333"
const PUBLICATION_ID = "88888888-8888-4888-8888-888888888888"
const TASK_ID = "99999999-9999-4999-8999-999999999999"
const FIXED_TIME = "2026-01-15T12:00:00Z"
const permissions = [
    "publish-read",
    "publish-write",
    "publish-create",
    "publish-changes",
    "election-event-publish-back-button",
]
interface Scenario {
    election: boolean
    large: boolean
    generationFails: boolean
    taskFails: boolean
    publicationFails: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
let completeTask: () => void
let generationFails = false
let publicationFails = false
const record = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    status: {voting_status: "NOT_STARTED"},
    voting_channels: {online: true},
    presentation: {},
}

function Fixture({election}: Scenario) {
    const auth = useContext(AuthContext)
    const settings = useContext(SettingsContext)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    tenantId: TENANT_ID,
                    isAuthenticated: true,
                    isGoldUser: () => true,
                    isAuthorized: (_super, tenant, permission) =>
                        tenant === TENANT_ID &&
                        (Array.isArray(permission) ? permission : [permission]).every((p) =>
                            permissions.includes(p)
                        ),
                }}
            >
                <SettingsContext.Provider
                    value={{
                        ...settings,
                        globalSettings: {...settings.globalSettings, QUERY_POLL_INTERVAL_MS: 50},
                    }}
                >
                    <RecordContextProvider value={record}>
                        <main>
                            <Publish
                                electionEventId={EVENT_ID}
                                electionId={election ? ELECTION_ID : undefined}
                                type={election ? EPublishType.Election : EPublishType.Event}
                            />
                        </main>
                    </RecordContextProvider>
                </SettingsContext.Provider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Publication lifecycle",
    component: Fixture,
    args: {
        election: false,
        large: false,
        generationFails: false,
        taskFails: false,
        publicationFails: false,
    },
    beforeEach: ({args}) => {
        sessionStorage.removeItem("pendingPublishAction")
        generationFails = args.generationFails
        publicationFails = args.publicationFails
        let taskReady = false
        let generated = false
        let published = false
        completeTask = () => {
            taskReady = true
        }
        const publication = () => ({
            id: PUBLICATION_ID,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            election_id: args.election ? ELECTION_ID : null,
            is_generated: generated,
            published_at: published ? FIXED_TIME : null,
            created_at: "2026-01-15T11:00:00Z",
        })
        boundary = graphqlBoundary({
            GenerateBallotPublication: () => {
                if (generationFails) throw new Error("Synthetic publication generation unavailable")
                return {
                    data: {
                        generate_ballot_publication: {
                            ballot_publication_id: PUBLICATION_ID,
                            task_execution: {id: TASK_ID, execution_status: "STARTED", logs: []},
                        },
                    },
                }
            },
            GetTaskById: () => {
                if (taskReady && !args.taskFails) generated = true
                return {
                    data: {
                        sequent_backend_tasks_execution: [
                            {
                                id: TASK_ID,
                                tenant_id: TENANT_ID,
                                election_event_id: EVENT_ID,
                                execution_status: taskReady
                                    ? args.taskFails
                                        ? "FAILED"
                                        : "SUCCESS"
                                    : "STARTED",
                                type: "GENERATE_BALLOT_PUBLICATION",
                                start_at: FIXED_TIME,
                                end_at: taskReady ? FIXED_TIME : null,
                                logs: args.taskFails
                                    ? [{log_text: "Synthetic ballot exceeded maximum size"}]
                                    : [],
                                annotations: {},
                                executed_by_user: null,
                            },
                        ],
                    },
                }
            },
            GetBallotPublicationChange: () => ({
                data: {
                    get_ballot_publication_changes: {
                        previous: {
                            ballot_publication_id: "previous-publication",
                            ballot_styles: {name: "Previous council"},
                        },
                        current: {
                            ballot_publication_id: PUBLICATION_ID,
                            ballot_styles: args.large
                                ? Object.fromEntries(
                                      Array.from({length: 600}, (_, index) => [
                                          `line_${String(index).padStart(3, "0")}`,
                                          `Synthetic candidate ${index}`,
                                      ])
                                  )
                                : {name: "Revised council"},
                        },
                    },
                },
            }),
            PublishBallot: () => {
                if (publicationFails) throw new Error("Synthetic publication write failed")
                published = true
                return {data: {publish_ballot: {ballot_publication_id: PUBLICATION_ID}}}
            },
        })
        data = dataBoundary({
            getList: async <RecordType extends RaRecord>(resource: string) => {
                if (resource !== "sequent_backend_ballot_publication") {
                    data.unexpected.push(resource)
                    throw new Error(`Unexpected list ${resource}`)
                }
                return {
                    data: (published ? [publication()] : []) as RecordType[],
                    total: published ? 1 : 0,
                }
            },
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                if (resource !== "sequent_backend_ballot_publication" || id !== PUBLICATION_ID) {
                    data.unexpected.push(`${resource}/${id}`)
                    throw new Error("Unexpected publication record")
                }
                return {data: publication() as RecordType}
            },
        })
        return () => {
            expect(boundary.unexpected).toEqual([])
            expect(data.unexpected).toEqual([])
            boundary.client.stop()
            sessionStorage.removeItem("pendingPublishAction")
        }
    },
} satisfies Meta<typeof Fixture>
export default meta
type Story = StoryObj<typeof meta>

async function generate(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.click(await canvas.findByRole("button", {name: "Generate Publication"}))
    await waitFor(() => expect(boundary.calls.some(({name}) => name === "GetTaskById")).toBe(true))
    expect(boundary.calls.filter(({name}) => name === "GetBallotPublicationChange")).toEqual([])
    expect(boundary.calls.filter(({name}) => name === "PublishBallot")).toEqual([])
    completeTask()
    await waitFor(() =>
        expect(boundary.calls.some(({name}) => name === "GetBallotPublicationChange")).toBe(true)
    )
    await waitFor(() => expect(canvas.queryByRole("progressbar")).not.toBeInTheDocument())
    return canvas
}

export const EventGenerationAndPublication: Story = {
    play: async ({canvasElement}) => {
        const canvas = await generate(canvasElement)
        await expect(await canvas.findByText(/Revised council/)).toBeVisible()
        const generation = boundary.calls.find(({name}) => name === "GenerateBallotPublication")!
        expect(generation.variables).toEqual({electionEventId: EVENT_ID})
        const changes = boundary.calls.find(({name}) => name === "GetBallotPublicationChange")!
        expect(changes.variables).toEqual({
            electionEventId: EVENT_ID,
            ballotPublicationId: PUBLICATION_ID,
            limit: 50,
        })
        await userEvent.click(canvas.getAllByRole("button", {name: "Publish Changes"}).at(-1)!)
        await waitFor(() =>
            expect(boundary.calls.filter(({name}) => name === "PublishBallot")).toHaveLength(1)
        )
        expect(boundary.calls.find(({name}) => name === "PublishBallot")!.variables).toEqual({
            electionEventId: EVENT_ID,
            ballotPublicationId: PUBLICATION_ID,
        })
        await expect(await canvas.findByText(PUBLICATION_ID)).toBeVisible()
        await expect(await canvas.findByText(FIXED_TIME, {exact: false})).toBeVisible()
    },
}

export const GenerationRequestFailureRestoresList: Story = {
    args: {generationFails: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Generate Publication"}))
        await waitFor(() =>
            expect(
                within(document.body).getByText("Error loading ballot publication")
            ).toBeVisible()
        )
        await expect(
            await canvas.findByRole("button", {name: "Generate Publication"})
        ).toBeEnabled()
        expect(
            boundary.calls.filter(({name}) => name === "GetTaskById" || name === "PublishBallot")
        ).toEqual([])
    },
}
