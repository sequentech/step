// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef, useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {RealmPasswordPolicy} from "@/queries/RealmPasswordPolicy"
import {
    PasswordPolicyAccordion,
    type PasswordPolicyAccordionHandle,
} from "./PasswordPolicyAccordion"
import {pending} from "../../../../ui-essentials/.storybook/screens"

type UpdateReply = "updated" | "notUpdated" | "error"

interface Scenario {
    /** What reading the realm's password policy does. */
    reads: ReadState
    /** The realm's password policy. */
    policy: RealmPasswordPolicy
    /** What the password policy update answers. */
    update: UpdateReply
    canEdit: boolean
    /** Whether the accordion starts expanded. */
    expanded: boolean
    onDirty: () => void
}

const POLICY: RealmPasswordPolicy = {
    configured: true,
    minimum_length: 10,
    maximum_length: 64,
    include_uppercase: true,
    include_lowercase: true,
    include_digits: false,
    include_special_characters: false,
}

const HIDDEN_FOCUS =
    "The information icons are focusable but aria-hidden, as MUI hides icons without a title."

let graphql: ReturnType<typeof graphqlBoundary>

// The event's edit form owns the expansion and calls `save` when the event is saved.
function Fixture({canEdit, expanded: initiallyExpanded, onDirty}: Scenario) {
    const accordion = useRef<PasswordPolicyAccordionHandle | null>(null)
    const [expanded, setExpanded] = useState(initiallyExpanded)
    const [saved, setSaved] = useState<string>("")
    return (
        <AdminStoryProvider boundary={graphql}>
            <PasswordPolicyAccordion
                ref={accordion}
                electionEventId={EVENT_ID}
                canEdit={canEdit}
                expanded={expanded}
                onChange={() => setExpanded((current) => !current)}
                onDirty={onDirty}
            />
            <button
                type="button"
                onClick={async () => setSaved(String(await accordion.current?.save()))}
            >
                Save event
            </button>
            <output aria-label="Saved">{saved}</output>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/PasswordPolicyAccordion",
    component: PasswordPolicyAccordion,
    args: {
        reads: "records",
        policy: POLICY,
        update: "updated",
        canEdit: true,
        expanded: true,
        onDirty: fn(),
    },
    parameters: {
        expectedFailure: {
            reason: HIDDEN_FOCUS,
            a11y: ["aria-hidden-focus"],
        },
    },
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        update: {control: "inline-radio", options: ["updated", "notUpdated", "error"]},
    },
    beforeEach: async ({args}) => {
        let policy = args.policy
        graphql = graphqlBoundary(
            {
                GetRealmPasswordPolicy: () => {
                    if (args.reads === "loading") return pending()
                    if (args.reads === "error")
                        return {errors: [new GraphQLError("Synthetic Keycloak failure")]}
                    return {data: {get_realm_password_policy: policy}}
                },
                UpdateRealmPasswordPolicy: ({variables}) => {
                    if (args.update === "error")
                        return {errors: [new GraphQLError("Synthetic Keycloak failure")]}
                    const updated = args.update === "updated"
                    if (updated) {
                        const {election_event_id: _event, ...values} = variables
                        policy = {...policy, ...values, configured: true}
                    }
                    return {data: {update_realm_password_policy: {updated}}}
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string) => i18n.t(`electionEventScreen.field.passwordPolicy.${key}`)
const help = (key: string) => i18n.t(`electionEventScreen.field.passwordPolicy.help.${key}`)
const edit = (key: string) => i18n.t(`electionEventScreen.edit.${key}`)
const checkbox = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("checkbox", {name: new RegExp(`^${label(key)}`)})
const minimum = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("spinbutton", {name: label("minimumLength")})

async function saveEvent(canvasElement: HTMLElement, result: string) {
    const canvas = within(canvasElement)
    await userEvent.click(canvas.getByRole("button", {name: "Save event"}))
    await waitFor(() => expect(canvas.getByLabelText("Saved")).toHaveTextContent(result))
}

async function notified(message: string) {
    const notice = await within(document.body).findByRole("alert")
    await waitFor(() => expect(notice).toBeVisible())
    expect(notice).toHaveTextContent(message)
}

const updates = () => graphql.calls.filter(({name}) => name === "UpdateRealmPasswordPolicy")

export const Populated: Story = {
    parameters: {widgets: ["PasswordPolicyFieldLabel"]},
    play: async ({canvasElement, args}) => {
        await expect(await minimum(canvasElement)).toHaveValue(10)
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("spinbutton", {name: label("maximumLength")})).toHaveValue(64)
        expect(checkbox(canvasElement, "includeUppercase")).toBeChecked()
        expect(checkbox(canvasElement, "includeDigits")).not.toBeChecked()
        expect(canvas.queryByText(label("notConfigured"))).toBeNull()
        expect(graphql.calls).toEqual([
            expect.objectContaining({
                name: "GetRealmPasswordPolicy",
                variables: {election_event_id: EVENT_ID},
            }),
        ])
        expect(args.onDirty).not.toHaveBeenCalled()
    },
}

export const FieldInformation: Story = {
    parameters: {widgets: ["PasswordPolicyFieldLabel"]},
    play: async ({canvasElement}) => {
        await minimum(canvasElement)
        await userEvent.hover(
            within(canvasElement).getByLabelText(help("minimumLength"), {selector: "svg"})
        )
        const tooltip = await within(document.body).findByRole("tooltip")
        await waitFor(() => expect(tooltip).toHaveTextContent(help("minimumLength")))
    },
}

export const NotConfigured: Story = {
    args: {policy: {...POLICY, configured: false}},
    parameters: {
        expectedFailure: {
            reason: `${HIDDEN_FOCUS} The warning colour of the notice lacks contrast on white.`,
            a11y: ["aria-hidden-focus", "color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText(label("notConfigured"))).toBeVisible()
        await expect(await minimum(canvasElement)).toBeEnabled()
    },
}

export const Collapsed: Story = {
    args: {expanded: false},
    play: async ({canvasElement}) => {
        const summary = within(canvasElement).getByRole("button", {name: edit("password_policy")})
        expect(summary).toHaveAttribute("aria-expanded", "false")
        await userEvent.click(summary)
        await waitFor(() => expect(summary).toHaveAttribute("aria-expanded", "true"))
        await expect(await minimum(canvasElement)).toBeVisible()
    },
}

export const Loading: Story = {
    parameters: {expectedFailure: null},
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("loading"))).toBeVisible()
        expect(canvas.queryByRole("spinbutton")).toBeNull()
    },
}

export const LoadError: Story = {
    parameters: {expectedFailure: null},
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(edit("password_policy_load_error"))).toBeVisible()
        expect(canvas.queryByRole("spinbutton")).toBeNull()
        // Nothing was changed, so saving the event leaves the policy alone.
        await saveEvent(canvasElement, "true")
        expect(updates()).toEqual([])
    },
}

