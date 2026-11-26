// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {SigningRequestStatus} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {ManualVerificationCompletion} from "./ManualVerification"
import {ACTOR, OTHER_ACTOR, panel} from "./__stories__/manualVerificationStoryFixture"

let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/User/ManualVerificationCompletion",
    component: ManualVerificationCompletion,
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    args: {data: panel(SigningRequestStatus.Executed)},
    decorators: [
        (Story, {parameters}) => (
            <AdminStoryProvider
                boundary={boundary}
                auth={{userId: parameters.otherActor ? OTHER_ACTOR : ACTOR}}
                roles={parameters.noPermission ? [] : [IPermissions.VOTER_MANUALLY_VERIFY]}
            >
                <Story />
            </AdminStoryProvider>
        ),
    ],
} satisfies Meta<typeof ManualVerificationCompletion>
export default meta

export const ExecutedForRequester: StoryObj<typeof meta> = {
    async play({canvasElement}) {
        expect(
            await within(canvasElement).findByRole("button", {
                name: "Download verification document",
            })
        ).toBeEnabled()
        expect(boundary.calls).toEqual([])
    },
}

export const HiddenFromOtherActor: StoryObj<typeof meta> = {
    parameters: {otherActor: true},
    async play({canvasElement}) {
        expect(
            within(canvasElement).queryByRole("button", {name: "Download verification document"})
        ).not.toBeInTheDocument()
        expect(boundary.calls).toEqual([])
    },
}

export const HiddenBeforeExecution: StoryObj<typeof meta> = {
    ...HiddenFromOtherActor,
    parameters: {},
    args: {data: panel(SigningRequestStatus.Waiting)},
}

export const HiddenWithoutPermission: StoryObj<typeof meta> = {
    ...HiddenFromOtherActor,
    parameters: {noPermission: true},
}
