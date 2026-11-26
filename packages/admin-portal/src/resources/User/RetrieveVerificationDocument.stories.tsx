// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {RetrieveVerificationDocument} from "./ManualVerification"
import {recovery} from "./__stories__/manualVerificationStoryFixture"

const makeBoundary = () =>
    graphqlBoundary({
        ManualVerification: () => ({
            data: {
                get_manual_verification_pdf: {
                    document_id: null,
                    status: "failed",
                    signing_request: null,
                },
            },
        }),
    })
let boundary: ReturnType<typeof makeBoundary>
const meta = {
    title: "Admin/User/RetrieveVerificationDocument",
    component: RetrieveVerificationDocument,
    beforeEach: () => {
        boundary = makeBoundary()
    },
    decorators: [
        (Story) => (
            <AdminStoryProvider boundary={boundary}>
                <Story />
            </AdminStoryProvider>
        ),
    ],
    args: {variables: recovery},
} satisfies Meta<typeof RetrieveVerificationDocument>
export default meta

export const RetriesSameRequest: StoryObj<typeof meta> = {
    async play({canvasElement}) {
        const button = await within(canvasElement).findByRole("button", {
            name: "Download verification document",
        })
        expect(boundary.calls).toEqual([])
        await userEvent.click(button)
        await waitFor(() => expect(boundary.calls).toHaveLength(1))
        await waitFor(() => expect(button).toBeEnabled())
        await userEvent.click(button)
        await waitFor(() => expect(boundary.calls).toHaveLength(2))
        expect(boundary.calls.map(({name, variables}) => ({name, variables}))).toEqual([
            {name: "ManualVerification", variables: recovery},
            {name: "ManualVerification", variables: recovery},
        ])
    },
}
