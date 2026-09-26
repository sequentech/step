// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {
    EResultsPublicationStatus,
    EResultsRouteScope,
    EResultsWebsiteAccess,
    EResultsWebsiteStatus,
    EResultsWebsiteVisibilityScope,
    i18n,
    type IResultsWebsitePolicy,
} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    STORY_SETTINGS,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {FIXED_TIME, STORY_IDS, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Contest, Sequent_Backend_Election} from "@/gql/graphql"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {taskHandler, taskWidgetDefects} from "@/components/tally/__stories__/DownloadFixture"
import {
    COUNCIL_CONTEST,
    COUNCIL_ELECTION,
    DEPUTY_CONTEST,
    DEPUTY_ELECTION,
    TALLY_IDS,
    TallyStoryContext,
    tallyData,
    tallyExecution,
    tallySession,
} from "./__stories__/TallyFixture"
import {ResultsWebsitePublication} from "./ResultsWebsitePublication"
import {EStoryWorkflow} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    policy: IResultsWebsitePolicy | null
    /** Whether the tally has completed with results. */
    tallied: boolean
    /** Whether the event already has a published version. */
    published: boolean
    /** Realm roles of the signed-in user; the admin group's otherwise. */
    roles?: string[]
    /** What the publication service answers. */
    service: "task" | "warning"
}

const PUBLICATION_ID = storyId(9, 7)
const TASK_ID = storyId(9, 8)
const ENABLED_POLICY: IResultsWebsitePolicy = {
    status: EResultsWebsiteStatus.ENABLED,
    access: EResultsWebsiteAccess.PUBLIC,
    visibility_scope: EResultsWebsiteVisibilityScope.FULL_EVENT,
}
const COUNCIL_LABEL = "Council election - Council members"
const DEPUTY_LABEL = "Deputy election - Deputy"

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({policy, tallied, roles}: Scenario) {
    const workflow = tallied ? EStoryWorkflow.RESULTS : EStoryWorkflow.TALLY
    const execution = tallied ? tallyExecution(workflow) : undefined
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <WidgetsContextProvider>
                <TallyStoryContext data={tallied ? tallyData() : null}>
                    <ResultsWebsitePublication
                        tenantId={TENANT_ID}
                        electionEventId={EVENT_ID}
                        tallySession={tallySession(workflow)}
                        tallySessionExecution={execution}
                        resultsEventId={execution?.results_event_id ?? null}
                        contests={[COUNCIL_CONTEST, DEPUTY_CONTEST] as Sequent_Backend_Contest[]}
                        elections={
                            [COUNCIL_ELECTION, DEPUTY_ELECTION] as Sequent_Backend_Election[]
                        }
                        resultsWebsitePolicy={policy}
                    />
                </TallyStoryContext>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/ResultsWebsitePublication",
    component: ResultsWebsitePublication,
    args: {
        policy: ENABLED_POLICY,
        tallied: true,
        published: true,
        roles: ["publish-results-read", "publish-results-write"],
        service: "task",
    },
    argTypes: {
        policy: {table: {disable: true}},
        service: {control: "inline-radio", options: ["task", "warning"]},
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            sequent_backend_tally_results_publication: args.published
                ? [
                      {
                          id: PUBLICATION_ID,
                          tenant_id: TENANT_ID,
                          election_event_id: EVENT_ID,
                          version: 1,
                          publication_status: EResultsPublicationStatus.PUBLISHED,
                          route_scope: EResultsRouteScope.EVENT,
                          route_election_id: null,
                          access: EResultsWebsiteAccess.PUBLIC,
                          published_contest_ids: [STORY_IDS.contest],
                          published_at: FIXED_TIME,
                          revoked_at: null,
                      },
                  ]
                : [],
        })
        graphql = graphqlBoundary(
            {
                ...taskHandler("SUCCESS", "PUBLISH_RESULTS_WEBSITE"),
                PublishResultsWebsite: () => ({
                    data: {
                        publishResultsWebsite:
                            args.service === "task"
                                ? {
                                      publication_id: storyId(9, 9),
                                      task_execution_id: TASK_ID,
                                      publication_status: EResultsPublicationStatus.PUBLISHING,
                                      error_msg: null,
                                  }
                                : {
                                      publication_id: storyId(9, 9),
                                      task_execution_id: TASK_ID,
                                      publication_status: EResultsPublicationStatus.FAILED,
                                      error_msg: "Synthetic publication already running",
                                  },
                    },
                }),
                RevokeResultsPublication: ({variables}) => ({
                    data: {
                        revokeResultsPublication: {
                            publication_id: variables.publication_id,
                            publication_status: EResultsPublicationStatus.REVOKED,
                        },
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const publishButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {
        name: i18n.t("tally.resultsPublication.publishSelectedContests"),
    })

async function publishedRow(canvasElement: HTMLElement) {
    const chip = await within(canvasElement).findByText(EResultsPublicationStatus.PUBLISHED, {
        selector: ".MuiChip-label",
    })
    return chip.closest("tr") as HTMLElement
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("checkbox", {name: COUNCIL_LABEL})).toBeChecked()
        await expect(canvas.getByRole("checkbox", {name: DEPUTY_LABEL})).toBeChecked()
        await expect(
            canvas.getByText(i18n.t("tally.resultsPublication.selectedContestCount", {count: 2}))
        ).toBeVisible()
        await expect(publishButton(canvasElement)).toBeEnabled()
        const row = within(await publishedRow(canvasElement))
        await expect(
            row.getByRole("link", {name: i18n.t("tally.resultsPublication.open")})
        ).toHaveAttribute("href", `${STORY_SETTINGS.RESULTS_PORTAL_URL}/${EVENT_ID}`)
        expect(data.calls.map(({args}) => args[0])).toContain(
            "sequent_backend_tally_results_publication"
        )
        expect(graphql.calls).toEqual([])
    },
}

export const PublishSelectedContests: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("checkbox", {name: DEPUTY_LABEL}))
        await expect(
            canvas.getByText(i18n.t("tally.resultsPublication.selectedContestCount", {count: 1}))
        ).toBeVisible()
        await userEvent.click(publishButton(canvasElement))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(
            dialog.getByRole("button", {
                name: i18n.t("tally.resultsPublication.publishSelectedContests"),
            })
        )
        await expect(
            await within(document.body).findByText(
                i18n.t("tally.resultsPublication.publishStarted")
            )
        ).toBeVisible()
        const publish = graphql.calls.find(({name}) => name === "PublishResultsWebsite")
        expect(publish?.variables).toEqual({
            election_event_id: EVENT_ID,
            tally_session_id: STORY_IDS.tallySession,
            tally_session_execution_id: TALLY_IDS.execution,
            results_event_id: TALLY_IDS.resultsEvent,
            route_scope: EResultsRouteScope.EVENT,
            route_election_id: null,
            election_ids: [STORY_IDS.election, STORY_IDS.secondElection],
            contest_ids: [STORY_IDS.contest],
            access: EResultsWebsiteAccess.PUBLIC,
            visibility_scope: EResultsWebsiteVisibilityScope.FULL_EVENT,
        })
        expect(publish?.headers).toEqual({"x-hasura-role": "publish-results-write"})
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const PublicationServiceWarning: Story = {
    args: {service: "warning"},
    parameters: {
        expectedFailure: {
            reason:
                taskWidgetDefects("FAILED").expectedFailure.reason +
                " The warning notification has white text on the warning orange.",
            a11y: ["button-name", "color-contrast", "nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        await userEvent.click(publishButton(canvasElement))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(
            dialog.getByRole("button", {
                name: i18n.t("tally.resultsPublication.publishSelectedContests"),
            })
        )
        await expect(
            await within(document.body).findByText("Synthetic publication already running")
        ).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["PublishResultsWebsite"])
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const PublishOneElection: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            canvas.getByRole("combobox", {name: i18n.t("tally.resultsPublication.route")})
        )
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: i18n.t("tally.resultsPublication.electionResults"),
            })
        )
        await expect(
            canvas.getByRole("combobox", {name: i18n.t("tally.resultsPublication.election")})
        ).toHaveTextContent("Council election")
        // Only the chosen election's contests can be published on its route.
        await waitFor(() => expect(canvas.queryByRole("checkbox", {name: DEPUTY_LABEL})).toBeNull())
        await expect(canvas.getByRole("checkbox", {name: COUNCIL_LABEL})).toBeChecked()
    },
}

