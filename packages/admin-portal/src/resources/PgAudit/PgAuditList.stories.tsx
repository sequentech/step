// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {PgAuditTable, type PgAuditRow} from "@/gql/graphql"
import {PgAuditList} from "./PgAuditList"

interface Scenario {
    auditTable: PgAuditTable
    /** What reading the audit rows does. */
    reads: ReadState
    /** Whether the table has no audit rows. */
    empty: boolean
}

type AuditRecord = Omit<PgAuditRow, "__typename">

/** The audit service reports `server_timestamp` in microseconds since the epoch. */
const auditRecords = (): AuditRecord[] => [
    {
        id: 1,
        audit_type: "SESSION",
        class: "WRITE",
        command: "UPDATE",
        dbname: "hasura",
        server_timestamp: 1768478400000000,
        session_id: "session-0001",
        statement: "UPDATE sequent_backend.election SET status = $1",
        user: "hasura",
    },
    {
        id: 2,
        audit_type: "SESSION",
        class: "READ",
        command: "SELECT",
        dbname: "hasura",
        server_timestamp: 1768478730000000,
        session_id: "session-0002",
        statement: "SELECT id FROM sequent_backend.tenant",
        user: "admin",
    },
]

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Pg audit/PgAuditList",
    component: PgAuditList,
    args: {auditTable: PgAuditTable.PgauditHasura, reads: "records", empty: false},
    argTypes: {
        auditTable: {control: "inline-radio", options: Object.values(PgAuditTable)},
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
    },
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async ({args}) => {
        const rows = args.empty ? [] : auditRecords()
        data = resourceBoundary(
            {[PgAuditTable.PgauditHasura]: rows, [PgAuditTable.PgauditKeycloak]: rows},
            {reads: args.reads}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({auditTable}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <PgAuditList auditTable={auditTable} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const auditRow = (canvasElement: HTMLElement, statement: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(statement)})

const readResources = () =>
    data.calls.filter(({method}) => method === "getList").map(({args}) => args[0])

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const row = await auditRow(canvasElement, "UPDATE sequent_backend.election")
        await expect(within(row).getByText("Thu, 15 Jan 2026 12:00:00 GMT")).toBeVisible()
        await expect(within(row).getByText("session-0001")).toBeVisible()
        // The audit type, class and database columns start hidden.
        expect(within(row).queryByText("WRITE")).not.toBeInTheDocument()
        expect(readResources()).toEqual([PgAuditTable.PgauditHasura])
    },
}

export const KeycloakAudit: Story = {
    args: {auditTable: PgAuditTable.PgauditKeycloak},
    play: async ({canvasElement}) => {
        await auditRow(canvasElement, "SELECT id FROM sequent_backend.tenant")
        expect(readResources()).toEqual([PgAuditTable.PgauditKeycloak])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(readResources()).toEqual([PgAuditTable.PgauditHasura]))
        expect(within(canvasElement).queryByRole("row", {name: /UPDATE/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /UPDATE/})).toBeNull()
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        expectedFailure: {
            reason: "React-admin's empty list message is light grey below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("No Pgaudit hasuras yet.")
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("row")).toBeNull()
    },
}

export const FilterByUser: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await auditRow(canvasElement, "SELECT id")
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "User"})
        )
        await userEvent.type(await canvas.findByRole("textbox", {name: "User"}), "admin")
        await waitFor(() => expect(canvas.queryByRole("row", {name: /UPDATE/})).toBeNull())
        expect(data.calls.at(-1)?.args[1]).toMatchObject({filter: {user: "admin"}})
        await expect(await auditRow(canvasElement, "SELECT id")).toBeVisible()
    },
}
