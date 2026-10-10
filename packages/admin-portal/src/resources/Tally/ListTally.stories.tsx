// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, type PropsWithChildren} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {GraphQLError} from "graphql"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord, keysCeremonyRecord, trusteeRecords} from "@/__stories__/fixtures"
import {ElectionEventTallyContext} from "@/providers/ElectionEventTallyProvider"
import {ETallyType, ITallyExecutionStatus} from "@/types/ceremonies"
import {TallyStoryContext, tallyExecution, tallySession} from "./__stories__/TallyFixture"
import {ListTally} from "./ListTally"
import {
    EStoryPermissions,
    EStoryWorkflow,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the event has a tally. */
    empty: boolean
    /** Status of the event's tally session. */
    tallyStatus?: ITallyExecutionStatus
    /** Whether the keys ceremony has completed. */
    keysReady: boolean
    /** Whether listing the keys ceremonies fails. */
    ceremoniesError: boolean
    onSetTallyId: (tallyId: string | null, isTrustee?: boolean) => void
    onSetCreatingFlag: (type: ETallyType | null) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

/** Observes the tally screen's navigation, which the tally screen's provider owns. */
function TallyNavigation({
    onSetTallyId,
    onSetCreatingFlag,
    children,
}: PropsWithChildren<Pick<Scenario, "onSetTallyId" | "onSetCreatingFlag">>) {
    const context = useContext(ElectionEventTallyContext)
    return (
        <ElectionEventTallyContext.Provider
            value={{
                ...context,
                setTallyId: (id, isTrustee) => onSetTallyId(id, isTrustee),
                setCreatingFlag: onSetCreatingFlag,
            }}
        >
            {children}
        </ElectionEventTallyContext.Provider>
    )
}

function Fixture({onSetTallyId, onSetCreatingFlag}: Scenario) {
    const {permissions, workflow} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <RecordContextProvider value={eventRecord(workflow)}>
                <TallyStoryContext>
                    <TallyNavigation
                        onSetTallyId={onSetTallyId}
                        onSetCreatingFlag={onSetCreatingFlag}
                    >
                        <ListTally />
                    </TallyNavigation>
                </TallyStoryContext>
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const listDefects = {
    expectedFailure: {
        reason:
            "The tally row actions are icon buttons named only by a tooltip on their hidden icon, " +
            "and the status chip has white text below 4.5 contrast.",
        a11y: ["button-name", "color-contrast"],
    },
}

const emptyDefects = {
    expectedFailure: {
        reason: "The empty state's create buttons contain icon buttons.",
        a11y: ["nested-interactive"],
    },
}

const meta = {
    title: "Admin/Tally/ListTally",
    component: ListTally,
    args: {
        empty: false,
        keysReady: true,
        ceremoniesError: false,
        onSetTallyId: fn(),
        onSetCreatingFlag: fn(),
    },
    argTypes: {
        tallyStatus: {control: "select", options: Object.values(ITallyExecutionStatus)},
        onSetTallyId: {table: {disable: true}},
        onSetCreatingFlag: {table: {disable: true}},
    },
    parameters: listDefects,
    globals: {workflow: EStoryWorkflow.RESULTS},
    beforeEach: async ({args, globals}) => {
        const {workflow} = readStoryGlobals(globals)
        const session = tallySession(workflow, {
            ...(args.tallyStatus
                ? {
                      execution_status: args.tallyStatus,
                      is_execution_completed: args.tallyStatus === ITallyExecutionStatus.SUCCESS,
                  }
                : {}),
        })
        const ceremony = keysCeremonyRecord(
            args.keysReady ? EStoryWorkflow.ENDED : EStoryWorkflow.CREATED
        )
        data = resourceBoundary({
            sequent_backend_tally_session: args.empty ? [] : [session],
            sequent_backend_tally_session_execution: args.empty ? [] : [tallyExecution(workflow)],
        })
        graphql = graphqlBoundary(
            {
                ListKeysCeremony: () =>
                    args.ceremoniesError
                        ? {errors: [new GraphQLError("Synthetic keys service unavailable")]}
                        : {
                              data: {
                                  list_keys_ceremony: {
                                      items: [ceremony],
                                      total: {aggregate: {count: 1}},
                                  },
                              },
                          },
                TrusteeNames: () => ({
                    data: {
                        sequent_backend_trustee: trusteeRecords.map(({id, name}) => ({id, name})),
                    },
                }),
                UpdateTallyCeremony: ({variables}) => ({
                    data: {update_tally_ceremony: {tally_session_id: variables.tally_session_id}},
                }),
                RecountTallySession: ({variables}) => ({
                    data: {recount_tally_session: {tally_session_id: variables.tally_session_id}},
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

const tallyRow = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("row", {name: new RegExp(STORY_IDS.tallySession)})

/** The row action whose tooltip has this title. */
async function rowAction(canvasElement: HTMLElement, title: string) {
    const row = await tallyRow(canvasElement)
    const icon = within(row).getByLabelText(title)
    return icon.closest("button") as HTMLButtonElement
}

const operations = () => graphql.calls.map(({name}) => name)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const row = within(await tallyRow(canvasElement))
        await expect(
            row.getByText(i18n.t("electionEventScreen.tally.tallyType.ELECTORAL_RESULTS"))
        ).toBeVisible()
        await expect(row.getByText(ITallyExecutionStatus.SUCCESS)).toBeVisible()
        // The tally covers both elections.
        await expect(row.getByText("2")).toBeVisible()
        expect(within(canvasElement).queryByRole("alert")).toBeNull()
        expect(operations()).toEqual(expect.arrayContaining(["ListKeysCeremony", "TrusteeNames"]))
        const listKeys = graphql.calls.find(({name}) => name === "ListKeysCeremony")
        expect(listKeys?.headers).toEqual({"x-hasura-role": "admin-ceremony"})
    },
}

export const ViewTally: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(
            await rowAction(canvasElement, i18n.t("tallysheet.common.tallyCeremony.view"))
        )
        expect(args.onSetTallyId).toHaveBeenCalledWith(STORY_IDS.tallySession, false)
    },
}

export const CancelRunningTally: Story = {
    globals: {workflow: EStoryWorkflow.TALLY},
    play: async ({canvasElement}) => {
        await userEvent.click(
            await rowAction(canvasElement, i18n.t("tallysheet.common.tallyCeremony.cancel"))
        )
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByText(i18n.t("tally.common.dialog.cancelMessage"))).toBeVisible()
        await userEvent.click(
            dialog.getByRole("button", {name: i18n.t("tally.common.dialog.okCancel")})
        )
        await waitFor(() =>
            expect(
                within(document.body).getByText(i18n.t("tally.cancelTallyCeremonySuccess"))
            ).toBeVisible()
        )
        expect(graphql.calls.find(({name}) => name === "UpdateTallyCeremony")?.variables).toEqual({
            election_event_id: EVENT_ID,
            tally_session_id: STORY_IDS.tallySession,
            status: ITallyExecutionStatus.CANCELLED,
        })
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const RecountCompletedTally: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await rowAction(canvasElement, i18n.t("tally.recountTallyCeremony")))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(
            dialog.getByRole("button", {name: i18n.t("tally.recountTallyCeremonyOk")})
        )
        await waitFor(() =>
            expect(
                within(document.body).getByText(i18n.t("tally.recountTallyCeremonySuccess"))
            ).toBeVisible()
        )
        expect(graphql.calls.find(({name}) => name === "RecountTallySession")?.variables).toEqual({
            election_event_id: EVENT_ID,
            tally_session_id: STORY_IDS.tallySession,
        })
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const CreateTally: Story = {
    play: async ({canvasElement, args}) => {
        await tallyRow(canvasElement)
        await userEvent.click(
            within(canvasElement).getByRole("button", {
                name: i18n.t("electionEventScreen.tally.create.createTallyButton"),
            })
        )
        expect(args.onSetCreatingFlag).toHaveBeenCalledWith(ETallyType.ELECTORAL_RESULTS)
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: emptyDefects,
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("electionEventScreen.tally.emptyHeader"))
        ).toBeVisible()
        const create = await canvas.findByRole("button", {
            name: i18n.t("electionEventScreen.tally.create.createInitializationReportButton"),
        })
        await waitFor(() => expect(create).toBeEnabled())
        await userEvent.click(create)
        expect(args.onSetCreatingFlag).toHaveBeenCalledWith(ETallyType.INITIALIZATION_REPORT)
    },
}

export const EmptyBeforeTheKeysCeremony: Story = {
    args: {empty: true, keysReady: false},
    parameters: emptyDefects,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("electionEventScreen.tally.notify.noKeysTally"))
        ).toBeVisible()
        await expect(
            canvas.getByRole("button", {
                name: i18n.t("electionEventScreen.tally.create.createTallyButton"),
            })
        ).toBeDisabled()
    },
}

