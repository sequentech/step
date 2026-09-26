// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {
    ReportPasswordDialog,
    reportDecryptionCommand,
    type ReportDocumentAccess,
} from "./ReportPasswordDialog"

interface Scenario {
    roles: string[]
    access?: ReportDocumentAccess
    /** What the password lookup does. */
    lookup: "password" | "error" | "loading"
    onClose: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>

const DOCUMENT_ID = storyId(10, 1)
const PASSWORD = "Rp7-synthetic-Lm4"
const ENCRYPTED = {password_secret_id: storyId(10, 2)}
const PASSWORD_ROLES = [IPermissions.DOCUMENT_PASSWORD_READ, IPermissions.DOCUMENT_DOWNLOAD]

const meta = {
    title: "Admin/Reports/ReportPasswordDialog",
    component: ReportPasswordDialog,
    args: {roles: PASSWORD_ROLES, access: ENCRYPTED, lookup: "password", onClose: fn()},
    argTypes: {
        lookup: {control: "inline-radio", options: ["password", "error", "loading"]},
        onClose: {table: {disable: true}},
    },
    parameters: {
        widgets: ["PasswordDialog", "DecryptHelp"],
        expectedFailure: {
            reason: "The read-only password and decryption command fields have no label.",
            a11y: ["label"],
        },
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                GetDocumentPassword: () =>
                    args.lookup === "loading"
                        ? new Promise(() => {})
                        : args.lookup === "error"
                          ? {errors: [new GraphQLError("Synthetic secret store unavailable")]}
                          : {data: {get_document_password: {password: PASSWORD}}},
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({roles, access, onClose}) => (
        <AdminStoryProvider boundary={graphql} roles={roles}>
            <ReportPasswordDialog documentId={DOCUMENT_ID} access={access} onClose={onClose} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function dialog(name: string) {
    const element = await within(document.body).findByRole("dialog", {name})
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

const helpOnly = {widgets: ["DecryptHelp"]}

export const RevealThePassword: Story = {
    play: async () => {
        const password = await dialog("Password")
        await expect(password.getByDisplayValue(PASSWORD)).toHaveAttribute("readonly")
        await expect(password.getByDisplayValue(reportDecryptionCommand)).toBeVisible()
        expect(graphql.calls).toEqual([
            {
                name: "GetDocumentPassword",
                variables: {documentId: DOCUMENT_ID},
                headers: expect.objectContaining({"x-hasura-role": "document-password-read"}),
            },
        ])
    },
}

export const CloseWithOk: Story = {
    play: async ({args}) => {
        const password = await dialog("Password")
        await userEvent.click(password.getByRole("button", {name: "Ok"}))
        expect(args.onClose).toHaveBeenCalled()
    },
}

export const SecretAttributesWithPermission: Story = {
    args: {
        access: {...ENCRYPTED, voter_secret_attributes: true},
        roles: [...PASSWORD_ROLES, IPermissions.VOTER_SECRET_ATTRIBUTE_READ],
    },
    play: async () => {
        const password = await dialog("Password")
        await expect(password.getByDisplayValue(PASSWORD)).toBeVisible()
    },
}

export const SecretAttributesWithoutPermission: Story = {
    args: {access: {...ENCRYPTED, voter_secret_attributes: true}},
    parameters: helpOnly,
    play: async () => {
        const help = await dialog("How to decrypt the file")
        await expect(help.getByText("The PDF password could not be retrieved")).toBeVisible()
        await expect(help.getByDisplayValue(reportDecryptionCommand)).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

export const WithoutPasswordPermission: Story = {
    args: {roles: [IPermissions.DOCUMENT_DOWNLOAD]},
    parameters: helpOnly,
    play: async () => {
        const help = await dialog("How to decrypt the file")
        await expect(help.getByText("The PDF password could not be retrieved")).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

export const ReportWithoutSavedPassword: Story = {
    args: {access: undefined},
    parameters: helpOnly,
    play: async ({args}) => {
        const help = await dialog("How to decrypt the file")
        await expect(help.getByDisplayValue(reportDecryptionCommand)).toBeVisible()
        expect(help.queryByText("The PDF password could not be retrieved")).toBeNull()
        expect(graphql.calls).toEqual([])
        await userEvent.click(help.getByRole("button", {name: "Ok"}))
        expect(args.onClose).toHaveBeenCalled()
    },
}

export const LookupFailure: Story = {
    args: {lookup: "error"},
    parameters: helpOnly,
    play: async () => {
        const help = await dialog("How to decrypt the file")
        await waitFor(() =>
            expect(help.getByText("The PDF password could not be retrieved")).toBeVisible()
        )
        expect(help.queryByDisplayValue(PASSWORD)).toBeNull()
    },
}

export const LookingUpThePassword: Story = {
    args: {lookup: "loading"},
    parameters: {
        ...helpOnly,
        expectedFailure: {
            reason:
                "The lookup spinner has no accessible name and the decryption command field " +
                "has no label.",
            a11y: ["aria-progressbar-name", "label"],
        },
    },
    play: async () => {
        const help = await dialog("How to decrypt the file")
        await expect(help.getByRole("progressbar")).toBeVisible()
        expect(help.queryByText("The PDF password could not be retrieved")).toBeNull()
    },
}
