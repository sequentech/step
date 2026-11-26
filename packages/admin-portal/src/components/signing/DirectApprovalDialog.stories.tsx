// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {ISigningPanelData} from "@/lib/signing/api"
import {SigningAction} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {DirectApprovalDialog} from "./DirectApprovalDialog"
import {fakeApi, makePanel, MARIA, signedInAs} from "./__stories__/fixtures"

interface Scenario {
    onClose: () => void
}

let data: ISigningPanelData
let reload: () => Promise<ISigningPanelData>
const refusal = "Signing is unavailable for this request. No approval was submitted here."

const meta = {
    title: "Admin/Signing/DirectApprovalDialog",
    component: DirectApprovalDialog,
    args: {onClose: fn()},
    beforeEach: async () => {
        data = await makePanel({
            action: SigningAction.InitializeVoting,
            signingMode: "hsm-demo-raw",
            subject: {},
            required: 2,
        })
        reload = fn(async () => data)
    },
    render: ({onClose}) => (
        <AdminStoryProvider
            boundary={graphqlBoundary({})}
            roles={[IPermissions.SIGN_INITIALIZE_VOTING]}
            auth={signedInAs(MARIA)}
        >
            <DirectApprovalDialog
                data={data}
                api={fakeApi(data)}
                reload={reload}
                attempts={new Map()}
                onClose={onClose}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const RefusesUnreviewableRequest: Story = {
    play: async ({args}) => {
        const element = await within(document.body).findByRole("dialog", {name: /^Sign the /})
        const dialog = within(element)
        // The synthetic request does not name a reviewed whole Post. The ordinary
        // Storybook origin is not the dedicated signing origin either.
        await waitFor(() => expect(dialog.getByText(refusal)).toBeVisible())
        await expect(dialog.getByRole("button", {name: "Sign"})).toBeDisabled()
        expect(reload).not.toHaveBeenCalled()
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        expect(args.onClose).toHaveBeenCalledTimes(1)
    },
}
