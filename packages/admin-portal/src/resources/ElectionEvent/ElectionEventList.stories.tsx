// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {createStore, Provider as AtomProvider} from "jotai"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, eventRecord, storyId} from "@/__stories__/fixtures"
import type {ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import archivedElectionEventSelection from "@/atoms/archived-election-event-selection"
import {
    createFlow,
    type CreateFlow,
} from "@/components/election-event/create/__stories__/CreateElectionEventFixture"
import {NewResourceContext} from "@/providers/NewResourceProvider"
import {ElectionEventList} from "./ElectionEventList"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election_event"
const OLDER_EVENT_ID = storyId(2, 1)

interface Scenario {
    /** Whether the tenant has election events. */
    empty: boolean
    /** What reading the election events does. */
    reads: ReadState
    /** Whether the sidebar shows archived election events. */
    archived: boolean
}

let flow: CreateFlow

function Fixture({archived}: Scenario) {
    const {permissions} = useStoryGlobals()
    const [atoms] = useState(() => {
        const store = createStore()
        store.set(archivedElectionEventSelection, archived)
        return store
    })
    return (
        <AdminStoryProvider
            boundary={flow.graphql}
            dataProvider={flow.data.provider}
            role={permissions}
        >
            <AtomProvider store={atoms}>
                <NewResourceContext.Provider
                    value={{lastCreatedResource: null, setLastCreatedResource: flow.created}}
                >
                    <ElectionEventList />
                </NewResourceContext.Provider>
            </AtomProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/ElectionEventList",
    component: ElectionEventList,
    args: {empty: false, reads: "records", archived: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        flow = createFlow({
            reads: args.reads,
            events: args.empty
                ? []
                : [
                      eventRecord(undefined, {
                          id: OLDER_EVENT_ID,
                          created_at: "2025-01-01T00:00:00Z",
                      }),
                      eventRecord(undefined, {created_at: FIXED_TIME}),
                      eventRecord(undefined, {id: storyId(2, 2), is_archived: true}),
                  ],
        })
        await flow.graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText("Current location")

const listParams = () =>
    flow.data.calls.find(({method, args}) => method === "getList" && args[0] === RESOURCE)?.args[1]

const noEvents = () => i18n.t("electionEventScreen.error.noResult")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        // The list opens the newest election event of the tenant.
        await waitFor(() =>
            expect(location(canvasElement).textContent).toBe(`/${RESOURCE}/${EVENT_ID}`)
        )
        expect(listParams()).toMatchObject({
            sort: {field: "created_at", order: "DESC"},
            filter: {tenant_id: TENANT_ID, is_archived: false},
        })
        expect(flow.graphql.calls).toEqual([])
    },
}

export const ArchivedEvents: Story = {
    args: {archived: true},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(location(canvasElement).textContent).toBe(`/${RESOURCE}/${storyId(2, 2)}`)
        )
        expect(listParams()).toMatchObject({filter: {tenant_id: TENANT_ID, is_archived: true}})
    },
}

export const Empty: Story = {
    args: {empty: true},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(noEvents())).toBeVisible()
        await expect(canvas.getByRole("button", {name: i18n.t("common.label.add")})).toBeVisible()
        await expect(
            canvas.getByRole("button", {name: i18n.t("common.label.import")})
        ).toBeVisible()
        expect(location(canvasElement).textContent).toBe(`/${RESOURCE}`)
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {
        expectedFailure: {
            reason: "The loading spinner is a progressbar without an accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("progressbar")).toBeVisible()
        expect(canvas.queryByText(noEvents())).toBeNull()
        expect(listParams()).toBeDefined()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        // A failed read looks like a tenant without election events.
        await expect(await within(canvasElement).findByText(noEvents())).toBeVisible()
        expect(location(canvasElement).textContent).toBe(`/${RESOURCE}`)
    },
}

export const WithoutWritePermission: Story = {
    args: {empty: true},
    globals: {permissions: EStoryPermissions.TRUSTEE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(noEvents())).toBeVisible()
        expect(canvas.queryByRole("button", {name: i18n.t("common.label.add")})).toBeNull()
        expect(canvas.queryByRole("button", {name: i18n.t("common.label.import")})).toBeNull()
    },
}

export const AddOpensTheCreateDrawer: Story = {
    args: {empty: true},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        expectedFailure: {
            reason: "The drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: i18n.t("common.label.add")}))
        const form = await within(document.body).findByText("Create an Election Event")
        await waitFor(() => expect(form).toBeVisible())
        expect(flow.graphql.calls).toEqual([])
    },
}

export const ImportOpensTheImportDrawer: Story = {
    args: {empty: true},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        expectedFailure: {
            reason: "The drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            await canvas.findByRole("button", {name: i18n.t("common.label.import")})
        )
        const title = await within(document.body).findByText(
            i18n.t("electionEventScreen.import.eetitle")
        )
        await waitFor(() => expect(title).toBeVisible())
        expect(flow.uploads.calls).toEqual([])
    },
}
