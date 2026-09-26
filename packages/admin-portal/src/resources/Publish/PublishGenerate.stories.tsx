// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, type Identifier} from "react-admin"
import {EVotingStatus} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord, storyId} from "@/__stories__/fixtures"
import {IPermissions} from "@/types/keycloak"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {PublishGenerate} from "./PublishGenerate"
import {PublishStatus} from "./EPublishStatus"
import {EPublishType} from "./EPublishType"

interface Scenario {
    roles: string[]
    /** Shows a stored publication instead of the changes waiting to be published. */
    readOnly: boolean
    status: PublishStatus
    ballotPublicationId: string | null
    publishError: string | null
    onBack: () => void
    onPublish: () => void
    onGenerate: () => void
    onPreview: (id: Identifier) => void
    onDismissPublishError: () => void
    fetchAllPublishChanges: () => Promise<void>
}

let graphql: ReturnType<typeof graphqlBoundary>

const PUBLICATION_ID = storyId(8, 2)

const ballotStyle = (id: string, ballot: string) => ({
    id,
    tenant_id: STORY_IDS.tenant,
    election_event_id: EVENT_ID,
    election_id: STORY_IDS.election,
    area_id: STORY_IDS.area,
    ballot_eml: ballot,
})

const CHANGES = {
    previous: {
        ballot_publication_id: storyId(8, 1),
        ballot_styles: [ballotStyle(storyId(9, 1), "Council ballot")],
    },
    current: {
        ballot_publication_id: PUBLICATION_ID,
        ballot_styles: [ballotStyle(storyId(9, 2), "Revised council ballot")],
    },
}

const ALL_ROLES = [
    IPermissions.PUBLISH_READ,
    IPermissions.PUBLISH_WRITE,
    IPermissions.PUBLISH_REGENERATE,
    IPermissions.PUBLISH_EXPORT,
    IPermissions.PUBLISH_CHANGES,
    IPermissions.EE_PUBLISH_VIEW,
    IPermissions.EE_PUBLISH_PREVIEW,
    IPermissions.EE_PUBLISH_BACK_BUTTON,
]

const channel = (enabled: boolean) => ({
    is_channel_enabled: enabled,
    status: EVotingStatus.NOT_STARTED,
})

const meta = {
    title: "Admin/Publish/PublishGenerate",
    component: PublishGenerate,
    args: {
        roles: ALL_ROLES,
        readOnly: false,
        status: PublishStatus.Generated,
        ballotPublicationId: PUBLICATION_ID,
        publishError: null,
        onBack: fn(),
        onPublish: fn(),
        onGenerate: fn(),
        onPreview: fn(),
        onDismissPublishError: fn(),
        fetchAllPublishChanges: fn(async () => {}),
    },
    argTypes: {
        status: {control: "select", options: Object.values(PublishStatus)},
        onBack: {table: {disable: true}},
        onPublish: {table: {disable: true}},
        onGenerate: {table: {disable: true}},
        onPreview: {table: {disable: true}},
        onDismissPublishError: {table: {disable: true}},
        fetchAllPublishChanges: {table: {disable: true}},
    },
    parameters: {
        widgets: ["PublishActions", "PublishExport", "DiffView"],
    },
    beforeEach: () => {
        sessionStorage.removeItem("pendingPublishAction")
        graphql = graphqlBoundary({})
        const storyGraphql = graphql
        return () => expect(storyGraphql.unexpected).toEqual([])
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql} roles={args.roles} auth={{isGoldUser: () => true}}>
            <WidgetsContextProvider>
                <RecordContextProvider value={eventRecord()}>
                    <PublishGenerate
                        ballotPublicationId={args.ballotPublicationId}
                        publishType={EPublishType.Event}
                        data={CHANGES}
                        publishError={args.publishError}
                        onDismissPublishError={args.onDismissPublishError}
                        readOnly={args.readOnly}
                        status={args.status}
                        changingStatus={false}
                        electionEventId={EVENT_ID}
                        onBack={args.onBack}
                        onPublish={args.onPublish}
                        onGenerate={args.onGenerate}
                        fetchAllPublishChanges={args.fetchAllPublishChanges}
                        onPreview={args.onPreview}
                        kioskModeEnabled={channel(false)}
                        onlineModeEnabled={channel(true)}
                        earlyVotingEnabled={channel(false)}
                        telephoneVotingEnabled={channel(false)}
                    />
                </RecordContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function diffShown(canvasElement: HTMLElement, title: string) {
    const region = await within(canvasElement).findByRole("region", {name: title})
    await waitFor(() => expect(region).toHaveTextContent("Revised council ballot"))
    return region
}

export const ReviewChanges: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Changes to be Published")).toBeVisible()
        const changes = await diffShown(canvasElement, "Changes to Publish")
        await expect(changes).toHaveTextContent(PUBLICATION_ID)
        await expect(canvas.getByRole("region", {name: "Current"})).toHaveTextContent(
            "Council ballot"
        )
        for (const name of ["Regenerate", "Export", "Back", "Preview", "Publish Changes"]) {
            await expect(canvas.getByRole("button", {name})).toBeEnabled()
        }
        expect(graphql.calls).toEqual([])
    },
}

