// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {IKeysCeremonyExecutionStatus, IKeysCeremonyTrusteeStatus} from "@/services/KeyCeremony"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {CheckStep} from "./CheckStep"
import {PRIVATE_KEY, ceremony, ceremonyEvent} from "./__stories__/KeysCeremonyFixture"

interface Scenario {
    /** What checking a key does. */
    check: "service" | "failure"
    goNext: () => void
    goBack: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Keys ceremony/CheckStep",
    component: CheckStep,
    args: {check: "service", goNext: fn(), goBack: fn()},
    argTypes: {check: {control: "inline-radio", options: ["service", "failure"]}},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                CheckPrivateKey: ({variables}) => {
                    if (args.check === "failure") throw new Error("Synthetic check unavailable")
                    return {
                        data: {
                            check_private_key: {
                                is_valid: variables.privateKeyBase64 === PRIVATE_KEY,
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({check: _check, ...args}) => (
        <AdminStoryProvider boundary={graphql} role={EStoryPermissions.TRUSTEE}>
            <CheckStep
                electionEvent={ceremonyEvent()}
                currentCeremony={ceremony(IKeysCeremonyExecutionStatus.IN_PROGRESS, [
                    IKeysCeremonyTrusteeStatus.KEY_RETRIEVED,
                    IKeysCeremonyTrusteeStatus.KEY_GENERATED,
                    IKeysCeremonyTrusteeStatus.KEY_GENERATED,
                ])}
                {...args}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function uploadKey(canvasElement: HTMLElement, key: string) {
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("Missing key file chooser")
    await userEvent.upload(input, new File([key], "backup.txt", {type: "text/plain"}))
}

const INVALID = "Invalid Encrypted Private Key Backup, please try again"

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("heading", {name: "Check your Encrypted Private Key Backups"})
        ).toBeVisible()
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const ValidBackupIsVerified: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, PRIVATE_KEY)
        await expect(await canvas.findByText("Backup verified successfully.")).toBeVisible()
        expect(graphql.calls).toEqual([
            {
                name: "CheckPrivateKey",
                variables: {
                    electionEventId: EVENT_ID,
                    keysCeremonyId: STORY_IDS.keysCeremony,
                    privateKeyBase64: PRIVATE_KEY,
                },
                headers: {},
            },
        ])
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        expect(args.goNext).toHaveBeenCalledTimes(1)
    },
}

export const WrongBackupCanBeReplaced: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, "another trustee's backup")
        await expect(await canvas.findByText(INVALID)).toBeVisible()
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        await uploadKey(canvasElement, PRIVATE_KEY)
        await expect(await canvas.findByText("Backup verified successfully.")).toBeVisible()
        expect(canvas.queryByText(INVALID)).not.toBeInTheDocument()
        expect(graphql.calls.map(({variables}) => variables.privateKeyBase64)).toEqual([
            "another trustee's backup",
            PRIVATE_KEY,
        ])
    },
}

export const CheckServiceFailure: Story = {
    args: {check: "failure"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, PRIVATE_KEY)
        await expect(await canvas.findByText(INVALID)).toBeVisible()
        await waitFor(() => expect(canvas.queryByRole("progressbar")).not.toBeInTheDocument())
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(graphql.calls.map(({name}) => name)).toEqual(["CheckPrivateKey"])
    },
}

export const Back: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Back"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}
