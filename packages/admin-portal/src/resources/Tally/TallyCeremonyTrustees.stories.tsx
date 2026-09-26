// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord, keysCeremonyRecord, trusteeRecords} from "@/__stories__/fixtures"
import {STORY_TRUSTEE} from "@/__stories__/storyAuth"
import {ITallyTrusteeStatus} from "@/types/ceremonies"
import {
    COUNCIL_ELECTION,
    DEPUTY_ELECTION,
    TallyStoryContext,
    executionStatus,
    tallyExecution,
    tallySession,
} from "./__stories__/TallyFixture"
import {TallyCeremonyTrustees} from "./TallyCeremonyTrustees"
import {
    EStoryPermissions,
    EStoryWorkflow,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Status of the signed-in trustee's key fragment in the tally. */
    trusteeStatus: ITallyTrusteeStatus
    /** Whether the key the trustee uploads matches its fragment. */
    validKey: boolean
    onSetTallyId: (tallyId: string | null) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({onSetTallyId}: Scenario) {
    const {permissions, workflow} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <RecordContextProvider value={eventRecord(workflow)}>
                <TallyStoryContext onSetTallyId={onSetTallyId}>
                    <TallyCeremonyTrustees />
                </TallyStoryContext>
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyCeremonyTrustees",
    component: TallyCeremonyTrustees,
    args: {trusteeStatus: ITallyTrusteeStatus.WAITING, validKey: true, onSetTallyId: fn()},
    argTypes: {
        trusteeStatus: {control: "inline-radio", options: Object.values(ITallyTrusteeStatus)},
        onSetTallyId: {table: {disable: true}},
    },
    // A trustee restores its key fragment while the tally ceremony runs.
    globals: {workflow: EStoryWorkflow.TALLY, permissions: EStoryPermissions.TRUSTEE},
    beforeEach: async ({args}) => {
        const workflow = EStoryWorkflow.TALLY
        data = resourceBoundary({
            sequent_backend_tally_session: [tallySession(workflow)],
            sequent_backend_election: [COUNCIL_ELECTION, DEPUTY_ELECTION],
            sequent_backend_tally_session_execution: [
                tallyExecution(workflow, {
                    status: {
                        ...executionStatus(workflow),
                        trustees: trusteeRecords.map(({name}) => ({
                            name: String(name),
                            status:
                                name === STORY_TRUSTEE
                                    ? args.trusteeStatus
                                    : ITallyTrusteeStatus.WAITING,
                        })),
                    },
                }),
            ],
            sequent_backend_keys_ceremony: [keysCeremonyRecord()],
            sequent_backend_trustee: trusteeRecords,
        })
        graphql = graphqlBoundary(
            {
                RestorePrivateKey: () => ({
                    data: {restore_private_key: {is_valid: args.validKey}},
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const KEY = "c3ludGhldGljLXByaXZhdGUta2V5"

async function uploadKey(canvasElement: HTMLElement) {
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The key drop zone has no file input")
    await userEvent.upload(input, new File([KEY], "trustee1.key", {type: "text/plain"}))
}

const nextButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: i18n.t("tally.common.next")})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("checkbox", {name: "Council"})).toBeDisabled()
        await expect(canvas.getByText(i18n.t("tally.trusteeTitle"))).toBeVisible()
        // The next step needs a verified key.
        await expect(nextButton(canvasElement)).toBeDisabled()
        expect(data.writes).toEqual([])
        expect(graphql.calls).toEqual([])
    },
}

export const RestoreKeyAndContinue: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("checkbox", {name: "Council"})
        await uploadKey(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("keysGeneration.checkStep.verified"))
        ).toBeVisible()
        expect(graphql.calls.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "RestorePrivateKey",
                variables: {
                    electionEventId: EVENT_ID,
                    tallySessionId: STORY_IDS.tallySession,
                    privateKeyBase64: KEY,
                },
            },
        ])
        await userEvent.click(nextButton(canvasElement))
        // The status step lists the tally's trustees instead of the key upload.
        await expect(await canvas.findByRole("row", {name: /trustee2/})).toBeVisible()
        expect(canvas.queryByText(i18n.t("tally.trusteeTitle"))).toBeNull()
    },
}

export const InvalidKeyIsRejected: Story = {
    args: {validKey: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("checkbox", {name: "Council"})
        await uploadKey(canvasElement)
        await expect(
            await canvas.findByText(
                i18n.t("keysGeneration.checkStep.errorUploading", {error: "empty"})
            )
        ).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["RestorePrivateKey"])
        await expect(nextButton(canvasElement)).toBeDisabled()
    },
}

export const KeyAlreadyRestored: Story = {
    args: {trusteeStatus: ITallyTrusteeStatus.KEY_RESTORED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("row", {name: /trustee2/})).toBeVisible()
        expect(canvas.queryByText(i18n.t("tally.trusteeTitle"))).toBeNull()
        expect(canvas.queryByRole("button", {name: i18n.t("tally.common.next")})).toBeNull()
    },
}

export const CancelReturnsToTheTallies: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        // The footer buttons are recreated on each render, so click once the data has loaded.
        await canvas.findByRole("checkbox", {name: "Council"})
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("tally.common.cancel")}))
        await waitFor(() => expect(args.onSetTallyId).toHaveBeenCalledWith(null))
    },
}