export const PublishTheChanges: Story = {
    play: async ({canvasElement, args}) => {
        await diffShown(canvasElement, "Changes to Publish")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Publish Changes"}))
        expect(args.onPublish).toHaveBeenCalledOnce()
        expect(args.onGenerate).not.toHaveBeenCalled()
    },
}

export const Publishing: Story = {
    args: {status: PublishStatus.PublishedLoading},
    parameters: {
        expectedFailure: {
            reason: "The publish button's progress spinner has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await diffShown(canvasElement, "Changes to Publish")
        await expect(canvas.getByRole("button", {name: "Publish Changes"})).toBeDisabled()
        await expect(canvas.getByRole("progressbar")).toBeVisible()
    },
}

export const RegenerateAfterConfirming: Story = {
    play: async ({canvasElement, args}) => {
        await diffShown(canvasElement, "Changes to Publish")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Regenerate"}))
        const dialog = await within(document.body).findByRole("dialog")
        await expect(
            within(dialog).getByText(
                "You have clicked on a sensitive action, so we need you to confirm in order to continue"
            )
        ).toBeVisible()
        await userEvent.click(within(dialog).getByRole("button", {name: "Confirm"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(args.onGenerate).toHaveBeenCalledOnce()
        expect(args.onPublish).not.toHaveBeenCalled()
    },
}

export const PreviewTheChanges: Story = {
    play: async ({canvasElement, args}) => {
        await diffShown(canvasElement, "Changes to Publish")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Preview"}))
        expect(args.onPreview).toHaveBeenCalledWith(PUBLICATION_ID)
    },
}

export const PreviewWithoutAPublication: Story = {
    args: {ballotPublicationId: null},
    play: async ({canvasElement, args}) => {
        await diffShown(canvasElement, "Changes to Publish")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Preview"}))
        await waitFor(() =>
            expect(within(document.body).getByText("Error previewing publication")).toBeVisible()
        )
        expect(args.onPreview).not.toHaveBeenCalled()
    },
}

export const GoBack: Story = {
    play: async ({canvasElement, args}) => {
        await diffShown(canvasElement, "Changes to Publish")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Back"}))
        expect(args.onBack).toHaveBeenCalledOnce()
    },
}

export const PublishFailure: Story = {
    args: {publishError: "Synthetic publication write failed"},
    play: async ({canvasElement, args}) => {
        await diffShown(canvasElement, "Changes to Publish")
        const alert = within(canvasElement).getByRole("alert")
        await expect(alert).toHaveTextContent("Error publishing ballot publication")
        await expect(alert).toHaveTextContent("Synthetic publication write failed")
        await userEvent.click(within(alert).getByRole("button", {name: "Close"}))
        expect(args.onDismissPublishError).toHaveBeenCalledOnce()
    },
}

export const ViewAPublication: Story = {
    args: {readOnly: true},
    parameters: {widgets: ["PublishExport", "DiffView"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("View Publication")).toBeVisible()
        await diffShown(canvasElement, "Publication")
        await expect(canvas.getByRole("region", {name: "Previous Publication"})).toHaveTextContent(
            "Council ballot"
        )
        await expect(canvas.getByRole("button", {name: "Export"})).toBeEnabled()
        for (const name of ["Regenerate", "Publish Changes"]) {
            expect(canvas.queryByRole("button", {name})).toBeNull()
        }
    },
}

export const WithoutWritePermission: Story = {
    args: {roles: [IPermissions.PUBLISH_READ]},
    parameters: {widgets: ["PublishActions", "DiffView"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await diffShown(canvasElement, "Changes to Publish")
        for (const name of ["Regenerate", "Export", "Back", "Preview", "Publish Changes"]) {
            expect(canvas.queryByRole("button", {name})).toBeNull()
        }
    },
}
