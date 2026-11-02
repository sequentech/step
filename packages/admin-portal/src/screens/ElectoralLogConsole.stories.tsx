// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {groupRoles} from "@/__stories__/storyAuth"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    STORY_IDS,
    electionPresentation,
    electionRecord,
    eventPresentation,
    eventRecord,
    storyId,
} from "@/__stories__/fixtures"
import type {ConsolePage, ConsoleQueryResult, ConsoleTenants} from "@/services/ElectoralLogConsole"
import {ElectoralLogConsole} from "./ElectoralLogConsole"
import {EStoryPermissions, useStoryGlobals} from "../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** A user of the super-admin tenant, who browses any tenant and queries. */
    superAdmin: boolean
    /** Whether Harvest refuses to read the electoral log. */
    readFails: boolean
    /** Whether usernames, addresses and countries come back hidden. */
    personalData: "shown" | "hidden"
    /** What the event's electoral-log database answers to a query. */
    queryAnswer: "rows" | "truncated" | "error"
}

const OTHER_TENANT = storyId(1, 2)
const OTHER_EVENT = storyId(2, 3)
const NEXT_PAGE = "position:2"
const QUERY_SQL = `SELECT statement_kind, count(*) AS records
FROM electoral_log_messages
GROUP BY statement_kind
ORDER BY records DESC`

const TENANTS: ConsoleTenants = {
    tenants: [
        {
            id: TENANT_ID,
            slug: "example-council",
            events: [
                {
                    id: EVENT_ID,
                    is_archived: false,
                    presentation: eventPresentation,
                    elections: [
                        {
                            id: STORY_IDS.election,
                            presentation: electionPresentation("Council election"),
                        },
                    ],
                },
            ],
        },
        {
            id: OTHER_TENANT,
            slug: "harbour-board",
            events: [
                {
                    id: OTHER_EVENT,
                    is_archived: false,
                    presentation: {i18n: {en: {name: "Harbour board vote", alias: "Harbour"}}},
                    elections: [],
                },
            ],
        },
    ],
}

const RECORD_COLUMNS = [
    "position",
    "created",
    "statement_kind",
    "election_id",
    "user_id",
    "username",
]

const recordsPage = (after: string | null, personalData: Scenario["personalData"]) => {
    const username = (name: string) => (personalData === "shown" ? name : "hidden")
    const rows =
        after === NEXT_PAGE
            ? [[1, "2026-01-01T09:00:00Z", "KeyGeneration", null, null, null]]
            : [
                  [
                      3,
                      "2026-01-01T09:20:00Z",
                      "CastVote",
                      STORY_IDS.election,
                      STORY_IDS.user,
                      username("alice"),
                  ],
                  [
                      2,
                      "2026-01-01T09:10:00Z",
                      "SendCommunications",
                      null,
                      STORY_IDS.secondUser,
                      username("bob"),
                  ],
              ]
    return {
        columns: RECORD_COLUMNS,
        rows,
        next: after === NEXT_PAGE ? null : NEXT_PAGE,
        estimated_rows: 3,
        personal_columns: ["username"],
        personal_data: personalData,
    } satisfies ConsolePage
}

const ballotsPage: ConsolePage = {
    columns: ["ballot_id", "election_id", "area_id", "status"],
    rows: [["b3f1c2", STORY_IDS.election, STORY_IDS.area, "valid"]],
    next: null,
    estimated_rows: 1,
    personal_columns: [],
    personal_data: "shown",
}

const RECORD = {
    position: 3,
    created: "2026-01-01T09:20:00Z",
    statement_kind: "CastVote",
    username: "alice",
    personal_data: "shown",
    message: {statement: {head: {event_type: "CAST_VOTE", description: "Ballot cast"}}},
}

const queryAnswer = (answer: Scenario["queryAnswer"]): ConsoleQueryResult =>
    answer === "error"
        ? {error: 'ERROR: relation "electoral_log" does not exist'}
        : {
              columns: ["statement_kind", "records"],
              rows: [
                  ["CastVote", 2],
                  ["KeyGeneration", 1],
              ],
              truncated: answer === "truncated",
              elapsed_ms: 12,
          }

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const READ_ROLE = {"x-hasura-role": "electoral-log-console-read"}
const QUERY_ROLE = {"x-hasura-role": "electoral-log-console-query"}

