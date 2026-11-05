// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"
import {AdminStoryProvider, EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {AUDITOR_ROLE_ID, roleRecords} from "@/resources/Roles/__stories__/RolesFixture"
import {CreateUser} from "./CreateUser"
import {PROFILE_ATTRIBUTES, PROFILE_GROUPS} from "./__stories__/UserProfileFixture"
import {type UserServiceOutcome, userRecords, userServices} from "./__stories__/UserFormFixture"

interface Scenario {
    /** A voter of the event, or a user of the tenant when unset. */
    electionEventId?: string
    outcome: UserServiceOutcome
    close: () => void
}

let graphql: ReturnType<typeof userServices>
let data: ReturnType<typeof userRecords>

const meta = {
    title: "Admin/User/CreateUser",
    component: CreateUser,
    args: {electionEventId: EVENT_ID, outcome: "saved", close: fn()},
    argTypes: {
        outcome: {control: "inline-radio", options: ["saved", "failure", "passwordFailure"]},
    },
    beforeEach: async ({args}) => {
        graphql = userServices(args.outcome)
        data = userRecords()
        await graphql.ready
    },
    parameters: {
        expectedFailure: {
            reason: "The password fields and the temporary switch have no accessible label, and the switch tooltip puts aria-label on a plain div.",
            a11y: ["aria-prohibited-attr", "label", "label-title-only"],
        },
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

function Fixture({electionEventId, close}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <CreateUser
                electionEventId={electionEventId}
                close={close}
                userAttributes={PROFILE_ATTRIBUTES}
                userAttributeGroups={PROFILE_GROUPS}
                rolesList={roleRecords()}
            />
        </AdminStoryProvider>
    )
}

const PASSWORD = "Synthetic-Passw0rd"

async function fillVoter(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(await canvas.findByRole("textbox", {name: /Username/}), "bob")
    await userEvent.type(canvas.getByRole("textbox", {name: "Email"}), "bob@example.test")
    // New users start enabled.
    expect(canvas.getByRole("checkbox", {name: "Enabled *"})).toBeChecked()
    const area = canvas.queryByRole("combobox", {name: /Area/})
    if (area) {
        await userEvent.type(area, "North district")
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "North district"})
        )
    }
    return canvas
}

const passwordFields = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll<HTMLInputElement>("input[type=password]"))

const created = () => graphql.calls.find((call) => call.name === "CreateUser")?.variables

export const NewVoter: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = await fillVoter(canvasElement)
        // Voters get their roles from the event, so the role list is not offered.
        expect(canvas.queryByRole("columnheader", {name: "Role"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(created()).toMatchObject({
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            user: {
                username: "bob",
                email: "bob@example.test",
                enabled: true,
                attributes: {"area-id": [STORY_IDS.area]},
            },
            userRolesIds: [],
        })
        expect(graphql.calls.map((call) => call.name)).toEqual(["CreateUser"])
        const notice = await within(document.body).findByText("Voter created")
        await waitFor(() => expect(notice).toBeVisible())
    },
}

export const NewVoterWithPassword: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = await fillVoter(canvasElement)
        const [password, repeat] = passwordFields(canvasElement)
        await userEvent.type(password, PASSWORD)
        await userEvent.type(repeat, PASSWORD)
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(graphql.calls.map((call) => call.name)).toEqual(["CreateUser", "EditUser"])
        expect(graphql.calls[1].variables).toEqual({
            body: {
                user_id: STORY_IDS.secondUser,
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                password: PASSWORD,
                temporary: true,
            },
        })
    },
}

export const MismatchedPasswords: Story = {
    play: async ({canvasElement}) => {
        const canvas = await fillVoter(canvasElement)
        const [password, repeat] = passwordFields(canvasElement)
        await userEvent.type(password, PASSWORD)
        await userEvent.type(repeat, "Something-else1")
        await expect(await canvas.findByText("Passwords must match")).toBeVisible()
        expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const NewTenantUserWithRole: Story = {
    args: {electionEventId: undefined},
    play: async ({args, canvasElement}) => {
        const canvas = await fillVoter(canvasElement)
        expect(canvas.queryByRole("combobox", {name: /Area/})).toBeNull()
        const auditor = (await canvas.findByRole("gridcell", {name: "auditor"})).closest(
            "[role=row]"
        )
        await userEvent.click(within(auditor as HTMLElement).getByRole("checkbox"))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(created()).toMatchObject({
            tenantId: TENANT_ID,
            user: {username: "bob"},
            userRolesIds: [AUDITOR_ROLE_ID],
        })
    },
}

export const CreateFailure: Story = {
    args: {outcome: "failure"},
    play: async ({args, canvasElement}) => {
        const canvas = await fillVoter(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        // Both the notification and the alert under the form report it.
        const reports = await within(document.body).findAllByText(/Error creating voter/)
        expect(reports).toHaveLength(2)
        await waitFor(() => reports.forEach((report) => expect(report).toBeVisible()))
        expect(graphql.calls.map((call) => call.name)).toEqual(["CreateUser"])
        // The form stays open so the voter can be created again.
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const PasswordFailureAfterCreate: Story = {
    args: {outcome: "passwordFailure"},
    play: async ({args, canvasElement}) => {
        const canvas = await fillVoter(canvasElement)
        const [password, repeat] = passwordFields(canvasElement)
        await userEvent.type(password, PASSWORD)
        await userEvent.type(repeat, PASSWORD)
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        const notice = await within(document.body).findByText(
            /Voter created, but their password could not be set/
        )
        await waitFor(() => expect(notice).toBeVisible())
        expect(graphql.calls.map((call) => call.name)).toEqual(["CreateUser", "EditUser"])
        // The voter exists, so the form closes rather than offering to create it twice.
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}
