// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Identifier} from "react-admin"
import {EVotingStatus} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Ballot_Publication} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {PublishList} from "./PublishList"
import {PublishStatus} from "./EPublishStatus"
import {EPublishType} from "./EPublishType"

interface Scenario {
    roles: string[]
    /** Whether the event has any publication. */
    publications: boolean
    /** Lists the publications of the council election only. */
    election: boolean
    gold: boolean
    reauthenticate: (url: string) => Promise<void>
    onGenerate: () => void
    setBallotPublicationId: (id: Identifier) => void
    onPreview: (id: Identifier) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const PUBLICATION_IDS = {
    first: storyId(8, 1),
    latest: storyId(8, 2),
    otherElection: storyId(8, 3),
}

const publication = (
    id: string,
    createdAt: string,
    overrides: Partial<Sequent_Backend_Ballot_Publication> = {}
) =>
    ({
        id,
        tenant_id: STORY_IDS.tenant,
        election_event_id: EVENT_ID,
        election_id: STORY_IDS.election,
        election_ids: [STORY_IDS.election],
        is_generated: true,
        published_at: null,
        created_at: createdAt,
        created_by_user_id: "admin",
        labels: {},
        annotations: {},
        ...overrides,
    }) as Sequent_Backend_Ballot_Publication

const PUBLICATIONS = [
    publication(PUBLICATION_IDS.first, "2026-01-10T09:00:00Z", {
        published_at: "2026-01-10T09:30:00Z",
    }),
    publication(PUBLICATION_IDS.latest, "2026-01-15T11:00:00Z"),
    publication(PUBLICATION_IDS.otherElection, "2026-01-12T10:00:00Z", {
        election_id: storyId(3, 9),
        election_ids: [storyId(3, 9)],
        published_at: "2026-01-12T10:30:00Z",
    }),
]

const ALL_ROLES = [
    IPermissions.PUBLISH_READ,
    IPermissions.PUBLISH_WRITE,
    IPermissions.PUBLISH_CREATE,
    IPermissions.PUBLISH_CHANGES,
    IPermissions.EE_PUBLISH_VIEW,
    IPermissions.EE_PUBLISH_PREVIEW,
]

const channel = (enabled: boolean) => ({
    is_channel_enabled: enabled,
    status: EVotingStatus.NOT_STARTED,
})

const meta = {
    title: "Admin/Publish/PublishList",
    component: PublishList,
    args: {
        roles: ALL_ROLES,
        publications: true,
        election: false,
        gold: true,
        reauthenticate: fn(async () => {}),
        onGenerate: fn(),
        setBallotPublicationId: fn(),
        onPreview: fn(),
    },
    argTypes: {
        reauthenticate: {table: {disable: true}},
        onGenerate: {table: {disable: true}},
        setBallotPublicationId: {table: {disable: true}},
        onPreview: {table: {disable: true}},
    },
    parameters: {
        expectedFailure: {
            reason: "The view and preview row actions are icon buttons with no accessible name.",
            a11y: ["button-name"],
        },
    },
    beforeEach: ({args}) => {
        sessionStorage.removeItem("pendingPublishAction")
        localStorage.removeItem("electionEventPublishTabIndex")
        data = resourceBoundary({
            sequent_backend_ballot_publication: args.publications ? PUBLICATIONS : [],
        })
        graphql = graphqlBoundary({})
        return () => sessionStorage.removeItem("pendingPublishAction")
    },
    render: (args) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={args.roles}
            auth={{isGoldUser: () => args.gold, reauthWithGold: args.reauthenticate}}
        >
            <PublishList
                status={PublishStatus.Published}
                publishType={args.election ? EPublishType.Election : EPublishType.Event}
                electionStatus={null}
                electionPresentation={null}
                electionEventId={EVENT_ID}
                electionId={args.election ? STORY_IDS.election : undefined}
                canRead
                canWrite
                kioskModeEnabled={channel(false)}
                onlineModeEnabled={channel(true)}
                earlyVotingEnabled={channel(false)}
                telephoneVotingEnabled={channel(false)}
                changingStatus={false}
                onGenerate={args.onGenerate}
                onChangeStatus={() => {}}
                setBallotPublicationId={args.setBallotPublicationId}
                onPreview={args.onPreview}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const row = (canvasElement: HTMLElement, id: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(`^${id} `)})

const cells = (element: HTMLElement) =>
    within(element)
        .getAllByRole("cell")
        .map((cell) => cell.textContent)

const rowAction = (element: HTMLElement, icon: string) =>
    element.querySelector(`.${icon}`)?.closest("button") as HTMLButtonElement

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Publish History")).toBeVisible()
        const latest = await row(canvasElement, PUBLICATION_IDS.latest)
        expect(cells(latest).slice(0, 4)).toEqual([
            PUBLICATION_IDS.latest,
            "",
            "",
            "2026-01-15T11:00:00Z",
        ])
        // Newest first, across every election of the event.
        const ids = canvas
            .getAllByRole("row")
            .slice(1)
            .map((element) => cells(element)[0])
        expect(ids).toEqual([
            PUBLICATION_IDS.latest,
            PUBLICATION_IDS.otherElection,
            PUBLICATION_IDS.first,
        ])
        await expect(canvas.getByRole("button", {name: "Publish Changes"})).toBeVisible()
        expect(data.calls[0].args[1]).toMatchObject({
            filter: {election_event_id: EVENT_ID},
            sort: {field: "created_at", order: "DESC"},
        })
        expect(data.writes).toEqual([])
    },
}

