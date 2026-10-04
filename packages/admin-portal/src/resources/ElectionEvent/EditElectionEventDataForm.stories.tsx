// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {
    RecordContextProvider,
    ResourceContextProvider,
    SaveContextProvider,
    type Identifier,
    type RaRecord,
    type SaveHandler,
} from "react-admin"
import {i18n, initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {
    EditElectionEventDataForm,
    type Sequent_Backend_Election_Event_Extended,
} from "./EditElectionEventDataForm"
import {
    eventDataBoundaries,
    eventDataEvent,
    type EventDataScenario,
} from "./__stories__/EventDataFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

type Values = Sequent_Backend_Election_Event_Extended

interface Scenario extends EventDataScenario {
    /** The edit view's transform, which receives the values the form prepared. */
    transform: Mock<(values: Values) => Promise<RaRecord<Identifier>>>
    /** Receives what the edit view would write. */
    saved: Mock<(values: RaRecord<Identifier>) => void>
    /** A save rejects with this message. */
    saveError?: string
}

let boundaries: ReturnType<typeof eventDataBoundaries>

// The edit view provides the record and the save; like EditBase, the save applies the
// button's transform before writing.
function Fixture({transform, saved, saveError}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const save: SaveHandler<RaRecord> = async (values, options) => {
        const data = options?.transform ? await options.transform(values) : values
        if (saveError) throw new Error(saveError)
        saved(data)
    }
    return (
        <AdminStoryProvider
            boundary={boundaries.graphql}
            dataProvider={boundaries.data.provider}
            role={permissions}
            tenant={tenant}
        >
            <WidgetsContextProvider>
                <ResourceContextProvider value="sequent_backend_election_event">
                    <RecordContextProvider value={eventDataEvent()}>
                        <SaveContextProvider
                            value={{save, saving: false, mutationMode: "pessimistic"}}
                        >
                            <EditElectionEventDataForm transform={transform} />
                        </SaveContextProvider>
                    </RecordContextProvider>
                </ResourceContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const JSON_CONTRAST = "The JSON editors' item counts are light grey below the contrast minimum."

/** The advanced and realm attribute sections show a JSON editor. */
const openedSection = {expectedFailure: {reason: JSON_CONTRAST, a11y: ["color-contrast"]}}

const meta = {
    title: "Admin/Election event/EditElectionEventDataForm",
    component: EditElectionEventDataForm,
    args: {
        reads: "records",
        customOrder: false,
        realmAttributesFail: false,
        transform: fn(async (values: Values) => values),
        saved: fn(),
    },
    globals: {permissions: EStoryPermissions.ADMIN},
    beforeEach: async ({args}) => {
        boundaries = eventDataBoundaries(args)
        await Promise.all([boundaries.graphql.ready, initCore()])
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const edit = (key: string) => i18n.t(`electionEventScreen.edit.${key}`)
const field = (key: string) => i18n.t(`electionEventScreen.field.${key}`)
const results = (key: string) => i18n.t(`tally.resultsPublication.${key}`)
const section = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("button", {name: edit(key)})
const operations = () => boundaries.graphql.calls.map(({name}) => name)

async function loaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("textbox", {name: field("name")})).toHaveValue(
        "Council event"
    )
    return canvas
}

async function openSection(canvasElement: HTMLElement, key: string, open = "general") {
    const summary = section(canvasElement, key)
    await userEvent.click(summary)
    await waitFor(() => expect(summary).toHaveAttribute("aria-expanded", "true"))
    // Only one section is open: the one that was open finishes collapsing.
    const previous = section(canvasElement, open)
    await waitFor(() =>
        expect(
            previous.closest(".MuiAccordion-root")?.querySelector(".MuiCollapse-root")
        ).toHaveClass("MuiCollapse-hidden")
    )
}

async function choose(canvasElement: HTMLElement, label: string, option: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: label}))
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
    await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull())
}

async function notified(message: string) {
    const notice = await within(document.body).findByText(message, {
        selector: ".MuiSnackbarContent-message",
    })
    await waitFor(() => expect(notice).toBeVisible())
}

const saveButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: "Save"})

async function editDescription(canvasElement: HTMLElement) {
    const canvas = await loaded(canvasElement)
    const description = canvas.getByRole("textbox", {name: field("description")})
    await userEvent.clear(description)
    await userEvent.type(description, "Council members for 2026")
}