export const ReadOnly: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        await expect(await minimum(canvasElement)).toBeDisabled()
        expect(checkbox(canvasElement, "includeUppercase")).toBeDisabled()
        expect(checkbox(canvasElement, "includeSpecialCharacters")).toBeDisabled()
    },
}

export const SaveWithoutChanges: Story = {
    play: async ({canvasElement}) => {
        await minimum(canvasElement)
        await saveEvent(canvasElement, "true")
        expect(updates()).toEqual([])
    },
}

export const EditAndSave: Story = {
    play: async ({canvasElement, args}) => {
        const input = await minimum(canvasElement)
        await userEvent.clear(input)
        await userEvent.type(input, "14")
        await userEvent.click(checkbox(canvasElement, "includeDigits"))
        expect(args.onDirty).toHaveBeenCalled()
        await saveEvent(canvasElement, "true")
        expect(updates()[0].variables).toEqual({
            election_event_id: EVENT_ID,
            minimum_length: 14,
            maximum_length: 64,
            include_uppercase: true,
            include_lowercase: true,
            include_digits: true,
            include_special_characters: false,
        })
        // The saved policy is read again.
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toEqual([
                "GetRealmPasswordPolicy",
                "UpdateRealmPasswordPolicy",
                "GetRealmPasswordPolicy",
            ])
        )
        await expect(input).toHaveValue(14)
    },
}

export const InvalidPolicy: Story = {
    play: async ({canvasElement}) => {
        const input = await minimum(canvasElement)
        await userEvent.clear(input)
        await userEvent.type(input, "80")
        const error = label("errors.minimumExceedsMaximum")
        await expect(within(canvasElement).getByText(error)).toBeVisible()
        await saveEvent(canvasElement, "false")
        await notified(error)
        expect(updates()).toEqual([])
    },
}

export const NoCharacterClass: Story = {
    args: {policy: {...POLICY, include_lowercase: false}},
    play: async ({canvasElement}) => {
        await minimum(canvasElement)
        await userEvent.click(checkbox(canvasElement, "includeUppercase"))
        await expect(
            within(canvasElement).getByText(label("errors.characterClassRequired"))
        ).toBeVisible()
    },
}

export const UpdateNotApplied: Story = {
    args: {update: "notUpdated"},
    play: async ({canvasElement}) => {
        await minimum(canvasElement)
        await userEvent.click(checkbox(canvasElement, "includeDigits"))
        await saveEvent(canvasElement, "false")
        await notified(edit("password_policy_update_error"))
        expect(updates()).toHaveLength(1)
    },
}

export const UpdateFails: Story = {
    args: {update: "error"},
    play: async ({canvasElement}) => {
        await minimum(canvasElement)
        await userEvent.click(checkbox(canvasElement, "includeDigits"))
        await saveEvent(canvasElement, "false")
        await notified(edit("password_policy_update_error"))
        expect(updates()).toHaveLength(1)
    },
}
