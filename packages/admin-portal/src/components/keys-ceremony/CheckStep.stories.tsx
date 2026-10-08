// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {ISigningApi} from "@/lib/signing/api"
import {SigningAction} from "@/lib/signing/types"
import {IKeysCeremonyExecutionStatus, IKeysCeremonyTrusteeStatus} from "@/services/KeyCeremony"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {CheckStep} from "./CheckStep"
import {PRIVATE_KEY, ceremony, ceremonyEvent} from "./__stories__/KeysCeremonyFixture"
import {
    KEY_SHARE_ORGANIZATIONS,
    KeyShareOrganization,
    KeyShareRule,
    SIGNING_REQUEST,
    TRUSTEE_ROLES,
    applyKeyShareOverrides,
    keyShareApi,
    sha256,
    signKeyShare,
    signingNote,
    trusteeAuth,
} from "./__stories__/KeyShareSigningFixture"

interface Scenario {
    /** What checking a key does. */
    check: "service" | "failure"
    /** Whether the event's rule makes the trustee sign the check. */
    rule: KeyShareRule
    organization: KeyShareOrganization
    /** The trustee signed the request before, e.g. before a reload. */
    signedEarlier: boolean
    /** Whether the ceremony still takes the key share once it is signed. */
    stillTaken: boolean
    goNext: () => void
    goBack: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let api: Awaited<ReturnType<typeof keyShareApi>>

const meta = {
    title: "Admin/Keys ceremony/CheckStep",
    component: CheckStep,
    args: {
        check: "service",
        rule: KeyShareRule.NotRequired,
        organization: KeyShareOrganization.Overseas,
        signedEarlier: false,
        stillTaken: true,
        goNext: fn(),
        goBack: fn(),
    },
    argTypes: {
        check: {control: "inline-radio", options: ["service", "failure"]},
        rule: {control: "inline-radio", options: Object.values(KeyShareRule)},
        organization: {control: "inline-radio", options: Object.values(KeyShareOrganization)},
    },
    beforeEach: async ({args}) => {
        const organization = KEY_SHARE_ORGANIZATIONS[args.organization]
        api = await keyShareApi(
            SigningAction.ConfirmKeyShare,
            {keys_ceremony_id: STORY_IDS.keysCeremony},
            organization,
            PRIVATE_KEY,
            args.signedEarlier
        )
        graphql = graphqlBoundary(
            {
                CheckPrivateKey: ({variables}) => {
                    if (args.check === "failure") throw new Error("Synthetic check unavailable")
                    const isValid =
                        variables.privateKeyBase64 === PRIVATE_KEY &&
                        (args.stillTaken || !variables.signingRequestId)
                    // The rule makes a right key wait for the trustee's signature.
                    const waits =
                        isValid &&
                        args.rule === KeyShareRule.Required &&
                        !variables.signingRequestId
                    return {
                        data: {
                            check_private_key: {
                                is_valid: isValid,
                                signing_request: waits ? SIGNING_REQUEST : null,
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
        return applyKeyShareOverrides(organization)
    },
    render: ({
        check: _check,
        rule: _rule,
        organization,
        signedEarlier: _signed,
        stillTaken: _taken,
        ...args
    }) => (
        <AdminStoryProvider
            boundary={graphql}
            role={EStoryPermissions.TRUSTEE}
            roles={TRUSTEE_ROLES}
            auth={trusteeAuth(KEY_SHARE_ORGANIZATIONS[organization])}
            signingApi={api as ISigningApi}
        >
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
const VERIFIED = "Backup verified successfully."

const checkCall = async (key: string, signingRequestId?: string) => ({
    name: "CheckPrivateKey",
    variables: {
        electionEventId: EVENT_ID,
        keysCeremonyId: STORY_IDS.keysCeremony,
        privateKeyBase64: key,
        ...(signingRequestId ? {signingRequestId} : {keyShareSha256: await sha256(key)}),
    },
    headers: {},
})

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
        await expect(await canvas.findByText(VERIFIED)).toBeVisible()
        // With the rule off the check is recorded at once; only its hash goes beside the key.
        expect(graphql.calls).toEqual([await checkCall(PRIVATE_KEY)])
        expect(api.getRequest).not.toHaveBeenCalled()
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
        await expect(await canvas.findByText(VERIFIED)).toBeVisible()
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

/** The rule needs the trustee's signature: the check is recorded once they have signed. */
export const SignedCheckIsRecorded: Story = {
    args: {rule: KeyShareRule.Required},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const organization = KEY_SHARE_ORGANIZATIONS[args.organization]
        await uploadKey(canvasElement, PRIVATE_KEY)
        await expect(await canvas.findByText(signingNote(organization))).toBeVisible()
        // Behind the open signing panel, the step can't go on yet.
        expect(canvas.getByRole("button", {name: "Next", hidden: true})).toBeDisabled()
        await expect(api.getRequest).toHaveBeenCalledWith(SIGNING_REQUEST.id)

        await signKeyShare(organization, PRIVATE_KEY, {keys_ceremony_id: STORY_IDS.keysCeremony})

        await expect(await canvas.findByText(VERIFIED, {}, {timeout: 10000})).toBeVisible()
        expect(graphql.calls).toEqual([
            await checkCall(PRIVATE_KEY),
            await checkCall(PRIVATE_KEY, SIGNING_REQUEST.id),
        ])
        // The key share goes to the check only, never to the signing service.
        const sent = JSON.stringify(Object.values(api).map((call) => call.mock.calls))
        expect(sent).not.toContain(PRIVATE_KEY)
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        expect(args.goNext).toHaveBeenCalledTimes(1)
    },
}

/** The same flow for an organization that renames the labels in its translations. */
export const SignedCheckForAnotherOrganization: Story = {
    ...SignedCheckIsRecorded,
    args: {rule: KeyShareRule.Required, organization: KeyShareOrganization.StudentCouncil},
}

/** A request the trustee signed before (e.g. before a reload) is recorded from its panel. */
export const SignedEarlierIsRecordedFromThePanel: Story = {
    args: {rule: KeyShareRule.Required, signedEarlier: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, PRIVATE_KEY)
        const record = await within(document.body).findByRole("button", {
            name: "Record my key share",
        })
        await userEvent.click(record)
        await expect(await canvas.findByText(VERIFIED)).toBeVisible()
        expect(graphql.calls.map(({variables}) => variables.signingRequestId)).toEqual([
            undefined,
            SIGNING_REQUEST.id,
        ])
        await expect(api.approve).not.toHaveBeenCalled()
    },
}

/** The ceremony refuses the signed key share (e.g. it changed meanwhile): the step says why. */
export const SignedButNoLongerTaken: Story = {
    args: {rule: KeyShareRule.Required, signedEarlier: true, stillTaken: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, PRIVATE_KEY)
        await userEvent.click(
            await within(document.body).findByRole("button", {name: "Record my key share"})
        )
        await expect(
            await canvas.findByText(
                "Your signed key share could not be recorded: The ceremony no longer takes this key share. Drop your key share file again."
            )
        ).toBeVisible()
        expect(canvas.queryByText(VERIFIED)).not.toBeInTheDocument()
        // The key share is no longer kept: recording again asks for the file.
        await userEvent.click(
            within(document.body).getByRole("button", {name: "Record my key share"})
        )
        await expect(
            await canvas.findByText(
                "Your signed key share could not be recorded: Drop your key share file again to record your signed key share."
            )
        ).toBeVisible()
        expect(graphql.calls).toHaveLength(2)
    },
}
