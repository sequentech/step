// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ErrorPage} from "../ErrorPage"
import {VotingPortalError, VotingPortalErrorType} from "../../services/VotingPortalError"

const meta = {
    title: "Voting/Error page",
    parameters: {
        router: {
            path: "/tenant/:tenantId/event/:eventId",
            initialEntries: ["/tenant/tenant-1/event/event-1?lang=en"],
            errorElement: (
                <>
                    <ErrorPage />
                </>
            ),
        },
    },
    render: () => <></>,
} satisfies Meta
export default meta
type Story = StoryObj<typeof meta>

function errorStory(type: VotingPortalErrorType, title = "Oops! Unexpected Error"): Story {
    return {
        parameters: {
            router: {
                loader: () => {
                    throw new VotingPortalError(type)
                },
            },
        },
        play: async ({canvasElement}) => {
            const canvas = within(canvasElement)
            await expect(await canvas.findByRole("heading", {name: title})).toBeVisible()
            if (title === "Oops! Unexpected Error")
                await expect(canvas.getByText(type, {exact: true})).toBeVisible()
            if (type === VotingPortalErrorType.NO_ELECTION_EVENT)
                await expect(canvas.queryByRole("button", {name: "Back"})).not.toBeInTheDocument()
            else await expect(canvas.getByRole("button", {name: "Back"})).toBeVisible()
        },
    }
}

export const NoElectionEvent = errorStory(VotingPortalErrorType.NO_ELECTION_EVENT)
export const InternalError = errorStory(VotingPortalErrorType.INTERNAL_ERROR)
export const FetchFailure = errorStory(VotingPortalErrorType.UNABLE_TO_FETCH_DATA)
export const EncryptionFailure = errorStory(VotingPortalErrorType.UNABLE_TO_ENCRYPT_BALLOT)
export const CastFailure = errorStory(VotingPortalErrorType.UNABLE_TO_CAST_BALLOT)
export const MissingBallot = errorStory(VotingPortalErrorType.NO_BALLOT_STYLE)
export const HashMismatch = errorStory(VotingPortalErrorType.INCONSISTENT_HASH)
export const CertificateFailure = errorStory(
    VotingPortalErrorType.CERT_AUTH_FAILED,
    "Certificate Authentication Failed"
)
export const InvalidLoginHints = errorStory(
    VotingPortalErrorType.INVALID_LOGIN_HINT_PARAMETERS,
    "Invalid voting link"
)

export const HttpFailure: Story = {
    parameters: {
        router: {
            loader: () => {
                throw new Response(JSON.stringify({message: "Synthetic missing election"}), {
                    status: 404,
                    statusText: "Not Found",
                    headers: {"Content-Type": "application/json"},
                })
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {name: /404/})).toBeVisible()
        await expect(canvas.getByText("Not Found", {exact: true})).toBeVisible()
        await expect(canvas.getByText("Synthetic missing election")).toBeVisible()
    },
}

export const UnexpectedFailure: Story = {
    parameters: {
        router: {
            loader: () => {
                throw new Error("Synthetic unexpected failure")
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {name: "Oops! Unexpected Error"})
        ).toBeVisible()
        await expect(canvas.getByText("Synthetic unexpected failure")).toBeVisible()
    },
}
