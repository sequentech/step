// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SigningAction, SigningRequirement} from "@/lib/signing/types"
import {EditElectionEventSignatures} from "./EditElectionEventSignatures"
import {ruleOf} from "./signingSettings"
import {
    Organization,
    SIGNING_ROLES,
    SignaturesStory,
    applyOverrides,
    councilText,
    organizationOf,
    signingHandlers,
    signingRecords,
    type SigningRole,
} from "./__stories__/SignaturesFixture"

interface Scenario {
    organization: Organization
    role: SigningRole
    /** Whether the event record is locked down. */
    lockedDown: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof signingRecords>

const meta = {
    title: "Admin/Election event/Signatures/EditElectionEventSignatures",
    component: EditElectionEventSignatures,
    args: {organization: Organization.Overseas, role: "auditor", lockedDown: false},
    argTypes: {
        organization: {control: "inline-radio", options: Object.values(Organization)},
        role: {control: "select", options: Object.keys(SIGNING_ROLES)},
    },
    beforeEach: ({args}) => {
        const organization = organizationOf(args.organization)
        graphql = graphqlBoundary(signingHandlers(organization))
        data = signingRecords(organization)
        return applyOverrides(organization)
    },
    render: ({role, lockedDown}) => (
        <SignaturesStory boundary={graphql} data={data} role={role} lockedDown={lockedDown}>
            <EditElectionEventSignatures />
        </SignaturesStory>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string, options?: Record<string, unknown>) => i18n.t(`signing.${key}`, options)

const SUB_TABS = {
    rules: label("tab.protectedActions"),
    certificates: label("tab.certificates"),
    requests: label("tab.requests"),
}

async function subTabs(canvasElement: HTMLElement) {
    await within(canvasElement).findByText(label("tab.intro"))
    return within(canvasElement)
        .queryAllByRole("tab")
        .map((tab) => tab.textContent)
}

/** The operations the story read, which the sub-tabs' permissions select. */
const reads = () => graphql.calls.map(({name}) => name)

export const ConfigurationManager: Story = {
    args: {role: "configurationManager"},
    play: async ({canvasElement}) => {
        expect(await subTabs(canvasElement)).toEqual([SUB_TABS.rules])
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("button", {
                name: label("protectedActions.edit", {action: label("actions.open-voting.label")}),
            })
        ).toBeVisible()
        expect(canvas.queryByText(label("readOnly.chip"))).toBeNull()
        expect(reads()).not.toContain("GetSigningCertificates")
        expect(reads()).not.toContain("GetSigningRequests")
    },
}

export const SecurityOfficer: Story = {
    args: {role: "securityOfficer"},
    play: async ({canvasElement}) => {
        expect(await subTabs(canvasElement)).toEqual([SUB_TABS.certificates])
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("button", {name: label("certificates.import")})
        ).toBeVisible()
        expect(canvas.queryByText(label("readOnly.chip"))).toBeNull()
        expect(reads()).toEqual(["GetSigningCertificates"])
    },
}

export const Ofov: Story = {
    args: {role: "ofov"},
    play: async ({canvasElement}) => {
        expect(await subTabs(canvasElement)).toEqual([SUB_TABS.requests])
        await expect(
            await within(canvasElement).findByRole("button", {name: label("requests.exportCsv")})
        ).toBeVisible()
        expect(reads()).toEqual(["GetSigningRequests"])
    },
}

export const Auditor: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(await subTabs(canvasElement)).toEqual([
            SUB_TABS.rules,
            SUB_TABS.certificates,
            SUB_TABS.requests,
        ])
        // Protected actions: read only, with view (eye) buttons instead of edit.
        const view = label("protectedActions.view", {action: label("actions.close-voting.label")})
        await expect(await canvas.findByRole("button", {name: view})).toBeVisible()
        await expect(canvas.getAllByText(label("readOnly.chip"))[0]).toBeVisible()
        expect(canvas.queryByRole("button", {name: /^Edit /})).toBeNull()

        await userEvent.click(canvas.getByRole("tab", {name: SUB_TABS.certificates}))
        await expect(await canvas.findByText(label("certificates.issuersIntro"))).toBeVisible()
        await expect(canvas.getAllByText(label("readOnly.chip"))[0]).toBeVisible()
        expect(canvas.queryByRole("button", {name: label("certificates.import")})).toBeNull()

        await userEvent.click(canvas.getByRole("tab", {name: SUB_TABS.requests}))
        await expect(
            await canvas.findByRole("button", {name: label("requests.exportCsv")})
        ).toBeVisible()
    },
}

export const WithoutReadPermission: Story = {
    args: {role: "tabOnly"},
    play: async ({canvasElement}) => {
        expect(await subTabs(canvasElement)).toEqual([])
        await waitFor(() => expect(reads()).toEqual([]))
    },
}

export const SecondOrganization: Story = {
    args: {organization: Organization.StudentCouncil},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const {rules, capacities} = organizationOf(Organization.StudentCouncil)
        // The tenant's own action names and roles; its Off rows stay listed.
        await expect(
            await canvas.findByText(councilText("signing.actions.generate-election-returns.label"))
        ).toBeVisible()
        const [role] = capacities[SigningAction.GenerateElectionReturns]?.roles ?? []
        await expect(canvas.getAllByText(role.name)[0]).toBeVisible()
        const off = Object.values(SigningAction).filter(
            (action) => ruleOf(action, rules).requirement === SigningRequirement.NotRequired
        )
        expect(canvas.getAllByText(label("protectedActions.off"))).toHaveLength(off.length)
    },
}

export const LockedDownEvent: Story = {
    args: {role: "configurationManager", lockedDown: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // The event record's lockdown makes the rules read-only.
        await expect(await canvas.findByText(label("protectedActions.lockedDown"))).toBeVisible()
        expect(canvas.queryByRole("button", {name: /^Edit /})).toBeNull()
    },
}