export const ElectionPublications: Story = {
    args: {election: true},
    play: async ({canvasElement}) => {
        await row(canvasElement, PUBLICATION_IDS.latest)
        const ids = within(canvasElement)
            .getAllByRole("row")
            .slice(1)
            .map((element) => cells(element)[0])
        expect(ids).toEqual([PUBLICATION_IDS.latest, PUBLICATION_IDS.first])
        expect(data.calls[0].args[1]).toMatchObject({
            filter: {election_event_id: EVENT_ID, election_id: STORY_IDS.election},
        })
    },
}

export const ViewAPublication: Story = {
    play: async ({canvasElement, args}) => {
        const first = await row(canvasElement, PUBLICATION_IDS.first)
        await userEvent.click(rowAction(first, "publish-visibility-icon"))
        expect(args.setBallotPublicationId).toHaveBeenCalledWith(PUBLICATION_IDS.first)
        expect(args.onPreview).not.toHaveBeenCalled()
    },
}

export const PreviewAPublication: Story = {
    play: async ({canvasElement, args}) => {
        const latest = await row(canvasElement, PUBLICATION_IDS.latest)
        await userEvent.click(rowAction(latest, "publish-preview-icon"))
        expect(args.onPreview).toHaveBeenCalledWith(PUBLICATION_IDS.latest)
        expect(args.setBallotPublicationId).not.toHaveBeenCalled()
    },
}

export const WithoutRowActions: Story = {
    args: {
        roles: [IPermissions.PUBLISH_READ, IPermissions.PUBLISH_WRITE],
    },
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const latest = await row(canvasElement, PUBLICATION_IDS.latest)
        expect(within(latest).queryByRole("button")).toBeNull()
        expect(within(canvasElement).queryByRole("button", {name: "Publish Changes"})).toBeNull()
    },
}

export const GenerateTheFirstPublication: Story = {
    args: {publications: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No Publication Yet.")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Generate Publication"}))
        expect(args.onGenerate).toHaveBeenCalledOnce()
        expect(args.reauthenticate).not.toHaveBeenCalled()
    },
}

export const GenerationAsksForReauthentication: Story = {
    args: {publications: false, gold: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Generate Publication"}))
        await waitFor(() => expect(args.reauthenticate).toHaveBeenCalledOnce())
        // Back to the publish tab of the election event page.
        expect(args.reauthenticate).toHaveBeenCalledWith(expect.stringContaining("tabIndex=8"))
        expect(sessionStorage.getItem("pendingPublishAction")).toBe("true")
        expect(args.onGenerate).not.toHaveBeenCalled()
    },
}

export const WithoutReadPermission: Story = {
    args: {roles: []},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("No Publication Yet.")).toBeVisible()
        expect(canvas.queryByRole("button")).toBeNull()
        expect(data.calls).toEqual([])
    },
}
