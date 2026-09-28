// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, ResourceContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import EditElectionEventTextDataTable from "./EditElectionEventTextDataTable"
import {OVERRIDES, localizedEvent} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election_event"

interface Scenario {
    /** The overrides of each language. */
    overrides: Record<string, Record<string, string>>
    /** A write rejects with this message. */
    writeError?: string
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

// The event's edit view provides the record; the table writes through the data provider.
function Fixture({overrides}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <ResourceContextProvider value={RESOURCE}>
                <RecordContextProvider value={localizedEvent(overrides)}>
                    <EditElectionEventTextDataTable />
                </RecordContextProvider>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/EditElectionEventTextDataTable",
    component: EditElectionEventTextDataTable,
    args: {overrides: OVERRIDES},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        expectedFailure: {
            reason: "The row's edit and delete actions are icon buttons without accessible names.",
            a11y: ["button-name"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[RESOURCE]: [localizedEvent(args.overrides)]},
            {writeError: args.writeError}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const localization = (key: string) => i18n.t(`electionEventScreen.localization.${key}`)
const label = (key: string) => localization(`labels.${key}`)
const row = (canvasElement: HTMLElement, name: RegExp) =>
    within(canvasElement).findByRole("row", {name})

async function drawer() {
    const titles = await within(document.body).findAllByText(localization("common.title"))
    const title = titles.find((element) => element.checkVisibility())
    await waitFor(() => expect(title).toBeVisible())
    return within(title!.closest("form") as HTMLElement)
}

async function notified(message: string) {
    const notice = await within(document.body).findByText(message, {
        selector: ".MuiSnackbarContent-message",
    })
    await waitFor(() => expect(notice).toBeVisible())
}

const dialogClosed = () =>
    waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())

const written = () =>
    data.writes.map(
        ({params}) =>
            (params.data as Partial<ReturnType<typeof localizedEvent>> | undefined)?.presentation
                ?.i18n
    )

export const Populated: Story = {
    parameters: {widgets: ["LocalizationList"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await row(canvasElement, /welcome.*Welcome, voters/)).toBeVisible()
        await expect(
            await row(canvasElement, new RegExp(`footer.*${localization("scopes.global")}`))
        ).toBeVisible()
        await expect(
            canvas.getByRole("combobox", {name: localization("selectLanguage")})
        ).toHaveTextContent(i18n.t("common.language.en"))
        await expect(canvas.getByRole("button", {name: i18n.t("common.label.add")})).toBeVisible()
        expect(data.calls).toEqual([])
    },
}

export const SpanishOverrides: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("combobox", {name: localization("selectLanguage")}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: i18n.t("common.language.es")})
        )
        await expect(await row(canvasElement, /welcome.*Bienvenidos/)).toBeVisible()
        await waitFor(() => expect(canvas.queryByRole("row", {name: /footer/})).toBeNull())
    },
}

export const Empty: Story = {
    args: {overrides: {}},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("combobox", {name: localization("selectLanguage")})
        ).toBeVisible()
        expect(canvas.queryByRole("row", {name: /welcome/})).toBeNull()
    },
}

export const WithoutLocalizationPermissions: Story = {
    globals: {permissions: EStoryPermissions.TRUSTEE},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const welcome = await row(canvasElement, /welcome/)
        expect(within(welcome).queryByRole("button")).toBeNull()
        expect(canvas.queryByRole("button", {name: i18n.t("common.label.add")})).toBeNull()
        expect(canvas.queryByRole("combobox", {name: localization("selectLanguage")})).toBeNull()
    },
}

async function addOverride(canvasElement: HTMLElement, key: string, value: string) {
    await userEvent.click(
        within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
    )
    const form = await drawer()
    await userEvent.type(form.getByRole("textbox", {name: label("key")}), key)
    await userEvent.type(form.getByRole("textbox", {name: label("value")}), value)
    await userEvent.click(form.getByRole("button", {name: "Save"}))
}

export const AddAnOverride: Story = {
    parameters: {
        expectedFailure: {
            reason: "The row actions are unnamed icon buttons.",
            a11y: ["button-name"],
        },
    },
    play: async ({canvasElement}) => {
        await row(canvasElement, /welcome/)
        await addOverride(canvasElement, "greeting", "Good morning")
        await notified(localization("notify.success"))
        expect(data.writes).toEqual([
            expect.objectContaining({method: "update", resource: RESOURCE}),
        ])
        expect(data.writes[0].params).toMatchObject({id: EVENT_ID})
        expect(written()).toEqual([
            {
                ...OVERRIDES,
                en: {...OVERRIDES.en, "votingPortal:greeting": "Good morning"},
            },
        ])
    },
}

export const DuplicateOverride: Story = {
    parameters: {
        expectedFailure: {
            reason: "The open drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        await row(canvasElement, /welcome/)
        await addOverride(canvasElement, "welcome", "Hello again")
        await notified("An override with this key and portal scope already exists.")
        expect(data.writes).toEqual([])
    },
}

export const InvalidDateTimeFormat: Story = {
    parameters: {
        expectedFailure: {
            reason: "The open drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        await row(canvasElement, /welcome/)
        await addOverride(canvasElement, "votingPortalDateTimeFormat", "YYYY-MM-DD")
        await notified(localization("notify.invalidDateTimeFormat"))
        expect(data.writes).toEqual([])
    },
}

export const EditAnOverride: Story = {
    parameters: {
        expectedFailure: {
            reason: "The row actions are unnamed icon buttons.",
            a11y: ["button-name"],
        },
    },
    play: async ({canvasElement}) => {
        const welcome = await row(canvasElement, /welcome/)
        await userEvent.click(within(welcome).getAllByRole("button")[0])
        const form = await drawer()
        expect(form.getByRole("textbox", {name: label("key")})).toHaveValue("welcome")
        const value = form.getByRole("textbox", {name: label("value")})
        expect(value).toHaveValue("Welcome, voters")
        await userEvent.clear(value)
        await userEvent.type(value, "Hello, voters")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await notified(localization("notify.success"))
        expect(written()).toEqual([
            {...OVERRIDES, en: {...OVERRIDES.en, "votingPortal:welcome": "Hello, voters"}},
        ])
    },
}

export const DeleteAnOverride: Story = {
    play: async ({canvasElement}) => {
        const footer = await row(canvasElement, /footer/)
        await userEvent.click(within(footer).getAllByRole("button")[1])
        const dialog = await within(document.body).findByRole("dialog")
        await expect(within(dialog).getByText(i18n.t("common.message.delete"))).toBeVisible()
        await userEvent.click(
            within(dialog).getByRole("button", {name: i18n.t("common.label.delete")})
        )
        await notified(localization("notify.success"))
        expect(written()).toEqual([{...OVERRIDES, en: {"votingPortal:welcome": "Welcome, voters"}}])
        await dialogClosed()
    },
}

export const WriteFails: Story = {
    args: {writeError: "Synthetic service unavailable"},
    play: async ({canvasElement}) => {
        const footer = await row(canvasElement, /footer/)
        await userEvent.click(within(footer).getAllByRole("button")[1])
        const dialog = await within(document.body).findByRole("dialog")
        await userEvent.click(
            within(dialog).getByRole("button", {name: i18n.t("common.label.delete")})
        )
        await notified(localization("notify.error"))
        expect(data.writes).toHaveLength(1)
        await dialogClosed()
    },
}
