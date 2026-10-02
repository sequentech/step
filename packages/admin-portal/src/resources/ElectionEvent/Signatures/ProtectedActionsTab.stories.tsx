// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SigningAction, SigningRequirement} from "@/lib/signing/types"
import {ProtectedActionsTab} from "./ProtectedActionsTab"
import {expiryKey, ruleOf, signaturesAccess} from "./signingSettings"
import {
    Organization,
    SIGNING_ROLES,
    SignaturesStory,
    applyOverrides,
    councilText,
    organizationOf,
    signingHandlers,
    signingRecords,
    type ISigningOrganization,
    type SigningRole,
} from "./__stories__/SignaturesFixture"

interface Scenario {
    organization: Organization
    role: SigningRole
    /** Whether the event is locked down. */
    lockedDown: boolean
    /** The event has published no configuration version yet. */
    beforeFirstPublication?: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof signingRecords>

const meta = {
    title: "Admin/Election event/Signatures/ProtectedActionsTab",
    component: ProtectedActionsTab,
    args: {organization: Organization.Overseas, role: "configurationManager", lockedDown: false},
    argTypes: {
        organization: {control: "inline-radio", options: Object.values(Organization)},
        role: {control: "select", options: Object.keys(SIGNING_ROLES)},
    },
    beforeEach: ({args}) => {
        const organization = args.beforeFirstPublication
            ? unpublished(organizationOf(args.organization))
            : organizationOf(args.organization)
        graphql = graphqlBoundary(signingHandlers(organization))
        data = signingRecords(organization)
        return applyOverrides(organization)
    },
    render: ({role, lockedDown}) => {
        const roles = new Set<string>(SIGNING_ROLES[role])
        return (
            <SignaturesStory boundary={graphql} data={data} role={role}>
                <ProtectedActionsTab
                    electionEventId={EVENT_ID}
                    access={signaturesAccess((permission) => roles.has(permission))}
                    lockedDown={lockedDown}
                />
            </SignaturesStory>
        )
    },
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string, options?: Record<string, unknown>) => i18n.t(`signing.${key}`, options)

/** The organization's event before its first configuration version is published. */
const unpublished = (organization: ISigningOrganization): ISigningOrganization => ({
    ...organization,
    configVersion: 0,
    capacities: Object.fromEntries(
        Object.entries(organization.capacities).map(([action, capacity]) => [
            action,
            capacity && {...capacity, config_version: 0},
        ])
    ),
})

/** The footer's version sentence followed by who saved the rules last. */
const footer = (version: number, editor: string) =>
    new RegExp(
        `^${escapeRegExp(label("protectedActions.footerVersion", {version}))} .* ${escapeRegExp(editor)}\\.$`
    )
const escapeRegExp = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")

/** The row of an action, found by its (possibly overridden) label. */
async function actionRow(canvasElement: HTMLElement, action: string) {
    const cell = await within(canvasElement).findByText(action)
    return within(cell.closest("tr") as HTMLElement)
}

/** The cells of an action's row: action, applies to, who can sign, signatures, expiry, waiting. */
const cells = (row: ReturnType<typeof within>) =>
    row.getAllByRole("cell").map((cell: HTMLElement) => cell.textContent)

/** Each action's row shows its rule from the organization's data, not from the code. */
async function expectRules(canvasElement: HTMLElement, organization: Organization) {
    const {rules, capacities} = organizationOf(organization)
    for (const action of Object.values(SigningAction)) {
        const rule = ruleOf(action, rules)
        const row = await actionRow(canvasElement, label(`actions.${action}.label`))
        const [, appliesTo, whoCanSign, signatures, expires] = cells(row)
        expect(appliesTo).toBe(label(`actions.${action}.appliesTo`))
        if (rule.requirement === SigningRequirement.NotRequired) {
            expect([whoCanSign, signatures, expires]).toEqual([
                "–",
                label("protectedActions.off"),
                "–",
            ])
            continue
        }
        expect(whoCanSign).toBe((capacities[action]?.roles ?? []).map(({name}) => name).join(""))
        if (
            action === SigningAction.ConfirmKeyShare ||
            action === SigningAction.ContributeKeyShare
        ) {
            expect([signatures, expires]).toEqual([label("protectedActions.eachTrustee"), "–"])
        } else {
            expect([signatures, expires]).toEqual([
                String(rule.signatures),
                label(`expiry.${expiryKey(rule.expires_minutes)}`),
            ])
        }
    }
}

export const ConfigurationManager: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expectRules(canvasElement, Organization.Overseas)
        for (const group of [
            "voting",
            "results-and-reports",
            "enrollment",
            "configuration-and-keys",
        ]) {
            await expect(canvas.getByText(label(`groups.${group}`))).toBeVisible()
        }
        // Waiting requests of the action, from the capacity route.
        const closing = await actionRow(canvasElement, label("actions.close-voting.label"))
        await expect(
            closing.getByLabelText(label("protectedActions.waitingCount", {count: 1}))
        ).toBeVisible()
        expect(canvas.queryByText(label("readOnly.chip"))).toBeNull()
        await expect(
            canvas.getByRole("button", {
                name: label("protectedActions.edit", {action: label("actions.open-voting.label")}),
            })
        ).toBeVisible()
        // The footer: the configuration version from the capacity route, and who saved last.
        const {configVersion, editor} = organizationOf(Organization.Overseas)
        await expect(await canvas.findByText(footer(configVersion, editor))).toBeVisible()
        expect(graphql.calls.find(({name}) => name === "GetSigningRules")?.headers).toEqual({
            "x-hasura-role": "signing-rules-read",
        })
    },
}

