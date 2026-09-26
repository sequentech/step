// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, STORY_IDS, storyId, type StoryRecord} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Phone_Blacklist} from "@/gql/graphql"
import {IPermissions} from "@/types/keycloak"
import {PhoneBlacklist} from "./PhoneBlacklist"
import {iconButton, ivrEvent} from "./__stories__/IvrFixture"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    empty: boolean
    /** Replaces the signed-in group's roles. */
    roles?: string[]
    /** Whether the blocklist mutations fail. */
    mutationsFail: boolean
}

const RESOURCE = "sequent_backend_phone_blacklist"

const entries: StoryRecord<Sequent_Backend_Phone_Blacklist>[] = [
    {phone: "+34600000009", reason: null},
    {phone: "+34600000003", reason: "Repeated test calls"},
].map(({phone, reason}, index) => ({
    id: storyId(5, index + 1),
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    phone_e164: phone,
    reason,
    created_by: STORY_IDS.user,
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
}))

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({roles}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            roles={roles}
            tenant={tenant}
        >
            <RecordContextProvider value={ivrEvent()}>
                <PhoneBlacklist />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const mutation = (field: string, id: string, fails: boolean) => () => {
    if (fails) throw new Error("Synthetic blocklist service unavailable")
    return {data: {[field]: {id}}}
}

const rowActionDefects = {
    expectedFailure: {
        reason: "The row's edit and delete actions are icon buttons without a name.",
        a11y: ["button-name"],
    },
}

// Stories that leave a drawer open; the list behind the modal drawer is hidden.
const openDrawerDefects = {
    expectedFailure: {
        reason: "The add and edit drawers are modal dialogs without an accessible name.",
        a11y: ["aria-dialog-name"],
    },
}

const meta = {
    title: "Admin/Election event/PhoneBlacklist",
    component: PhoneBlacklist,
    args: {reads: "records", empty: false, mutationsFail: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {...rowActionDefects, widgets: ["RowActions"]},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[RESOURCE]: args.empty ? [] : entries}, {reads: args.reads})
        graphql = graphqlBoundary(
            {
                CreatePhoneBlacklistEntry: mutation(
                    "create_phone_blacklist_entry",
                    storyId(5, 3),
                    args.mutationsFail
                ),
                UpdatePhoneBlacklistEntry: mutation(
                    "update_sequent_backend_phone_blacklist_by_pk",
                    entries[1].id,
                    args.mutationsFail
                ),
                DeletePhoneBlacklistEntry: mutation(
                    "delete_phone_blacklist_entry",
                    entries[1].id,
                    args.mutationsFail
                ),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const entryRow = (canvasElement: HTMLElement, phone: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(`\\${phone}`)})

const drawer = async () => within(await within(document.body).findByRole("presentation"))

const operations = () => graphql.calls.map(({name}) => name)

export const Populated: Story = {
    parameters: {widgets: ["RowActions"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const row = await entryRow(canvasElement, "+34600000003")
        await expect(within(row).getByText("Repeated test calls")).toBeVisible()
        expect(iconButton(row, "EditIcon")).not.toBeNull()
        expect(iconButton(row, "DeleteIcon")).not.toBeNull()
        const phones = within(await canvas.findByRole("table"))
            .getAllByRole("row")
            .slice(1)
            .map((tableRow) => tableRow.querySelector(".column-phone_e164")?.textContent)
        expect(phones).toEqual(["+34600000003", "+34600000009"])
        expect(data.calls.find(({method}) => method === "getList")?.args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID, election_event_id: EVENT_ID},
            sort: {field: "phone_e164", order: "ASC"},
        })
        expect(graphql.calls).toEqual([])
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.PHONE_BLACKLIST_READ]},
    parameters: {expectedFailure: null, widgets: []},
    play: async ({canvasElement}) => {
        const row = await entryRow(canvasElement, "+34600000003")
        expect(iconButton(row, "EditIcon")).toBeNull()
        expect(iconButton(row, "DeleteIcon")).toBeNull()
        expect(within(canvasElement).queryByRole("button", {name: "Add"})).toBeNull()
    },
}

export const DeleteOnly: Story = {
    args: {roles: [IPermissions.PHONE_BLACKLIST_READ, IPermissions.PHONE_BLACKLIST_DELETE]},
    play: async ({canvasElement}) => {
        const row = await entryRow(canvasElement, "+34600000003")
        expect(iconButton(row, "EditIcon")).toBeNull()
        expect(iconButton(row, "DeleteIcon")).not.toBeNull()
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        ...openDrawerDefects,
        widgets: ["Empty"],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("There are no entries in the blocklist")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Add"}))
        await expect(
            await (await drawer()).findByRole("textbox", {name: "Phone number"})
        ).toBeVisible()
    },
}