export const EmptyBeforePublication: Story = {
    args: {empty: true},
    globals: {workflow: EStoryWorkflow.KEYS},
    parameters: emptyDefects,
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                i18n.t("electionEventScreen.tally.notify.noPublication")
            )
        ).toBeVisible()
    },
}

export const TrusteeInvitedToParticipate: Story = {
    globals: {workflow: EStoryWorkflow.TALLY, permissions: EStoryPermissions.TRUSTEE},
    args: {tallyStatus: ITallyExecutionStatus.STARTED},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const link = await canvas.findByText("click on the ceremony's Key Action")
        await expect(canvas.getByRole("alert")).toBeVisible()
        expect(graphql.calls.find(({name}) => name === "ListKeysCeremony")?.headers).toEqual({
            "x-hasura-role": "trustee-ceremony",
        })
        await userEvent.click(link)
        expect(args.onSetTallyId).toHaveBeenCalledWith(STORY_IDS.tallySession, true)
    },
}

export const KeysCeremoniesFailToLoad: Story = {
    args: {ceremoniesError: true},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("Synthetic keys service unavailable")
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("row")).toBeNull()
    },
}

export const WithoutCeremonyPermissions: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    parameters: {
        expectedFailure: {
            reason: "The status chip has white text below 4.5 contrast.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const row = await tallyRow(canvasElement)
        expect(within(row).queryByRole("button")).toBeNull()
        expect(
            within(canvasElement).queryByRole("button", {
                name: i18n.t("electionEventScreen.tally.create.createTallyButton"),
            })
        ).toBeNull()
    },
}