export const EditARule: Story = {
    play: async ({canvasElement}) => {
        const action = label("actions.transmit-results.label")
        await userEvent.click(
            await within(canvasElement).findByRole("button", {
                name: label("protectedActions.edit", {action}),
            })
        )
        const drawer = within(await within(document.body).findByRole("dialog", {name: action}))
        await expect(drawer.getByText(label("pendingRequests", {count: 2}))).toBeVisible()
        await userEvent.click(drawer.getByRole("button", {name: label("rule.cancel")}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const Auditor: Story = {
    args: {role: "auditor"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expectRules(canvasElement, Organization.Overseas)
        await expect(canvas.getByText(label("readOnly.chip"))).toBeVisible()
        const action = label("actions.close-voting.label")
        expect(
            canvas.queryByRole("button", {name: label("protectedActions.edit", {action})})
        ).toBeNull()
        await userEvent.click(
            canvas.getByRole("button", {name: label("protectedActions.view", {action})})
        )
        const drawer = within(await within(document.body).findByRole("dialog", {name: action}))
        await expect(drawer.getByText(label("readOnly.rules"))).toBeVisible()
        expect(drawer.queryByRole("button", {name: label("rule.save")})).toBeNull()
    },
}

export const SecondOrganization: Story = {
    args: {organization: Organization.StudentCouncil},
    play: async ({canvasElement}) => {
        const council = organizationOf(Organization.StudentCouncil)
        // Every label is the tenant's (expectRules reads them through its overrides).
        await expectRules(canvasElement, Organization.StudentCouncil)
        // The renamed action, and its Posts renamed by the one term override.
        const results = await actionRow(
            canvasElement,
            councilText("signing.actions.generate-election-returns.label")
        )
        const [, appliesTo, whoCanSign] = cells(results)
        expect(appliesTo).toContain(councilText("signing.terms.post"))
        expect(whoCanSign).toBe(
            council.capacities[SigningAction.GenerateElectionReturns]?.roles
                .map(({name}) => name)
                .join("")
        )
        const approval = await actionRow(
            canvasElement,
            councilText("signing.actions.approve-voter.label")
        )
        expect(cells(approval)[1]).toBe(councilText("signing.actions.approve-voter.appliesTo"))
        await expect(
            await within(canvasElement).findByText(footer(council.configVersion, council.editor))
        ).toBeVisible()
    },
}

export const LockedDown: Story = {
    args: {lockedDown: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // Allowed to edit, but the rules belong to the configuration version now.
        await expect(await canvas.findByText(label("protectedActions.lockedDown"))).toBeVisible()
        await expect(canvas.getByText(label("readOnly.chip"))).toBeVisible()
        const action = label("actions.open-voting.label")
        expect(
            canvas.queryByRole("button", {name: label("protectedActions.edit", {action})})
        ).toBeNull()
        await userEvent.click(
            canvas.getByRole("button", {name: label("protectedActions.view", {action})})
        )
        const drawer = within(await within(document.body).findByRole("dialog", {name: action}))
        expect(drawer.queryByRole("button", {name: label("rule.save")})).toBeNull()
    },
}

export const BeforeTheFirstPublication: Story = {
    args: {beforeFirstPublication: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const {editor} = organizationOf(Organization.Overseas)
        // No "configuration version 0": the rules join the first version once it is published.
        await expect(
            await canvas.findByText(
                new RegExp(
                    `^${escapeRegExp(label("protectedActions.footerFirstVersion"))} .* ${escapeRegExp(editor)}\\.$`
                )
            )
        ).toBeVisible()
        expect(canvas.queryByText(/version 0\b/)).toBeNull()
    },
}
