// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {IMessagingAccount} from "@/types/messaging"
import {MessagingAccountEditor} from "./MessagingAccountEditor"
import {
    MESSENGER_ACCOUNT_ID,
    messengerAccount,
    viberAccount,
    whatsappAccount,
} from "./__stories__/MessagingFixture"

enum EStoryAccount {
    NEW = "NEW",
    WHATSAPP = "WHATSAPP",
    VIBER = "VIBER",
    MESSENGER = "MESSENGER",
}

interface Scenario {
    account: EStoryAccount
    canWrite: boolean
    mutationsFail: boolean
}

const ACCOUNTS: Record<EStoryAccount, () => IMessagingAccount | undefined> = {
    [EStoryAccount.NEW]: () => undefined,
    [EStoryAccount.WHATSAPP]: whatsappAccount,
    [EStoryAccount.VIBER]: viberAccount,
    [EStoryAccount.MESSENGER]: messengerAccount,
}

let graphql: ReturnType<typeof graphqlBoundary>
const onChanged = fn()
const onClose = fn()

const reply = (fails: boolean, data: Record<string, unknown>) => () => {
    if (fails) throw new Error("Synthetic messaging service unavailable")
    return {data}
}

const meta = {
    title: "Admin/Settings/MessagingAccountEditor",
    component: MessagingAccountEditor,
    args: {account: EStoryAccount.WHATSAPP, canWrite: true, mutationsFail: false},
    argTypes: {account: {control: "inline-radio", options: Object.values(EStoryAccount)}},
    parameters: {widgets: ["CredentialInput", "CallbackSection"]},
    beforeEach: async ({args}) => {
        onChanged.mockClear()
        onClose.mockClear()
        graphql = graphqlBoundary({
            UpsertMessagingAccount: reply(args.mutationsFail, {
                upsert_messaging_account: {id: MESSENGER_ACCOUNT_ID},
            }),
            ReplaceMessagingAccountCredentials: reply(args.mutationsFail, {
                replace_messaging_account_credentials: {
                    replaced: ["VERIFY_TOKEN"],
                    verify_token: "synthetic-verify-token",
                },
            }),
        })
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider
            boundary={graphql}
            roles={args.canWrite ? [IPermissions.MESSAGING_ACCOUNT_WRITE] : []}
        >
            <MessagingAccountEditor
                key={args.account}
                account={ACCOUNTS[args.account]()}
                canWrite={args.canWrite}
                onChanged={onChanged}
                onClose={onClose}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WhatsApp: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("textbox", {name: /WhatsApp Business Account ID/})
        ).toHaveValue("100000000000001")
        await expect(canvas.getAllByText(/Set · replaced/).length).toBeGreaterThan(0)
        expect(canvas.queryByLabelText("Access token")).toBeNull()
        await expect(canvas.getByText(/While approval is pending/)).toBeVisible()
        await userEvent.clear(canvas.getByRole("textbox", {name: /Phone number ID/}))
        await userEvent.type(canvas.getByRole("textbox", {name: /Phone number ID/}), "300")
        await expect(canvas.getByText(/Messages will come from another number/)).toBeVisible()
    },
}

export const GenerateVerifyToken: Story = {
    parameters: {widgets: ["CallbackSection", "VerifyTokenDialog"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: "Callback path"})).toHaveValue(
            "/webhooks/meta/wa-hook-key"
        )
        await userEvent.click(canvas.getByRole("button", {name: "Generate verify token"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByRole("textbox", {name: "Verify token"})).toHaveValue(
            "synthetic-verify-token"
        )
        expect(graphql.calls[0]).toMatchObject({
            name: "ReplaceMessagingAccountCredentials",
            variables: {credentials: {}, generateVerifyToken: true},
            headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_WRITE},
        })
        await userEvent.click(dialog.getByRole("button", {name: "Done"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const ViberTemplates: Story = {
    args: {account: EStoryAccount.VIBER},
    parameters: {widgets: ["ViberTemplatesEditor", "CallbackSection", "CredentialInput"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByRole("textbox", {name: "Partner template ID"})).toHaveLength(3)
        await expect(canvas.getByRole("textbox", {name: "Callback path"})).toHaveValue(
            "/webhooks/viber/viber-hook-key"
        )
        expect(canvas.queryByRole("button", {name: "Generate verify token"})).toBeNull()
    },
}

export const ChangeMessengerPage: Story = {
    args: {account: EStoryAccount.MESSENGER},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const page = canvas.getByRole("textbox", {name: /Facebook Page ID/})
        await userEvent.clear(page)
        await userEvent.type(page, "999")
        await expect(
            canvas.getByText(/voters connected to Council Elections get codes only after/)
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(onClose).toHaveBeenCalled())
        expect(graphql.calls[0]).toMatchObject({
            name: "UpsertMessagingAccount",
            variables: {sender: {provider: "MESSENGER_SEND_API", page_id: "999"}},
        })
        expect(onChanged).toHaveBeenCalled()
    },
}

export const NewAccount: Story = {
    args: {account: EStoryAccount.NEW},
    parameters: {widgets: ["CredentialInput"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: /From address/})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await expect(canvas.getAllByText("Required").length).toBeGreaterThan(0)
        expect(graphql.calls).toEqual([])
    },
}

export const SaveFails: Story = {
    args: {account: EStoryAccount.MESSENGER, mutationsFail: true},
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        const notice = await within(document.body).findByText("The account could not be saved.")
        await waitFor(() => expect(notice).toBeVisible())
        expect(onClose).not.toHaveBeenCalled()
    },
}

export const ReadOnly: Story = {
    args: {canWrite: false},
    parameters: {
        expectedFailure: {
            reason: "Disabled fields draw their helper text in the theme's disabled grey.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox", {name: /Phone number ID/})).toBeDisabled()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Replace"})).toBeNull()
    },
}