export const RevokePublishedVersion: Story = {
    play: async ({canvasElement}) => {
        const row = within(await publishedRow(canvasElement))
        await userEvent.click(
            row.getByRole("button", {name: i18n.t("tally.resultsPublication.revoke")})
        )
        await expect(
            await within(document.body).findByText(i18n.t("tally.resultsPublication.revoked"))
        ).toBeVisible()
        expect(graphql.calls.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "RevokeResultsPublication",
                variables: {election_event_id: EVENT_ID, publication_id: PUBLICATION_ID},
            },
        ])
    },
}

export const AuthenticatedPolicyLocksAccess: Story = {
    args: {
        policy: {
            status: EResultsWebsiteStatus.ENABLED,
            access: EResultsWebsiteAccess.AUTHENTICATED,
            visibility_scope: EResultsWebsiteVisibilityScope.AREA_BASED,
        },
        published: false,
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const access = canvas.getByRole("combobox", {
            name: i18n.t("tally.resultsPublication.access"),
        })
        await expect(access).toHaveTextContent(
            i18n.t("tally.resultsPublication.authenticatedAccess")
        )
        await expect(access).toHaveAttribute("aria-disabled", "true")
        await expect(
            canvas.getByRole("combobox", {name: i18n.t("tally.resultsPublication.visibility")})
        ).toHaveTextContent(i18n.t("tally.resultsPublication.personalVisibility"))
        await expect(
            await canvas.findByText(i18n.t("tally.resultsPublication.noPublications"))
        ).toBeVisible()
    },
}

export const DisabledPolicy: Story = {
    args: {policy: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("tally.resultsPublication.disabledPolicy"))
        ).toBeVisible()
        await expect(publishButton(canvasElement)).toBeDisabled()
        await publishedRow(canvasElement)
    },
}

export const WaitingForTheTally: Story = {
    args: {tallied: false, published: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("tally.resultsPublication.waitingForTally"))
        ).toBeVisible()
        await expect(
            canvas.getByText(i18n.t("tally.resultsPublication.noTalliedContests"))
        ).toBeVisible()
        await expect(publishButton(canvasElement)).toBeDisabled()
    },
}

export const ReadOnly: Story = {
    args: {roles: ["publish-results-read"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("tally.resultsPublication.writePermissionRequired"))
        ).toBeVisible()
        await expect(publishButton(canvasElement)).toBeDisabled()
        const row = within(await publishedRow(canvasElement))
        await expect(
            row.getByRole("button", {name: i18n.t("tally.resultsPublication.revoke")})
        ).toBeDisabled()
    },
}

export const WithoutPublicationPermissions: Story = {
    args: {roles: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("tally.resultsPublication.readPermissionRequired"))
        ).toBeVisible()
        expect(canvas.queryByRole("table")).toBeNull()
        expect(data.calls).toEqual([])
    },
}
