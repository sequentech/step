// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {RequesterSigning, SigningAction, SigningRequirement} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {RuleDrawer} from "./RuleDrawer"
import {ruleOf, signaturesAccess} from "./signingSettings"
import {
    Organization,
    SIGNING_ROLES,
    SignaturesStory,
    applyOverrides,
    capacityOf,
    councilText,
    organizationOf,
    roleId,
    signingHandlers,
    signingRecords,
    type SigningRole,
} from "./__stories__/SignaturesFixture"

interface Scenario {
    organization: Organization
    role: SigningRole
    action: SigningAction
    /** Whether saving fails. */
    failWrites: boolean
    /** The typed error the save is refused with (`extensions`), if any. */
    refuse?: Record<string, unknown>
    /** The Posts the save reports as short of signers. */
    shortPosts?: Array<{election_id: string; name: string; count: number}>
    onClose: () => void
    onSaved: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof signingRecords>

const meta = {
    title: "Admin/Election event/Signatures/RuleDrawer",
    component: RuleDrawer,
    args: {
        organization: Organization.Overseas,
        role: "configurationManager",
        action: SigningAction.GenerateElectionReturns,
        failWrites: false,
        onClose: fn(),
        onSaved: fn(),
    },
    argTypes: {
        organization: {control: "inline-radio", options: Object.values(Organization)},
        role: {control: "select", options: Object.keys(SIGNING_ROLES)},
        action: {control: "select", options: Object.values(SigningAction)},
    },
    beforeEach: ({args}) => {
        const organization = organizationOf(args.organization)
        graphql = graphqlBoundary(
            signingHandlers(organization, {
                failWrites: args.failWrites,
                refuse: args.refuse ? {SigningPutRule: args.refuse} : {},
                shortPosts: args.shortPosts,
            })
        )
        data = signingRecords(organization)
        return applyOverrides(organization)
    },
    render: ({organization: name, role, action, onClose, onSaved}) => {
        const organization = organizationOf(name)
        const roles = new Set<string>(SIGNING_ROLES[role])
        return (
            <SignaturesStory boundary={graphql} data={data} role={role}>
                <RuleDrawer
                    electionEventId={EVENT_ID}
                    rule={ruleOf(action, organization.rules)}
                    capacity={capacityOf(organization, action)}
                    access={signaturesAccess((permission) => roles.has(permission))}
                    onClose={onClose}
                    onSaved={onSaved}
                />
            </SignaturesStory>
        )
    },
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string, options?: Record<string, unknown>) => i18n.t(`signing.${key}`, options)

async function drawer() {
    const heading = await within(document.body).findByRole("heading", {level: 2})
    return within(heading.closest("section") as HTMLElement)
}

const saveButton = (panel: ReturnType<typeof within>) =>
    panel.getByRole("button", {name: label("rule.save")})

const signaturesField = (panel: ReturnType<typeof within>) =>
    panel.getByRole("spinbutton", {name: label("rule.signaturesNeeded")})

/** The variables of the one SigningPutRule the story sent. */
const savedRule = () => graphql.calls.filter(({name}) => name === "SigningPutRule")

/** Waits for a notification, which fades in. */
async function notified(text: string) {
    const messages = await within(document.body).findAllByText(text)
    await waitFor(() => messages.forEach((message) => expect(message).toBeVisible()))
}

export const Editable: Story = {
    play: async ({args}) => {
        const panel = await drawer()
        await expect(
            panel.getByRole("heading", {name: label("actions.generate-election-returns.label")})
        ).toBeVisible()
        await expect(panel.getByLabelText(label("rule.needsSignatures"))).toBeChecked()
        // Who can sign is read-only without role-write.
        await expect(panel.getByText("SBEI")).toBeVisible()
        await expect(
            panel.getByText(
                label("readOnly.whoCanSign", {
                    action: label("actions.generate-election-returns.permissionName"),
                })
            )
        ).toBeVisible()
        expect(panel.queryByRole("combobox", {name: label("rule.whoCanSign")})).toBeNull()
        await expect(panel.getByText(label("rule.signaturesNeededHelp", {n: 3}))).toBeVisible()
        await expect(saveButton(panel)).toBeDisabled()

        await userEvent.clear(signaturesField(panel))
        await userEvent.type(signaturesField(panel), "2")
        await userEvent.click(panel.getByRole("combobox", {name: label("rule.expiresAfter")}))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: label("expiry.none")})
        )
        await expect(saveButton(panel)).toBeEnabled()
        await userEvent.click(saveButton(panel))
        await waitFor(() => expect(args.onSaved).toHaveBeenCalledOnce())
        expect(savedRule()).toEqual([
            {
                name: "SigningPutRule",
                // Staff without admin-user save rules with their own permission.
                headers: {"x-hasura-role": IPermissions.SIGNING_RULES_WRITE},
                variables: {
                    election_event_id: EVENT_ID,
                    action: SigningAction.GenerateElectionReturns,
                    requirement: SigningRequirement.Required,
                    signatures: 2,
                    requester_signing: RequesterSigning.Allowed,
                    expires_minutes: null,
                    expected_revision: 3,
                    roles: null,
                },
            },
        ])
    },
}