function Fixture({superAdmin}: Scenario) {
    const {permissions} = useStoryGlobals()
    const roles = groupRoles(permissions)
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            auth={
                superAdmin
                    ? {
                          // A super-admin tenant user passes the checks of any tenant.
                          isAuthorized: (checkSuperAdmin, someTenantId, permission) =>
                              (checkSuperAdmin || someTenantId === TENANT_ID) &&
                              [permission].flat().some((role) => roles.includes(role)),
                      }
                    : undefined
            }
        >
            <ElectoralLogConsole />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Screens/ElectoralLogConsole",
    component: ElectoralLogConsole,
    args: {superAdmin: false, readFails: false, personalData: "shown", queryAnswer: "rows"},
    argTypes: {
        personalData: {control: "inline-radio", options: ["shown", "hidden"]},
        queryAnswer: {control: "inline-radio", options: ["rows", "truncated", "error"]},
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            sequent_backend_election_event: [eventRecord()],
            sequent_backend_election: [
                electionRecord(),
                electionRecord(undefined, {
                    id: STORY_IDS.secondElection,
                    presentation: electionPresentation("Deputy election"),
                }),
            ],
        })
        graphql = graphqlBoundary(
            {
                ElectoralLogConsoleTenants: () => ({
                    data: {electoral_log_console_tenants: TENANTS},
                }),
                ElectoralLogConsolePage: ({variables}) =>
                    args.readFails
                        ? {errors: [new GraphQLError("Synthetic electoral log unavailable")]}
                        : {
                              data: {
                                  electoral_log_console_page:
                                      variables.table === "ballots"
                                          ? ballotsPage
                                          : recordsPage(variables.after, args.personalData),
                              },
                          },
                ElectoralLogConsoleRecord: ({variables}) => ({
                    data: {electoral_log_console_record: {...RECORD, position: variables.position}},
                }),
                ElectoralLogConsoleQuery: () => ({
                    data: {electoral_log_console_query: queryAnswer(args.queryAnswer)},
                }),
            },
            {schema: true}
        )
        await graphql.ready
        const boundary = graphql
        return () => expect(boundary.unexpected).toEqual([])
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const pageReads = () => graphql.calls.filter(({name}) => name === "ElectoralLogConsolePage")
const lastPageRead = () => pageReads().at(-1)

/** Picks an option of the labelled select. */
async function choose(canvasElement: HTMLElement, label: string, option: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: label}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
}

const recordsGridDefect = {
    expectedFailure: {
        reason: "The records grid's view column has an empty header.",
        a11y: ["empty-table-header"],
    },
}

const recordsShown = (canvasElement: HTMLElement) =>
    within(canvasElement).findByText("SendCommunications")

export const Populated: Story = {
    parameters: {...recordsGridDefect, widgets: ["TablesTab", "FilterInput"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await recordsShown(canvasElement)).toBeVisible()
        await expect(canvas.getByRole("heading", {name: "Electoral Log"})).toBeVisible()
        expect(canvas.getAllByRole("tab").map(({textContent}) => textContent)).toEqual(["Tables"])
        await expect(canvas.getByRole("combobox", {name: "Election Event"})).toHaveTextContent(
            "Council"
        )
        await expect(canvas.getByText("alice")).toBeVisible()
        await expect(canvas.getByText("About 3 rows in this table before filters.")).toBeVisible()
        for (const filter of ["Kind", "User or Voter ID", "Ballot ID"]) {
            await expect(canvas.getByRole("textbox", {name: filter})).toHaveValue("")
        }
        // An ordinary admin browses the events of their own tenant, which Hasura lists.
        expect(graphql.calls.map(({name}) => name)).not.toContain("ElectoralLogConsoleTenants")
        expect(data.calls.find(({method}) => method === "getList")?.args).toEqual([
            "sequent_backend_election_event",
            expect.objectContaining({filter: {tenant_id: TENANT_ID}}),
        ])
        expect(pageReads()).toEqual([
            {
                name: "ElectoralLogConsolePage",
                variables: {
                    electionEventId: EVENT_ID,
                    table: "records",
                    filters: {},
                    order: "newest-first",
                    after: null,
                    limit: 25,
                },
                headers: READ_ROLE,
            },
        ])
    },
}

