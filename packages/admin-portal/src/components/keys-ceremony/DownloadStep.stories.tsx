// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {recordDownloads} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {STORY_TRUSTEE} from "@/__stories__/storyAuth"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
} from "@/services/KeyCeremony"
import {PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE} from "@/services/privateKeyDownloadError"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {DownloadStep} from "./DownloadStep"
import {
    KEYS_CEREMONY_RESOURCE,
    PRIVATE_KEY,
    ceremony,
    ceremonyEvent,
} from "./__stories__/KeysCeremonyFixture"

interface Scenario {
    /** What the private key service answers. */
    key: "key" | "empty" | "unavailable" | "failure"
    /** The ceremony's execution status. */
    execution: EStatus
    /** The signed-in trustee's status in the ceremony. */
    trustee: TStatus
    goNext: () => void
    goBack: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let saved: ReturnType<typeof recordDownloads>

const currentCeremony = ({execution, trustee}: Pick<Scenario, "execution" | "trustee">) =>
    ceremony(execution, [trustee, TStatus.KEY_GENERATED, TStatus.KEY_GENERATED])

const meta = {
    title: "Admin/Keys ceremony/DownloadStep",
    component: DownloadStep,
    args: {
        key: "key",
        execution: EStatus.IN_PROGRESS,
        trustee: TStatus.KEY_GENERATED,
        goNext: fn(),
        goBack: fn(),
    },
    argTypes: {
        key: {control: "inline-radio", options: ["key", "empty", "unavailable", "failure"]},
        execution: {control: "select", options: Object.values(EStatus)},
        trustee: {control: "select", options: Object.values(TStatus)},
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({[KEYS_CEREMONY_RESOURCE]: [currentCeremony(args)]})
        graphql = graphqlBoundary(
            {
                GetPrivateKey: () => {
                    if (args.key === "failure") throw new Error("Synthetic key service failure")
                    if (args.key === "unavailable") {
                        return {
                            errors: [
                                new GraphQLError("The ceremony has moved on", {
                                    extensions: {code: PRIVATE_KEY_DOWNLOAD_UNAVAILABLE_ERROR_CODE},
                                }),
                            ],
                        }
                    }
                    return {
                        data: {
                            get_private_key: {
                                private_key_base64: args.key === "empty" ? "" : PRIVATE_KEY,
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
        saved = recordDownloads()
        return saved.restore
    },
    render: ({key: _key, execution, trustee, ...args}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={EStoryPermissions.TRUSTEE}
        >
            <DownloadStep
                electionEvent={ceremonyEvent()}
                currentCeremony={currentCeremony({execution, trustee})}
                {...args}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const downloadButton = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("button", {name: "Download your Encrypted Private Key"})

async function download(canvasElement: HTMLElement) {
    const button = await downloadButton(canvasElement)
    // The latest ceremony status decides whether the key can still be downloaded.
    await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
    await userEvent.click(button)
}

const keyRequest = {
    name: "GetPrivateKey",
    variables: {electionEventId: EVENT_ID, keysCeremonyId: STORY_IDS.keysCeremony},
    headers: {},
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await downloadButton(canvasElement)).toBeEnabled()
        await expect(
            canvas.getByRole("heading", {name: "Download Encrypted Private Key"})
        ).toBeVisible()
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        await waitFor(() =>
            expect(data.calls[0]).toEqual({
                method: "getOne",
                args: [
                    KEYS_CEREMONY_RESOURCE,
                    expect.objectContaining({id: STORY_IDS.keysCeremony}),
                ],
            })
        )
        expect(graphql.calls).toEqual([])
    },
}

export const DownloadAndConfirmBackups: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await download(canvasElement)
        await expect(
            await canvas.findByText("Encrypted Private Key downloaded successfully.")
        ).toBeVisible()
        expect(graphql.calls).toEqual([keyRequest])
        expect(saved.downloads).toEqual([
            {
                name: `encrypted_private_key_trustee_${STORY_TRUSTEE}_Council.txt`,
                href: expect.stringMatching(/^blob:/),
            },
        ])
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        const confirm = dialog.getByRole("button", {name: "Confirm Backups and Continue"})
        expect(confirm).toBeDisabled()
        await userEvent.click(dialog.getByRole("checkbox", {name: "First backup secured"}))
        expect(confirm).toBeDisabled()
        await userEvent.click(dialog.getByRole("checkbox", {name: "Second backup secured"}))
        await userEvent.click(confirm)
        expect(args.goNext).toHaveBeenCalledTimes(1)
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const CancelBackupConfirmation: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await download(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Next"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("checkbox", {name: "First backup secured"}))
        await userEvent.click(dialog.getByRole("button", {name: "Go Back"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        const reopened = within(await within(document.body).findByRole("dialog"))
        // Cancelling clears the acknowledgements.
        expect(reopened.getByRole("checkbox", {name: "First backup secured"})).not.toBeChecked()
        expect(args.goNext).not.toHaveBeenCalled()
        await userEvent.click(reopened.getByRole("button", {name: "Go Back"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const EmptyKey: Story = {
    args: {key: "empty"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await download(canvasElement)
        await expect(await canvas.findByText("Download error, empty file")).toBeVisible()
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(saved.downloads).toEqual([])
        expect(graphql.calls).toEqual([keyRequest])
    },
}

export const KeyServiceFailure: Story = {
    args: {key: "failure"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await download(canvasElement)
        await expect(
            await canvas.findByText("The private key could not be downloaded. Please try again.")
        ).toBeVisible()
        // The download can be retried.
        await expect(await downloadButton(canvasElement)).toBeEnabled()
        expect(saved.downloads).toEqual([])
    },
}

export const ServiceReportsTheCeremonyMovedOn: Story = {
    args: {key: "unavailable"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await download(canvasElement)
        await expect(
            await canvas.findByText(
                "Private key download is no longer available because the ceremony has moved on."
            )
        ).toBeVisible()
        await expect(await downloadButton(canvasElement)).toBeDisabled()
        expect(graphql.calls).toEqual([keyRequest])
        expect(saved.downloads).toEqual([])
    },
}

export const AlreadyVerified: Story = {
    args: {trustee: TStatus.KEY_CHECKED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("Your private key was already downloaded and verified.")
        ).toBeVisible()
        await expect(await downloadButton(canvasElement)).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const CeremonyFinished: Story = {
    args: {execution: EStatus.SUCCESS, trustee: TStatus.KEY_RETRIEVED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "Private key download is no longer available because the ceremony has moved on."
            )
        ).toBeVisible()
        await expect(await downloadButton(canvasElement)).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const Back: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(await within(canvasElement).findByRole("button", {name: "Back"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}