export const RolesEditableWithRoleWrite: Story = {
    args: {role: "rulesAndRoles"},
    play: async ({args}) => {
        const panel = await drawer()
        const roles = panel.getByRole("combobox", {name: label("rule.whoCanSign")})
        await userEvent.click(roles)
        const options = await within(document.body).findAllByRole("option")
        expect(options.map((option) => option.textContent)).toEqual([
            "Auditor",
            "Configuration Manager",
            "OFOV",
            "SBEI",
            "Security Officer",
            "Trustee",
        ])
        await userEvent.click(await within(document.body).findByRole("option", {name: "OFOV"}))
        await userEvent.click(panel.getByRole("button", {name: label("rule.save")}))
        await waitFor(() => expect(args.onSaved).toHaveBeenCalledOnce())
        // Roles are sent by group id.
        expect(savedRule()[0].variables.roles).toEqual({add: [roleId("OFOV")], remove: []})
    },
}

export const CapacityWarning: Story = {
    args: {action: SigningAction.TransmitResults},
    play: async () => {
        const panel = await drawer()
        await userEvent.clear(signaturesField(panel))
        await userEvent.type(signaturesField(panel), "3")
        // The Posts come from the capacity, grouped by how many can sign there.
        const {posts} = capacityOf(
            organizationOf(Organization.Overseas),
            SigningAction.TransmitResults
        )
        const short = posts.filter(({count}) => count === 2).map(({name}) => name)
        const warning = await panel.findByText(
            label("validation.shortPosts", {
                count: short.length,
                posts: new Intl.ListFormat("en", {type: "conjunction"}).format(short),
                n: 2,
                required: 3,
            })
        )
        await expect(warning).toBeVisible()
        await expect(panel.getByText(label("rule.signaturesNeededShortHelp"))).toBeVisible()
        // Short Posts are a warning; the rule can still be saved.
        await expect(saveButton(panel)).toBeEnabled()
    },
}

export const RequesterWarning: Story = {
    args: {action: SigningAction.TransmitResults},
    play: async () => {
        const action = SigningAction.TransmitResults
        const organization = organizationOf(Organization.Overseas)
        const required = ruleOf(action, organization.rules).signatures
        // Without the requester, the Posts with just enough signers fall one short.
        const {posts} = capacityOf(organization, action)
        const tight = posts.filter(({count}) => count === required).map(({name}) => name)
        const warning = label("validation.requesterShort", {
            count: tight.length,
            posts: new Intl.ListFormat("en", {type: "conjunction"}).format(tight),
            n: required - 1,
            required,
        })
        const panel = await drawer()
        await expect(panel.getByText(warning)).toBeVisible()
        // Letting the requester sign removes the warning.
        await userEvent.click(panel.getByLabelText(label("rule.requesterSigning")))
        await waitFor(() => expect(panel.queryByText(warning)).toBeNull())
    },
}

