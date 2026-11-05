// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import EditPassword from "./EditPassword"
import {PASSWORD_POLICY_VIOLATION_ERROR_CODE} from "./editPasswordError"

interface Scenario {
    open: boolean
    /** What the user service answers. */
    outcome: "saved" | "policy" | "failure"
    handleClose: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/User/EditPassword",
    component: EditPassword,
    args: {open: true, outcome: "saved", handleClose: fn()},
    argTypes: {outcome: {control: "inline-radio", options: ["saved", "policy", "failure"]}},
    parameters: {
        expectedFailure: {
            reason: "The password fields and the temporary switch have no accessible label, and the switch tooltip puts aria-label on a plain div.",
            a11y: ["aria-prohibited-attr", "label", "label-title-only"],
        },
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                EditUser: () => {
                    if (args.outcome === "failure")
                        throw new Error("Synthetic user service failure")
                    if (args.outcome === "policy") {
                        return {
                            errors: [
                                new GraphQLError("Password policy violation", {
                                    extensions: {
                                        code: PASSWORD_POLICY_VIOLATION_ERROR_CODE,
                                        password_policy_rule: "minimumLength",
                                        password_policy_required_count: 12,
                                    },
                                }),
                            ],
                        }
                    }
                    return {data: {edit_user: {user: {id: STORY_IDS.user}, task_execution: null}}}
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({open, handleClose}) => (
        <AdminStoryProvider boundary={graphql}>
            <EditPassword
                open={open}
                handleClose={handleClose}
                id={STORY_IDS.user}
                electionEventId={STORY_IDS.event}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const PASSWORD = "Synthetic-Passw0rd"

async function fields() {
    const dialog = within(await within(document.body).findByRole("dialog"))
    const [password, repeat] = dialog
        .getAllByDisplayValue("")
        .filter((input) => input.getAttribute("type") === "password")
    return {dialog, password, repeat}
}

async function enterPassword(first: string, second: string) {
    const form = await fields()
    await userEvent.type(form.password, first)
    await userEvent.type(form.repeat, second)
    return form
}

const saved = (temporary: boolean) => ({
    name: "EditUser",
    variables: {
        body: {
            user_id: STORY_IDS.user,
            tenant_id: TENANT_ID,
            election_event_id: STORY_IDS.event,
            password: PASSWORD,
            temporary,
        },
    },
    headers: {},
})

export const Populated: Story = {
    play: async () => {
        const {dialog, password, repeat} = await fields()
        const title = dialog.getByRole("heading", {name: "Change password"})
        await waitFor(() => expect(title).toBeVisible())
        await expect(password).toBeVisible()
        await expect(repeat).toBeVisible()
        expect(dialog.getByRole("switch")).toBeChecked()
        expect(graphql.calls).toEqual([])
    },
}

export const Closed: Story = {
    args: {open: false},
    parameters: {expectedFailure: null},
    play: async () => {
        expect(within(document.body).queryByRole("dialog")).toBeNull()
    },
}

export const MismatchedPasswords: Story = {
    play: async () => {
        const {dialog} = await enterPassword(PASSWORD, "Something-else1")
        await expect(await dialog.findByText("Passwords must match")).toBeVisible()
        expect(dialog.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const SaveATemporaryPassword: Story = {
    play: async ({args}) => {
        const {dialog} = await enterPassword(PASSWORD, PASSWORD)
        await userEvent.click(dialog.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(graphql.calls).toEqual([saved(true)]))
        const notice = await within(document.body).findByText("Voter edited")
        await waitFor(() => expect(notice).toBeVisible())
        expect(args.handleClose).toHaveBeenCalledTimes(1)
    },
}

export const SaveAPermanentPassword: Story = {
    play: async ({args}) => {
        const {dialog} = await enterPassword(PASSWORD, PASSWORD)
        await userEvent.click(dialog.getByRole("switch"))
        await userEvent.click(dialog.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(graphql.calls).toEqual([saved(false)]))
        await waitFor(() => expect(args.handleClose).toHaveBeenCalledTimes(1))
    },
}

export const PasswordPolicyRejection: Story = {
    args: {outcome: "policy"},
    play: async ({args}) => {
        const {dialog} = await enterPassword(PASSWORD, PASSWORD)
        await userEvent.click(dialog.getByRole("button", {name: "Save"}))
        const message = "The password minimum length is 12."
        // The rule is shown under the password and the dialog stays open to correct it.
        await expect(await dialog.findByText(message)).toBeVisible()
        expect(graphql.calls).toEqual([saved(true)])
        expect(args.handleClose).not.toHaveBeenCalled()
    },
}

export const ServiceFailure: Story = {
    args: {outcome: "failure"},
    play: async ({args}) => {
        const {dialog} = await enterPassword(PASSWORD, PASSWORD)
        await userEvent.click(dialog.getByRole("button", {name: "Save"}))
        const notice = await within(document.body).findByText("Error editing voter")
        await waitFor(() => expect(notice).toBeVisible())
        expect(graphql.calls).toEqual([saved(true)])
        expect(args.handleClose).toHaveBeenCalledTimes(1)
    },
}