export const OpenARecord: Story = {
    parameters: {...recordsGridDefect, widgets: ["TablesTab", "RecordDialog"]},
    play: async ({canvasElement}) => {
        await recordsShown(canvasElement)
        const row = within(canvasElement).getByText("alice").closest("[role=row]") as HTMLElement
        await userEvent.click(within(row).getByRole("button", {name: "View record"}))
        const dialog = within(await within(document.body).findByRole("dialog", {name: "Record 3"}))
        await expect(await dialog.findByText("statement_kind")).toBeVisible()
        await expect(dialog.getByText("Message (decoded)")).toBeVisible()
        await expect(dialog.getByText(/"description": "Ballot cast"/)).toBeVisible()
        await expect(dialog.getByRole("button", {name: "Copy JSON"})).toBeEnabled()
        expect(graphql.calls.find(({name}) => name === "ElectoralLogConsoleRecord")).toEqual({
            name: "ElectoralLogConsoleRecord",
            variables: {electionEventId: EVENT_ID, position: 3},
            headers: READ_ROLE,
        })
        await userEvent.click(dialog.getByRole("button", {name: "Close"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const FilterTheRecords: Story = {
    parameters: {...recordsGridDefect, widgets: ["FilterInput"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await recordsShown(canvasElement)
        await userEvent.type(canvas.getByRole("textbox", {name: "Kind"}), "CastVote")
        await choose(canvasElement, "Election", "Deputy")
        await userEvent.click(canvas.getByRole("button", {name: "Apply Filters"}))
        await waitFor(() =>
            expect(lastPageRead()?.variables).toMatchObject({
                filters: {statement_kind: "CastVote", election_id: STORY_IDS.secondElection},
                after: null,
            })
        )
        await userEvent.click(canvas.getByRole("button", {name: "Clear"}))
        await waitFor(() => expect(lastPageRead()?.variables).toMatchObject({filters: {}}))
        await expect(canvas.getByRole("textbox", {name: "Kind"})).toHaveValue("")
    },
}

/** The ballot box keys areas by UUID: a filter that is not one is never sent. */
export const InvalidBallotFilter: Story = {
    parameters: {widgets: ["FilterInput"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await recordsShown(canvasElement)
        await choose(canvasElement, "Table", "Ballots")
        await expect(await canvas.findByText("b3f1c2")).toBeVisible()
        const area = canvas.getByRole("textbox", {name: "Area ID"})
        await userEvent.type(area, "north")
        await choose(canvasElement, "Status", "Valid")
        const reads = pageReads().length
        await userEvent.click(canvas.getByRole("button", {name: "Apply Filters"}))
        await expect(
            canvas.getByText(
                "Check the highlighted filters: dates must be valid, and the ballot box's area IDs must be UUIDs."
            )
        ).toBeVisible()
        await expect(area).toHaveAttribute("aria-invalid", "true")
        expect(pageReads()).toHaveLength(reads)
        await userEvent.clear(area)
        await userEvent.type(area, STORY_IDS.area)
        await userEvent.click(canvas.getByRole("button", {name: "Apply Filters"}))
        await waitFor(() =>
            expect(lastPageRead()?.variables).toMatchObject({
                table: "ballots",
                filters: {area_id: STORY_IDS.area, status: "valid"},
            })
        )
        await expect(area).toHaveAttribute("aria-invalid", "false")
    },
}

export const NextPage: Story = {
    parameters: recordsGridDefect,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await recordsShown(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Go to next page"}))
        await expect(await canvas.findByText("KeyGeneration")).toBeVisible()
        expect(lastPageRead()?.variables).toMatchObject({after: NEXT_PAGE})
        await userEvent.click(canvas.getByRole("button", {name: "Go to previous page"}))
        await expect(await recordsShown(canvasElement)).toBeVisible()
        expect(lastPageRead()?.variables).toMatchObject({after: null})
        await choose(canvasElement, "Order", "Oldest first")
        await waitFor(() =>
            expect(lastPageRead()?.variables).toMatchObject({order: "oldest-first", after: null})
        )
    },
}

export const PersonalDataHidden: Story = {
    args: {personalData: "hidden"},
    parameters: recordsGridDefect,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await recordsShown(canvasElement)
        await expect(
            canvas.getByText(
                "Usernames, IP addresses and countries show as hidden: seeing them needs the electoral-log-personal-data-read permission."
            )
        ).toBeVisible()
        expect(canvas.queryByText("alice")).toBeNull()
        expect(canvas.getAllByText("hidden")).toHaveLength(2)
    },
}

export const ReadFails: Story = {
    args: {readFails: true},
    parameters: {
        expectedFailure: {
            reason: "The empty grid's grid role owns no rows.",
            a11y: ["aria-required-children"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Synthetic electoral log unavailable")).toBeVisible()
        expect(canvas.queryByText("SendCommunications")).toBeNull()
        expect(canvas.queryByText(/rows in this table/)).toBeNull()
    },
}

/** A super-admin tenant user picks the tenant whose events to browse. */
export const SuperAdminChoosesATenant: Story = {
    args: {superAdmin: true},
    parameters: {...recordsGridDefect, widgets: ["TablesTab"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await recordsShown(canvasElement)
        expect(canvas.getAllByRole("tab").map(({textContent}) => textContent)).toEqual([
            "Tables",
            "Query",
        ])
        await expect(canvas.getByRole("combobox", {name: "Tenant"})).toHaveTextContent(
            "example-council"
        )
        expect(graphql.calls.find(({name}) => name === "ElectoralLogConsoleTenants")).toEqual({
            name: "ElectoralLogConsoleTenants",
            variables: {},
            headers: READ_ROLE,
        })
        expect(lastPageRead()?.variables).toMatchObject({
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
        })
        await choose(canvasElement, "Tenant", "harbour-board")
        await waitFor(() =>
            expect(lastPageRead()?.variables).toMatchObject({
                tenantId: OTHER_TENANT,
                electionEventId: OTHER_EVENT,
                filters: {},
            })
        )
        await expect(canvas.getByRole("combobox", {name: "Election Event"})).toHaveTextContent(
            "Harbour"
        )
        expect(data.calls).toEqual([])
    },
}

async function openQuery(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await recordsShown(canvasElement)
    await userEvent.click(canvas.getByRole("tab", {name: "Query"}))
    await expect(await canvas.findByText("Choose the election event to query.")).toBeVisible()
    await expect(canvas.getByRole("button", {name: "Run Query"})).toBeDisabled()
    await choose(canvasElement, "Tenant", "example-council")
    await choose(canvasElement, "Election Event", "Council")
    await userEvent.click(canvas.getByRole("button", {name: "Run Query"}))
    return canvas
}

export const RunAQuery: Story = {
    args: {superAdmin: true},
    parameters: {widgets: ["QueryTab"]},
    play: async ({canvasElement}) => {
        const canvas = await openQuery(canvasElement)
        await expect(await canvas.findByText("Rows: 2 · 12 ms")).toBeVisible()
        await expect(canvas.getByText("KeyGeneration")).toBeVisible()
        expect(canvas.queryByText(/Only the first/)).toBeNull()
        expect(graphql.calls.filter(({name}) => name === "ElectoralLogConsoleQuery")).toEqual([
            {
                name: "ElectoralLogConsoleQuery",
                variables: {tenantId: TENANT_ID, electionEventId: EVENT_ID, sql: QUERY_SQL},
                headers: QUERY_ROLE,
            },
        ])
    },
}

export const TruncatedQuery: Story = {
    args: {superAdmin: true, queryAnswer: "truncated"},
    parameters: {widgets: ["QueryTab"]},
    play: async ({canvasElement}) => {
        const canvas = await openQuery(canvasElement)
        await expect(await canvas.findByText("Only the first 2 rows are shown.")).toBeVisible()
    },
}

export const QueryRejected: Story = {
    args: {superAdmin: true, queryAnswer: "error"},
    parameters: {widgets: ["QueryTab"]},
    play: async ({canvasElement}) => {
        const canvas = await openQuery(canvasElement)
        await expect(
            await canvas.findByText('ERROR: relation "electoral_log" does not exist')
        ).toBeVisible()
        expect(canvas.queryByText(/^Rows:/)).toBeNull()
        await expect(canvas.getByRole("button", {name: "Run Query"})).toBeEnabled()
    },
}

export const WithoutConsolePermission: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(
                "You need the electoral-log-console-read permission to open the electoral log."
            )
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}