export const MoreThanAnyPostRefused: Story = {
    args: {action: SigningAction.TransmitResults},
    play: async () => {
        const panel = await drawer()
        await userEvent.clear(signaturesField(panel))
        await userEvent.type(signaturesField(panel), "4")
        await expect(
            await panel.findByText(label("validation.tooMany", {n: 4, max: 3}))
        ).toBeVisible()
        await expect(signaturesField(panel)).toHaveAttribute("aria-invalid", "true")
        await expect(saveButton(panel)).toBeDisabled()
        await userEvent.clear(signaturesField(panel))
        await expect(await panel.findByText(label("validation.atLeastOne"))).toBeVisible()
        await expect(saveButton(panel)).toBeDisabled()
    },
}

export const PendingRequestsNotice: Story = {
    args: {action: SigningAction.TransmitResults},
    play: async () => {
        const panel = await drawer()
        // Two waiting requests: the plural form.
        await expect(panel.getByText(label("pendingRequests", {count: 2}))).toBeVisible()
        expect(label("pendingRequests", {count: 2})).toMatch(/^2 requests/)
    },
}

export const TrusteeActionOnOff: Story = {
    args: {organization: Organization.StudentCouncil, action: SigningAction.ConfirmKeyShare},
    play: async ({args}) => {
        const panel = await drawer()
        const toggle = panel.getByLabelText(label("rule.trusteesSign"))
        await expect(toggle).not.toBeChecked()
        await expect(panel.getByText(label("rule.trusteesHelp"))).toBeVisible()
        await userEvent.click(toggle)
        // Trustee actions have no number, roles or expiry to set.
        expect(panel.queryByRole("spinbutton")).toBeNull()
        expect(panel.queryByText(label("rule.requesterSigning"))).toBeNull()
        await userEvent.click(saveButton(panel))
        await waitFor(() => expect(args.onSaved).toHaveBeenCalledOnce())
        expect(savedRule()[0].variables).toMatchObject({
            action: SigningAction.ConfirmKeyShare,
            requirement: SigningRequirement.Required,
            signatures: 1,
            requester_signing: RequesterSigning.Allowed,
            expected_revision: 0,
        })
    },
}

export const TrusteeActionOn: Story = {
    args: {action: SigningAction.ContributeKeyShare},
    play: async () => {
        const panel = await drawer()
        await expect(panel.getByLabelText(label("rule.trusteesSign"))).toBeChecked()
        await expect(saveButton(panel)).toBeDisabled()
    },
}

export const ReadOnly: Story = {
    args: {role: "auditor"},
    play: async ({args}) => {
        const panel = await drawer()
        await expect(panel.getByText(label("readOnly.rules"))).toBeVisible()
        await expect(panel.getByLabelText(label("rule.needsSignatures"))).toBeDisabled()
        await expect(signaturesField(panel)).toHaveAttribute("readonly")
        expect(panel.queryByRole("button", {name: label("rule.save")})).toBeNull()
        // No pending notice: nothing is saved from here.
        expect(panel.queryByText(label("pendingRequests", {count: 1}))).toBeNull()
        await userEvent.click(panel.getAllByRole("button", {name: i18n.t("common.label.close")})[1])
        expect(args.onClose).toHaveBeenCalledOnce()
        expect(graphql.calls).toEqual([])
    },
}

export const SaveFailure: Story = {
    args: {failWrites: true},
    play: async ({args}) => {
        const panel = await drawer()
        await userEvent.click(panel.getByRole("checkbox", {name: label("rule.requesterSigning")}))
        await userEvent.click(saveButton(panel))
        await notified(label("rule.saveError"))
        expect(args.onSaved).not.toHaveBeenCalled()
    },
}