export const EmptyWithoutCreatePermission: Story = {
    args: {empty: true, roles: [IPermissions.PHONE_BLACKLIST_READ]},
    parameters: {expectedFailure: null, widgets: ["Empty"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("There are no entries in the blocklist")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Add"})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null, widgets: []},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({args}) => args[0])).toContain(RESOURCE))
        expect(within(canvasElement).queryByRole("row", {name: /\+34600000003/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null, widgets: []},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /\+34600000003/})).toBeNull()
    },
}

export const AddEntry: Story = {
    play: async ({canvasElement}) => {
        await entryRow(canvasElement, "+34600000003")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add"}))
        const form = await drawer()
        await userEvent.type(form.getByRole("textbox", {name: "Phone number"}), " +34600000005 ")
        await userEvent.type(form.getByRole("textbox", {name: "Reason"}), "Abusive calls")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await expect(await within(document.body).findByText("Saved successfully")).toBeVisible()
        expect(graphql.calls).toEqual([
            {
                name: "CreatePhoneBlacklistEntry",
                variables: {
                    election_event_id: EVENT_ID,
                    phone_e164: "+34600000005",
                    reason: "Abusive calls",
                },
                headers: {"x-hasura-role": IPermissions.PHONE_BLACKLIST_CREATE},
            },
        ])
        await waitFor(() =>
            expect(within(document.body).queryByRole("textbox", {name: "Phone number"})).toBeNull()
        )
    },
}

export const AddRequiresAPhoneNumber: Story = {
    parameters: openDrawerDefects,
    play: async ({canvasElement}) => {
        await entryRow(canvasElement, "+34600000003")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add"}))
        const form = await drawer()
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await expect(await form.findByText("Phone number is required")).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

export const AddFailure: Story = {
    args: {mutationsFail: true},
    parameters: openDrawerDefects,
    play: async ({canvasElement}) => {
        await entryRow(canvasElement, "+34600000003")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add"}))
        const form = await drawer()
        await userEvent.type(form.getByRole("textbox", {name: "Phone number"}), "+34600000005")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await expect(await within(document.body).findByText("Failed to save")).toBeVisible()
        expect(operations()).toEqual(["CreatePhoneBlacklistEntry"])
        // The drawer stays open with the entry, to try again.
        await expect(form.getByRole("textbox", {name: "Phone number"})).toHaveValue("+34600000005")
    },
}

export const EditReason: Story = {
    play: async ({canvasElement}) => {
        const row = await entryRow(canvasElement, "+34600000003")
        await userEvent.click(iconButton(row, "EditIcon") as HTMLElement)
        const form = await drawer()
        await expect(form.getByRole("textbox", {name: "Phone number"})).toBeDisabled()
        await expect(form.getByRole("textbox", {name: "Phone number"})).toHaveValue("+34600000003")
        const reason = form.getByRole("textbox", {name: "Reason"})
        await expect(reason).toHaveValue("Repeated test calls")
        await userEvent.clear(reason)
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await expect(await within(document.body).findByText("Saved successfully")).toBeVisible()
        // A cleared reason is stored as null.
        expect(graphql.calls).toEqual([
            {
                name: "UpdatePhoneBlacklistEntry",
                variables: {id: entries[1].id, reason: null},
                headers: {"x-hasura-role": IPermissions.PHONE_BLACKLIST_UPDATE},
            },
        ])
    },
}

export const DeleteAfterConfirmation: Story = {
    play: async ({canvasElement}) => {
        const row = await entryRow(canvasElement, "+34600000003")
        await userEvent.click(iconButton(row, "DeleteIcon") as HTMLElement)
        const dialog = within(await within(document.body).findByRole("dialog"))
        expect(graphql.calls).toEqual([])
        await userEvent.click(dialog.getByRole("button", {name: "Delete"}))
        await expect(await within(document.body).findByText("Deleted successfully")).toBeVisible()
        expect(graphql.calls).toEqual([
            {
                name: "DeletePhoneBlacklistEntry",
                variables: {id: entries[1].id, election_event_id: EVENT_ID},
                headers: {"x-hasura-role": IPermissions.PHONE_BLACKLIST_DELETE},
            },
        ])
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const DeleteFailure: Story = {
    args: {mutationsFail: true},
    play: async ({canvasElement}) => {
        const row = await entryRow(canvasElement, "+34600000003")
        await userEvent.click(iconButton(row, "DeleteIcon") as HTMLElement)
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: "Delete"}))
        await expect(await within(document.body).findByText("Failed to delete")).toBeVisible()
        expect(operations()).toEqual(["DeletePhoneBlacklistEntry"])
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}
