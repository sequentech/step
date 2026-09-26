// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionEventTallyContext} from "@/providers/ElectionEventTallyProvider"
import type {IMiruTransmissionPackageData} from "@/types/miru"
import {EditElectionEventTally} from "./EditElectionEventTally"
import {
    answerOrPending,
    paramsOf,
    readsOf,
    recordsOrPending,
} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

/** Which tally screen the tally store selects. */
type View = "list" | "ceremony" | "trustee" | "transmission"

interface Scenario {
    view: View
}

const transmissionPackage: IMiruTransmissionPackageData = {
    election_id: STORY_IDS.election,
    area_id: STORY_IDS.area,
    servers: [],
    documents: [],
    logs: [],
    threshold: 1,
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

function Fixture({view}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const tally = useContext(ElectionEventTallyContext)
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <ElectionEventTallyContext.Provider
                value={{
                    ...tally,
                    tallyId: view === "list" ? null : STORY_IDS.tallySession,
                    isTrustee: view === "trustee",
                    selectedTallySessionData: view === "transmission" ? transmissionPackage : null,
                }}
            >
                <RecordContextProvider value={eventRecord()}>
                    <EditElectionEventTally />
                </RecordContextProvider>
            </ElectionEventTallyContext.Provider>
        </AdminStoryProvider>
    )
}

// The tally list, ceremonies and transmission wizard have their own sections;
// their reads stay loading here.
const meta = {
    title: "Admin/Election event/EditElectionEventTally",
    component: EditElectionEventTally,
    args: {view: "list"},
    argTypes: {
        view: {control: "inline-radio", options: ["list", "ceremony", "trustee", "transmission"]},
    },
    beforeEach: async () => {
        data = recordsOrPending()
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const listHeader = "Election Event Tally"

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText(listHeader)).toBeVisible()
        await waitFor(() =>
            expect(paramsOf(data, "getList", "sequent_backend_tally_session")).toMatchObject({
                filter: {election_event_id: EVENT_ID},
            })
        )
    },
}

export const CeremonyOfTheSelectedTally: Story = {
    args: {view: "ceremony"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Start")).toBeVisible()
        await expect(canvas.getByText("Results")).toBeVisible()
        expect(canvas.queryByText(listHeader)).toBeNull()
        expect(readsOf(data)).not.toContain("getList sequent_backend_tally_session")
    },
}

export const TrusteeCeremony: Story = {
    args: {view: "trustee"},
    globals: {permissions: EStoryPermissions.TRUSTEE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Trustees process")).toBeVisible()
        expect(canvas.queryByText(listHeader)).toBeNull()
        // The trustee's ceremony is that of the selected tally session.
        await waitFor(() =>
            expect(paramsOf(data, "getOne", "sequent_backend_tally_session")).toMatchObject({
                id: STORY_IDS.tallySession,
            })
        )
    },
}

export const TransmissionPackage: Story = {
    args: {view: "transmission"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Allows you to export a Transmission Package to Destination Servers or download it."
            )
        ).toBeVisible()
        expect(canvas.queryByText(listHeader)).toBeNull()
        expect(canvas.queryByText("Trustees process")).toBeNull()
    },
}