export const SecondOrganization: Story = {
    args: {organization: Organization.StudentCouncil},
    play: async () => {
        const action = SigningAction.GenerateElectionReturns
        const {posts, roles} = capacityOf(organizationOf(Organization.StudentCouncil), action)
        const panel = await drawer()
        // The tenant renames the action and its Posts; the roles are its own.
        await expect(
            panel.getByRole("heading", {name: councilText(`signing.actions.${action}.label`)})
        ).toBeVisible()
        for (const {name} of roles) await expect(panel.getByText(name)).toBeVisible()
        const fewest = Math.min(...posts.map(({count}) => count))
        const short = posts.filter(({count}) => count === fewest).map(({name}) => name)
        await userEvent.clear(signaturesField(panel))
        await userEvent.type(signaturesField(panel), String(fewest + 1))
        await expect(
            await panel.findByText(
                label("validation.shortPosts", {
                    count: short.length,
                    posts: new Intl.ListFormat("en", {type: "conjunction"}).format(short),
                    n: fewest,
                    required: fewest + 1,
                })
            )
        ).toBeVisible()
        await userEvent.clear(signaturesField(panel))
        await userEvent.type(signaturesField(panel), String(fewest))
        // The helper names the tenant's term for a Post, from the one override.
        const helper = label("rule.signaturesNeededHelp", {n: fewest})
        expect(helper).toContain(councilText("signing.terms.post"))
        await expect(await panel.findByText(helper)).toBeVisible()
    },
}

export const RolesNeedRoleRead: Story = {
    args: {role: "rulesAndRoleWriteOnly"},
    play: async () => {
        const panel = await drawer()
        // Without role-read the roles can't be listed, so they stay read-only.
        expect(panel.queryByRole("combobox", {name: label("rule.whoCanSign")})).toBeNull()
        await expect(
            panel.getByText(
                label("readOnly.whoCanSign", {
                    action: label("actions.generate-election-returns.permissionName"),
                })
            )
        ).toBeVisible()
        await expect(saveButton(panel)).toBeDisabled()
    },
}

export const RolesWithoutRulesWrite: Story = {
    args: {role: "rolesWithoutRules"},
    play: async () => {
        const panel = await drawer()
        // Roles change with the rule's save, which needs signing-rules-write.
        await expect(panel.getByText(label("readOnly.rules"))).toBeVisible()
        expect(panel.queryByRole("combobox", {name: label("rule.whoCanSign")})).toBeNull()
        expect(panel.queryByRole("button", {name: label("rule.save")})).toBeNull()
    },
}

export const EnableWithSigners: Story = {
    args: {
        organization: Organization.StudentCouncil,
        role: "rulesAndRoles",
        action: SigningAction.InitializeVoting,
    },
    play: async ({args}) => {
        const organization = organizationOf(Organization.StudentCouncil)
        const [role] = organization.roles
        const panel = await drawer()
        // Off, and nobody holds the sign permission yet.
        await userEvent.click(panel.getByLabelText(label("rule.needsSignatures")))
        await expect(
            await panel.findByText(label("validation.tooMany", {n: 1, max: 0}))
        ).toBeVisible()
        await expect(saveButton(panel)).toBeDisabled()
        // Choosing who signs makes the number the server's to check.
        await userEvent.click(panel.getByRole("combobox", {name: label("rule.whoCanSign")}))
        await userEvent.click(await within(document.body).findByRole("option", {name: role}))
        await expect(await panel.findByText(label("rule.checkedOnSave"))).toBeVisible()
        await userEvent.clear(signaturesField(panel))
        await userEvent.type(signaturesField(panel), "2")
        await userEvent.click(saveButton(panel))
        await waitFor(() => expect(args.onSaved).toHaveBeenCalledOnce())
        expect(savedRule()[0].variables).toMatchObject({
            action: SigningAction.InitializeVoting,
            requirement: SigningRequirement.Required,
            signatures: 2,
            roles: {add: [roleId(role)], remove: []},
        })
    },
}