export const Populated: Story = {
    parameters: {widgets: ["EventSaveButton", "ResultsWebsitePolicyFields"]},
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        await expect(canvas.getByRole("tab", {name: "English", selected: true})).toBeVisible()
        await expect(canvas.getByRole("tab", {name: "Spanish"})).toBeVisible()
        await expect(section(canvasElement, "realm_attributes")).toBeVisible()
        await expect(section(canvasElement, "password_policy")).toBeVisible()
        await expect(canvas.getByRole("button", {name: edit("importCandidates")})).toBeVisible()
        await expect(
            canvas.getByRole("button", {name: i18n.t("googleMeet.generateButton")})
        ).toBeVisible()
        expect(saveButton(canvasElement)).toBeDisabled()
        await waitFor(() =>
            expect(operations()).toEqual(
                expect.arrayContaining(["GetRealmAttributes", "GetRealmPasswordPolicy"])
            )
        )
        expect(
            boundaries.data.calls.map(({method, args: [resource]}) => `${method} ${resource}`)
        ).toEqual(
            expect.arrayContaining([
                "getOne sequent_backend_tenant",
                "getList sequent_backend_election",
                "getList sequent_backend_support_material",
            ])
        )
        expect(args.transform).not.toHaveBeenCalled()
    },
}

export const RealmAttributes: Story = {
    parameters: openedSection,
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "realm_attributes")
        const attribute = await within(canvasElement).findByText(/voter_certificate_policy/)
        await waitFor(() => expect(attribute).toBeVisible())
        expect(
            boundaries.graphql.calls.find(({name}) => name === "GetRealmAttributes")
        ).toMatchObject({variables: {election_event_id: EVENT_ID}})
    },
}

export const RealmAttributesLoadError: Story = {
    parameters: openedSection,
    args: {realmAttributesFail: true},
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "realm_attributes")
        await expect(
            await within(canvasElement).findByText(edit("realm_attributes_load_error"))
        ).toBeVisible()
    },
}

export const WithoutRealmAttributes: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {widgets: ["EventSaveButton", "ResultsWebsitePolicyFields"]},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(canvas.queryByRole("button", {name: edit("realm_attributes")})).toBeNull()
        expect(canvas.queryByRole("button", {name: i18n.t("googleMeet.generateButton")})).toBeNull()
        expect(canvas.getByRole("textbox", {name: field("name")})).toBeEnabled()
        await waitFor(() => expect(operations()).toContain("GetRealmPasswordPolicy"))
        expect(operations()).not.toContain("GetRealmAttributes")
    },
}

export const LockedDownAdministrator: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LOCKDOWN},
    parameters: {...openedSection, widgets: ["ResultsWebsitePolicyFields"]},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        // Without the event write permission there is nothing to save.
        expect(canvas.getByRole("textbox", {name: field("name")})).toBeDisabled()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        await openSection(canvasElement, "languageAndRegion")
        await expect(
            canvas.getByRole("combobox", {name: field("numberFormatPolicy.policyLabel")})
        ).toHaveAttribute("aria-disabled", "true")
        await openSection(canvasElement, "advancedConfigurations", "languageAndRegion")
        await expect(canvas.getByRole("combobox", {name: results("policyAccess")})).toBeVisible()
    },
}

export const Trustee: Story = {
    parameters: openedSection,
    globals: {permissions: EStoryPermissions.TRUSTEE},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        await openSection(canvasElement, "advancedConfigurations")
        expect(canvas.queryByRole("combobox", {name: results("policyAccess")})).toBeNull()
    },
}

export const SaveTheEvent: Story = {
    parameters: {widgets: ["EventSaveButton", "ResultsWebsitePolicyFields"]},
    play: async ({canvasElement, args}) => {
        await editDescription(canvasElement)
        await userEvent.click(saveButton(canvasElement))
        await waitFor(() => expect(args.saved).toHaveBeenCalledTimes(1))
        const [values] = args.transform.mock.calls[0]
        expect(values).toMatchObject({
            id: EVENT_ID,
            enabled_languages: {en: true, es: true},
            presentation: {
                i18n: {en: {description: "Council members for 2026"}},
                results_website: JSON.stringify({
                    status: "disabled",
                    access: "public",
                    visibility_scope: "full_event",
                }),
            },
        })
        const custom = boundaries.graphql.calls.filter(({name}) => name === "SetCustomUrls")
        expect(custom.map(({variables}) => variables.key)).toEqual(["login", "enrollment", "saml"])
        expect(custom[0].variables).toMatchObject({election_id: EVENT_ID, dns_prefix: ""})
        expect(
            boundaries.graphql.calls.find(({name}) => name === "SetVoterAuthentication")?.variables
        ).toEqual({electionEventId: EVENT_ID, enrollment: "", otp: ""})
        expect(
            boundaries.graphql.calls.find(({name}) => name === "ConfigureResultsWebsitePolicy")
                ?.variables
        ).toEqual({
            election_event_id: EVENT_ID,
            status: "disabled",
            access: "public",
            visibility_scope: "full_event",
        })
        // The unchanged password policy and realm attributes are not written.
        expect(operations()).not.toContain("UpdateRealmPasswordPolicy")
        expect(operations()).not.toContain("UpdateRealmAttributes")
    },
}

