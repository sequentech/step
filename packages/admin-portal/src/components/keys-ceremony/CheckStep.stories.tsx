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
import {CUSTODY_PROOF, ceremony, ceremonyEvent} from "./__stories__/KeysCeremonyFixture"
import {
    CUSTODY_PROOF_ORGANIZATIONS,
    CustodyProofOrganization,
    CustodyProofRule,
    SIGNING_REQUEST,
    TRUSTEE_ROLES,
    applyCustodyProofOverrides,
    custodyProofApi,
    sha256,
    signCustodyProof,
    signingNote,
    trusteeAuth,
} from "./__stories__/CustodyProofSigningFixture"

interface Scenario {
    /** What checking a key does. */
    check: "service" | "failure"
    /** Whether the event's rule makes the trustee sign the check. */
    rule: CustodyProofRule
    organization: CustodyProofOrganization
    /** The trustee signed the request before, e.g. before a reload. */
    signedEarlier: boolean
    /** Whether the ceremony still takes the custody proof once it is signed. */
    stillTaken: boolean
    goNext: () => void
    goBack: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let api: Awaited<ReturnType<typeof custodyProofApi>>

const meta = {
    title: "Admin/Keys ceremony/CheckStep",
    component: CheckStep,
    args: {
        check: "service",
        rule: CustodyProofRule.NotRequired,
        organization: CustodyProofOrganization.Overseas,
        signedEarlier: false,
        stillTaken: true,
        goNext: fn(),
        goBack: fn(),
    },
    argTypes: {
        check: {control: "inline-radio", options: ["service", "failure"]},
        rule: {control: "inline-radio", options: Object.values(CustodyProofRule)},
        organization: {control: "inline-radio", options: Object.values(CustodyProofOrganization)},
    },
    beforeEach: async ({args}) => {
        const organization = CUSTODY_PROOF_ORGANIZATIONS[args.organization]
        api = await custodyProofApi(
            SigningAction.ConfirmCustodyProof,
            {keys_ceremony_id: STORY_IDS.keysCeremony},
            organization,
            CUSTODY_PROOF,
            args.signedEarlier
        )
        graphql = graphqlBoundary(
            {
                ConfirmKeyCustody: ({variables}) => {
                    if (args.check === "failure") throw new Error("Synthetic check unavailable")
                    const isValid =
                        variables.custodyProofJson === CUSTODY_PROOF &&
                        (args.stillTaken || !variables.signingRequestId)
                    // The rule makes a right key wait for the trustee's signature.
                    const waits =
                        isValid &&
                        args.rule === CustodyProofRule.Required &&
                        !variables.signingRequestId
                    return {
                        data: {
                            confirm_key_custody: {
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
        return applyCustodyProofOverrides(organization)
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
            auth={trusteeAuth(CUSTODY_PROOF_ORGANIZATIONS[organization])}
            signingApi={api as ISigningApi}
        >
            <CheckStep
                electionEvent={ceremonyEvent()}
                currentCeremony={ceremony(IKeysCeremonyExecutionStatus.IN_PROGRESS, [
                    IKeysCeremonyTrusteeStatus.CUSTODY_REQUEST_RETRIEVED,
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
    await userEvent.upload(input, new File([key], "custody-proof.json", {type: "text/plain"}))
}

const INVALID = "Invalid custody proof, please try again"
const VERIFIED = "Trustee custody confirmed successfully."

const checkCall = async (key: string, signingRequestId?: string) => ({
    name: "ConfirmKeyCustody",
    variables: {
        electionEventId: EVENT_ID,
        keysCeremonyId: STORY_IDS.keysCeremony,
        custodyProofJson: key,
        ...(signingRequestId ? {signingRequestId} : {custodyProofSha256: await sha256(key)}),
    },
    headers: {"x-hasura-role": "trustee-ceremony"},
})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Confirm Trustee Custody"})).toBeVisible()
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const ValidCustodyProofIsVerified: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, CUSTODY_PROOF)
        await expect(await canvas.findByText(VERIFIED)).toBeVisible()
        // With the rule off the check is recorded at once; only its hash goes beside the key.
        expect(graphql.calls).toEqual([await checkCall(CUSTODY_PROOF)])
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
        await uploadKey(canvasElement, CUSTODY_PROOF)
        await expect(await canvas.findByText(VERIFIED)).toBeVisible()
        expect(canvas.queryByText(INVALID)).not.toBeInTheDocument()
        expect(graphql.calls.map(({variables}) => variables.custodyProofJson)).toEqual([
            "another trustee's backup",
            CUSTODY_PROOF,
        ])
    },
}

export const CheckServiceFailure: Story = {
    args: {check: "failure"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, CUSTODY_PROOF)
        await expect(await canvas.findByText(INVALID)).toBeVisible()
        await waitFor(() => expect(canvas.queryByRole("progressbar")).not.toBeInTheDocument())
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(graphql.calls.map(({name}) => name)).toEqual(["ConfirmKeyCustody"])
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
    args: {rule: CustodyProofRule.Required},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const organization = CUSTODY_PROOF_ORGANIZATIONS[args.organization]
        await uploadKey(canvasElement, CUSTODY_PROOF)
        await expect(await canvas.findByText(signingNote(organization))).toBeVisible()
        // Behind the open signing panel, the step can't go on yet.
        expect(canvas.getByRole("button", {name: "Next", hidden: true})).toBeDisabled()
        await expect(api.getRequest).toHaveBeenCalledWith(SIGNING_REQUEST.id)

        await signCustodyProof(organization, CUSTODY_PROOF, {
            keys_ceremony_id: STORY_IDS.keysCeremony,
        })

        await expect(await canvas.findByText(VERIFIED, {}, {timeout: 10000})).toBeVisible()
        expect(graphql.calls).toEqual([
            await checkCall(CUSTODY_PROOF),
            await checkCall(CUSTODY_PROOF, SIGNING_REQUEST.id),
        ])
        // The custody proof goes to the check only, never to the signing service.
        const sent = JSON.stringify(Object.values(api).map((call) => call.mock.calls))
        expect(sent).not.toContain(CUSTODY_PROOF)
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        expect(args.goNext).toHaveBeenCalledTimes(1)
    },
}

/** The same flow for an organization that renames the labels in its translations. */
export const SignedCheckForAnotherOrganization: Story = {
    ...SignedCheckIsRecorded,
    args: {rule: CustodyProofRule.Required, organization: CustodyProofOrganization.StudentCouncil},
}

/** A request the trustee signed before (e.g. before a reload) is recorded from its panel. */
export const SignedEarlierIsRecordedFromThePanel: Story = {
    args: {rule: CustodyProofRule.Required, signedEarlier: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, CUSTODY_PROOF)
        const record = await within(document.body).findByRole("button", {
            name: "Record my custody proof",
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

/** The ceremony refuses the signed custody proof (e.g. it changed meanwhile): the step says why. */
export const SignedButNoLongerTaken: Story = {
    args: {rule: CustodyProofRule.Required, signedEarlier: true, stillTaken: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await uploadKey(canvasElement, CUSTODY_PROOF)
        await userEvent.click(
            await within(document.body).findByRole("button", {name: "Record my custody proof"})
        )
        await expect(
            await canvas.findByText(
                "Your signed custody proof could not be recorded: The ceremony no longer takes this custody proof. Drop your custody proof file again. (custody-proof.json)"
            )
        ).toBeVisible()
        expect(canvas.queryByText(VERIFIED)).not.toBeInTheDocument()
        // The custody proof is no longer kept: recording again asks for the file.
        await userEvent.click(
            within(document.body).getByRole("button", {name: "Record my custody proof"})
        )
        await expect(
            await canvas.findByText(
                "Your signed custody proof could not be recorded: Drop your custody proof file again to record your signed custody proof. (custody-proof.json)"
            )
        ).toBeVisible()
        expect(graphql.calls).toHaveLength(2)
    },
}