export const SavedWithShortPosts: Story = {
    args: {
        role: "rulesAndRoles",
        shortPosts: [{election_id: "dili", name: "Dili PE", count: 1}],
    },
    play: async ({args}) => {
        const panel = await drawer()
        await userEvent.click(panel.getByLabelText(label("rule.requesterSigning")))
        await userEvent.click(saveButton(panel))
        // The server counts the saved roles and names the Posts that can't reach the number.
        await notified(label("rule.savedShort", {count: 1, posts: "Dili PE"}))
        expect(args.onSaved).toHaveBeenCalledOnce()
    },
}

export const EventWideTooMany: Story = {
    args: {action: SigningAction.ApproveConfiguration},
    play: async () => {
        const {max} = capacityOf(
            organizationOf(Organization.Overseas),
            SigningAction.ApproveConfiguration
        )
        const panel = await drawer()
        await userEvent.clear(signaturesField(panel))
        await userEvent.type(signaturesField(panel), String(max + 1))
        // An event-wide action has no Posts to name.
        await expect(
            await panel.findByText(label("validation.tooManyEvent", {n: max + 1, max}))
        ).toBeVisible()
        await expect(saveButton(panel)).toBeDisabled()
    },
}

/** Saves a changed requester setting, which the server refuses. */
const refusedSave =
    (message: string): Story["play"] =>
    async ({args}) => {
        const panel = await drawer()
        await userEvent.click(panel.getByLabelText(label("rule.requesterSigning")))
        await userEvent.click(saveButton(panel))
        await notified(message)
        const explanation = panel.getByText(message)
        await expect(explanation).toBeVisible()
        await expect(explanation.closest("[role=alert]")).toHaveClass(/MuiAlert-colorError/)
        expect(within(document.body).getAllByText(message)).toHaveLength(1)
        expect(document.querySelector(".MuiSnackbar-root")).toBeNull()
        expect(args.onSaved).not.toHaveBeenCalled()
    }

export const RefusedForbidden: Story = {
    args: {refuse: {code: "forbidden"}},
    play: refusedSave(label("errors.forbidden")),
}

export const RefusedInvalid: Story = {
    args: {refuse: {code: "invalid"}},
    play: refusedSave(label("errors.invalid")),
}

export const RefusedStaleRevision: Story = {
    args: {refuse: {code: "conflict"}},
    play: refusedSave(label("errors.conflict")),
}

export const RefusedLockedDown: Story = {
    args: {refuse: {code: "locked-down"}},
    play: refusedSave(label("errors.lockedDown")),
}

const automatedCeremoniesRefusal: Story["play"] = async ({args}) => {
    const panel = await drawer()
    const toggle = panel.getByLabelText(label("rule.trusteesSign"))
    await userEvent.click(toggle)
    await userEvent.click(saveButton(panel))
    const message =
        "This event uses automatic key ceremonies. Trustees do not perform these steps, so their signatures cannot be required. To require trustee signatures, use manual key ceremonies."
    const alert = await panel.findByRole("alert")
    await expect(alert).toHaveTextContent(message)
    await expect(alert).toHaveClass(/MuiAlert-colorError/)
    expect(within(document.body).getAllByText(message)).toHaveLength(1)
    expect(document.querySelector(".MuiSnackbar-root")).toBeNull()
    expect(args.onSaved).not.toHaveBeenCalled()
    await userEvent.click(toggle)
    expect(panel.queryByText(message)).toBeNull()
}

export const AutomaticKeyCeremonyRefused: Story = {
    args: {
        organization: Organization.StudentCouncil,
        action: SigningAction.ConfirmKeyShare,
        refuse: {code: "invalid", reason: "automated-ceremonies"},
    },
    play: automatedCeremoniesRefusal,
}

export const AutomaticTallyStepRefused: Story = {
    args: {
        organization: Organization.StudentCouncil,
        action: SigningAction.ContributeKeyShare,
        refuse: {code: "invalid", reason: "automated-ceremonies"},
    },
    play: automatedCeremoniesRefusal,
}
