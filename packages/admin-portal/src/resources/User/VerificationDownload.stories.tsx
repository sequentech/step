// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, waitFor} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {VerificationDownload} from "./ManualVerification"
import {DOCUMENT, documentBoundary} from "./__stories__/manualVerificationStoryFixture"

let boundary: ReturnType<typeof documentBoundary>
const meta = {
    title: "Admin/User/VerificationDownload",
    component: VerificationDownload,
    beforeEach: () => {
        boundary = documentBoundary()
    },
    decorators: [
        (Story) => (
            <AdminStoryProvider boundary={boundary}>
                <Story />
            </AdminStoryProvider>
        ),
    ],
    args: {documentId: DOCUMENT, electionEventId: EVENT_ID, onDone: fn()},
} satisfies Meta<typeof VerificationDownload>
export default meta

export const ScopedDownload: StoryObj<typeof meta> = {
    async play({args}) {
        await waitFor(() => expect(args.onDone).toHaveBeenCalledTimes(1))
        expect(boundary.calls).toEqual(
            expect.arrayContaining([
                expect.objectContaining({
                    name: "GetDocument",
                    variables: {id: DOCUMENT, tenantId: TENANT_ID},
                }),
                expect.objectContaining({
                    name: "FetchDocument",
                    variables: {documentId: DOCUMENT, electionEventId: EVENT_ID},
                }),
            ])
        )
    },
}
