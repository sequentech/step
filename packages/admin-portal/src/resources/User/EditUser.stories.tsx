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
import {
    AUDITOR_ROLE_ID,
    MANAGER_ROLE_ID,
    roleRecords,
} from "@/resources/Roles/__stories__/RolesFixture"
import {EditUser} from "./EditUser"
import {PROFILE_ATTRIBUTES, PROFILE_GROUPS} from "./__stories__/UserProfileFixture"
import {type UserServiceOutcome, userRecords, userServices} from "./__stories__/UserFormFixture"

interface Scenario {
    /** A voter of the event, or a user of the tenant when unset. */
    electionEventId?: string
    outcome: UserServiceOutcome
    /** Replaces the admin's roles, e.g. to take away voter-write. */
    roles?: string[]
    close: () => void
}

const record = {
    id: STORY_IDS.user,
    username: "alice",
    email: "alice@example.test",
    first_name: "Alice",
    enabled: true,
    email_verified: true,
    attributes: {"city": ["Springfield"], "area-id": [STORY_IDS.area]},
    area: {id: STORY_IDS.area, name: "North district"},
}

let graphql: ReturnType<typeof userServices>
let data: ReturnType<typeof userRecords>

const meta = {
    title: "Admin/User/EditUser",
    component: EditUser,
    args: {electionEventId: EVENT_ID, outcome: "saved", close: fn()},
    argTypes: {outcome: {control: "inline-radio", options: ["saved", "failure"]}},
    beforeEach: async ({args}) => {
        graphql = userServices(args.outcome, [AUDITOR_ROLE_ID])
        data = userRecords()
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

function Fixture({electionEventId, roles, close}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            roles={roles}
        >
            <EditUser
                id={STORY_IDS.user}
                electionEventId={electionEventId}
                close={close}
                rolesList={roleRecords()}
                userAttributes={PROFILE_ATTRIBUTES}
                userAttributeGroups={PROFILE_GROUPS}
                record={record}
            />
        </AdminStoryProvider>
    )
}

async function loaded(canvasElement: HTMLElement, voter = true) {
    const canvas = within(canvasElement)
    await canvas.findByDisplayValue("alice")
    await waitFor(() => expect(graphql.calls.map((call) => call.name)).toContain("ListUserRoles"))
    // Fields of a voter stay locked until their cast votes show they have not voted.
    if (voter)
        await waitFor(() =>
            expect(data.calls.map(({args}) => args[0])).toContain("sequent_backend_cast_vote")
        )
    return canvas
}

const email = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("textbox", {name: "Email"})

async function reviewEmailChange(canvasElement: HTMLElement) {
    const canvas = await loaded(canvasElement)
    await waitFor(() => expect(email(canvasElement)).toBeEnabled())
    await userEvent.clear(email(canvasElement))
    await userEvent.type(email(canvasElement), "alice@new.example.test")
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
    await expect(await canvas.findByRole("heading", {name: "Review changes"})).toBeVisible()
    return canvas
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await waitFor(() => expect(email(canvasElement)).toBeEnabled())
        expect(email(canvasElement)).toHaveValue("alice@example.test")
        expect(canvas.getByRole("textbox", {name: "City"})).toHaveValue("Springfield")
        expect(canvas.getByRole("textbox", {name: /Username/})).toBeDisabled()
        expect(graphql.calls[0]).toEqual({
            name: "ListUserRoles",
            variables: {tenantId: TENANT_ID, userId: STORY_IDS.user, electionEventId: EVENT_ID},
            headers: {},
        })
    },
}

export const ReviewAndConfirm: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = await reviewEmailChange(canvasElement)
        expect(canvas.getByText("alice@new.example.test")).toBeVisible()
        expect(graphql.calls.map((call) => call.name)).toEqual(["ListUserRoles"])
        await userEvent.click(canvas.getByRole("button", {name: "Confirm changes"}))
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        expect(graphql.calls.find((call) => call.name === "EditUser")?.variables).toMatchObject({
            body: {
                user_id: STORY_IDS.user,
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                email: "alice@new.example.test",
            },
        })
        const notice = await within(document.body).findByText("Voter edited")
        await waitFor(() => expect(notice).toBeVisible())
    },
}

export const BackToEditFromReview: Story = {
    play: async ({args, canvasElement}) => {
        const canvas = await reviewEmailChange(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Edit"}))
        await waitFor(() => expect(email(canvasElement)).toBeVisible())
        expect(email(canvasElement)).toHaveValue("alice@new.example.test")
        expect(graphql.calls.map((call) => call.name)).toEqual(["ListUserRoles"])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const NothingChanged: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        const notice = await within(document.body).findByText("No changes to review")
        await waitFor(() => expect(notice).toBeVisible())
        expect(canvas.queryByRole("heading", {name: "Review changes"})).toBeNull()
        expect(graphql.calls.map((call) => call.name)).toEqual(["ListUserRoles"])
    },
}

export const EditFailure: Story = {
    args: {outcome: "failure"},
    play: async ({args, canvasElement}) => {
        const canvas = await reviewEmailChange(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Confirm changes"}))
        const reports = await within(document.body).findAllByText(/Error editing voter/)
        await waitFor(() => expect(reports[0]).toBeVisible())
        expect(graphql.calls.map((call) => call.name)).toEqual(["ListUserRoles", "EditUser"])
        // The review stays open to retry.
        expect(canvas.getByRole("button", {name: "Confirm changes"})).toBeEnabled()
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const WithoutEditPermission: Story = {
    args: {roles: ["voter-read"]},
    parameters: {
        expectedFailure: {
            reason: "The length hint of a disabled field is grey on white, below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(email(canvasElement)).toBeDisabled()
        expect(canvas.getByRole("textbox", {name: "City"})).toBeDisabled()
        expect(canvas.getByRole("checkbox", {name: "Enabled *"})).toBeDisabled()
    },
}

export const TenantUserRoles: Story = {
    args: {electionEventId: undefined},
    play: async ({args, canvasElement}) => {
        const canvas = await loaded(canvasElement, false)
        const row = async (name: string) =>
            within(
                (await canvas.findByRole("gridcell", {name})).closest("[role=row]") as HTMLElement
            ).getByRole("checkbox")
        await waitFor(async () => expect(await row("auditor")).toBeChecked())
        await userEvent.click(await row("auditor"))
        await userEvent.click(await row("voter-manager"))
        await waitFor(async () => expect(await row("voter-manager")).toBeChecked())
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await userEvent.click(await canvas.findByRole("button", {name: "Confirm changes"}))
        await waitFor(() => expect(args.close).toHaveBeenCalledTimes(1))
        const roleCalls = graphql.calls.filter((call) => call.name.endsWith("UserRole"))
        expect(roleCalls.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "DeleteUserRole",
                variables: {tenantId: TENANT_ID, roleId: AUDITOR_ROLE_ID, userId: STORY_IDS.user},
            },
            {
                name: "SetUserRole",
                variables: {tenantId: TENANT_ID, roleId: MANAGER_ROLE_ID, userId: STORY_IDS.user},
            },
        ])
    },
}