export const SaveFails: Story = {
    args: {saveError: "Synthetic service unavailable"},
    parameters: {widgets: ["EventSaveButton"]},
    play: async ({canvasElement, args}) => {
        await editDescription(canvasElement)
        await userEvent.click(saveButton(canvasElement))
        await notified("Synthetic service unavailable")
        expect(args.saved).not.toHaveBeenCalled()
    },
}

export const PublicResultsNeedTheWholeEvent: Story = {
    parameters: {...openedSection, widgets: ["EventSaveButton", "ResultsWebsitePolicyFields"]},
    play: async ({canvasElement, args}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "advancedConfigurations")
        await choose(canvasElement, results("policyVisibility"), results("areaBased"))
        await userEvent.click(saveButton(canvasElement))
        await notified("Public results must use full event visibility")
        expect(operations()).not.toContain("ConfigureResultsWebsitePolicy")
        expect(args.transform).not.toHaveBeenCalled()
    },
}

export const AuthenticatedAreaResults: Story = {
    parameters: {...openedSection, widgets: ["EventSaveButton", "ResultsWebsitePolicyFields"]},
    play: async ({canvasElement, args}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "advancedConfigurations")
        await choose(canvasElement, results("policyTitle"), results("enabled"))
        await choose(canvasElement, results("policyAccess"), results("authenticatedAccess"))
        await choose(canvasElement, results("policyVisibility"), results("areaBased"))
        await userEvent.click(saveButton(canvasElement))
        await waitFor(() => expect(args.saved).toHaveBeenCalledTimes(1))
        expect(
            boundaries.graphql.calls.find(({name}) => name === "ConfigureResultsWebsitePolicy")
                ?.variables
        ).toEqual({
            election_event_id: EVENT_ID,
            status: "enabled",
            access: "authenticated",
            visibility_scope: "area_based",
        })
    },
}

export const WeightedVotingConflicts: Story = {
    parameters: openedSection,
    play: async ({canvasElement, args}) => {
        await loaded(canvasElement)
        await openSection(canvasElement, "advancedConfigurations")
        await choose(
            canvasElement,
            field("weightedVotingPolicy.policyLabel"),
            field("weightedVotingPolicy.options.voters-weighted-voting")
        )
        await choose(
            canvasElement,
            field("delegatedVotingPolicy.policyLabel"),
            field("delegatedVotingPolicy.options.enabled")
        )
        await userEvent.click(saveButton(canvasElement))
        const message = field("weightedVotingPolicy.noDelegated")
        await waitFor(() => expect(within(canvasElement).getAllByText(message)).toHaveLength(2))
        expect(args.transform).not.toHaveBeenCalled()
        expect(operations()).not.toContain("SetCustomUrls")
    },
}

export const InvalidCustomDateTimeFormat: Story = {
    parameters: openedSection,
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        await openSection(canvasElement, "advancedConfigurations")
        await choose(
            canvasElement,
            field("votingPortalDateTimeFormat.policyLabel"),
            field("votingPortalDateTimeFormat.options.custom")
        )
        const custom = await canvas.findByRole("textbox", {
            name: field("votingPortalDateTimeFormat.customFormat.label"),
        })
        await userEvent.clear(custom)
        await userEvent.type(custom, "YYYY-MM-DD")
        await userEvent.click(saveButton(canvasElement))
        await expect(
            await canvas.findByText(field("votingPortalDateTimeFormat.customFormat.invalid"))
        ).toBeVisible()
        expect(args.transform).not.toHaveBeenCalled()
    },
}
