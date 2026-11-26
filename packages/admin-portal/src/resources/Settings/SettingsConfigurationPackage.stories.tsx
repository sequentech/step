// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within, waitFor} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {storyId} from "@/__stories__/fixtures"
import {atViewport} from "../../../../test-support/storybook/viewport"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsConfigurationPackage} from "./SettingsConfigurationPackage"
import {COMPLETE_DELIVERY, PACKAGE_PERMISSIONS} from "./configurationPackage"
import {record, refusedPreview} from "./configurationPackage.fixture"

interface Scenario {
    allowed: boolean
    /** Package permissions the operator lacks, for the Missing permissions state. */
    missing: string[]
    /** Whether the tenant's history holds an applied run. */
    applied: boolean
    blocked: boolean
    unavailable: boolean
    managedRoster: boolean
}
const DOCUMENT_ID = storyId(41, 1)
const TASK_ID = storyId(41, 2)
let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
const meta = {
    title: "Admin/Settings/SettingsConfigurationPackage",
    component: SettingsConfigurationPackage,
    args: {
        allowed: true,
        missing: [],
        applied: false,
        blocked: false,
        unavailable: false,
        managedRoster: false,
    },
    beforeEach: async ({args}) => {
        sessionStorage.clear()
        // The page also shows the tenant's signing card, which reads the
        // tenant record; no signing configured yet.
        data = resourceBoundary({sequent_backend_tenant: [{id: TENANT_ID, settings: {}}]})
        const annotations = args.blocked
            ? refusedPreview(["Unknown role: synthetic-reviewer"], {
                  missingTrustees: true,
                  missingCensus: true,
              })
            : record("previewed")
        annotations.application.tenantId = TENANT_ID
        annotations.application.packageSha256 = "ab".repeat(32)
        // A readable proposal, not credentials or arbitrary raw configuration.
        const tenantSection = annotations.application.sections[0] as {
            status: {preview: {changes: unknown[]}}
        }
        tenantSection.status.preview.changes = [{action: "update", keys: ["language_conf"]}]
        if (!args.blocked) {
            Object.assign(annotations.application, {
                managedRequirements: COMPLETE_DELIVERY,
                messagingPlanSha256: "cd".repeat(32),
            })
            Object.assign(annotations.application.coverage.sections, {
                "messaging-accounts": {
                    state: "present",
                    members: ["tenant_messaging_accounts.json"],
                },
            })
            annotations.application.sections.push({
                section: "messaging-accounts",
                members: ["tenant_messaging_accounts.json"],
                status: {
                    state: "ready",
                    preview: {
                        needsApply: true,
                        blockers: [],
                        changes: [
                            {
                                reference: "package:training-notices",
                                action: "replace",
                                channel: "EMAIL",
                                provider: "AWS_SES",
                            },
                        ],
                    },
                },
            })
        }
        let approvalPolicy: {required_approvals: number | null; generation: string} = {
            required_approvals: null,
            generation: "0",
        }
        graphql = graphqlBoundary({
            ArchitectApprovalPolicy: () => ({
                data: {get_architect_approval_policy: approvalPolicy},
            }),
            SetArchitectApprovalPolicy: ({variables}) => {
                approvalPolicy = {
                    required_approvals: variables.count ?? null,
                    generation: String(Number(approvalPolicy.generation) + 1),
                }
                return {data: {set_architect_approval_policy: approvalPolicy}}
            },
            ArchitectSignerRosterStatus: () => ({
                data: {
                    get_architect_signer_roster_status: {
                        __typename: "ArchitectSignerRosterStatus",
                        mode: args.managedRoster ? "portal_managed" : "not_enabled",
                        generation: "0",
                        active_sha256: null,
                        required_approvals: 2,
                        can_manage: args.managedRoster,
                    },
                },
            }),
            ListPackageApplications: () => ({
                data: {
                    sequent_backend_tasks_execution: args.applied
                        ? [
                              {
                                  id: storyId(41, 3),
                                  created_at: "2026-11-24T14:02:00Z",
                                  execution_status: "SUCCESS",
                                  executed_by_user: "story-operator",
                                  annotations: record("applied"),
                              },
                          ]
                        : [],
                },
            }),
            ApplyConfigurationPackage: () => ({
                data: {
                    apply_configuration_package: {
                        task_execution: {
                            id: TASK_ID,
                            execution_status: "IN_PROGRESS",
                            annotations: null,
                        },
                    },
                },
            }),
            GetPackageTask: () => ({
                data: {
                    sequent_backend_tasks_execution: args.unavailable
                        ? []
                        : [
                              {
                                  id: TASK_ID,
                                  execution_status: args.blocked ? "FAILED" : "SUCCESS",
                                  annotations,
                                  logs: [],
                              },
                          ],
                },
            }),
        })
        await graphql.ready
        return () => sessionStorage.clear()
    },
    render: ({allowed, missing}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={
                allowed
                    ? PACKAGE_PERMISSIONS.filter((permission) => !missing.includes(permission))
                    : []
            }
            auth={{userId: "story-operator"}}
        >
            <SettingsConfigurationPackage />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function showAdvanced(canvasElement: HTMLElement) {
    const details = within(canvasElement).getByTestId(
        "package-advanced-details"
    ) as HTMLDetailsElement
    if (!details.open) {
        const summary = details.querySelector("summary")
        if (!summary) throw new Error("Advanced options has no disclosure")
        await userEvent.click(summary)
    }
    await expect(details).toHaveAttribute("open")
}

async function preview(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await waitFor(() =>
        expect(
            canvas.getByTestId("package-upload")
        ).toBeEnabled()
    )
    await showAdvanced(canvasElement)
    await userEvent.type(
        canvas.getByRole("textbox", {name: i18n.t("configurationPackage.documentId")}),
        DOCUMENT_ID
    )
    await userEvent.click(
        canvas.getByRole("button", {name: i18n.t("configurationPackage.preview")})
    )
    await canvas.findByTestId("package-task")
}

export const PreviewAndConfirm: Story = {
    parameters: {
        widgets: [
            "SettingsConfigurationPackageForScope",
            "ApplicationTable",
            "ProposedChanges",
            "PortableReplacementWarning",
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByTestId("package-apply")).not.toBeVisible()
        await preview(canvasElement)
        await waitFor(() => expect(canvas.getByTestId("package-apply")).toBeEnabled())
        await expect(canvas.getByTestId("package-members")).toHaveTextContent(
            "export_voters-e1.csv"
        )
        await expect(canvas.getByTestId("section-tenant-configuration")).toHaveTextContent(
            "language_conf"
        )
        await userEvent.click(canvas.getByTestId("package-apply"))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByTestId("package-proposed-changes")).toHaveTextContent(
            "language_conf"
        )
        await expect(
            dialog.getByText(i18n.t("configurationPackage.messagingReplaceWarning"))
        ).toBeVisible()

        // Opening confirmation never applies; the only mutation so far is preview.
        expect(
            graphql.calls
                .filter(({name}) => name === "ApplyConfigurationPackage")
                .map(({variables}) => variables.mode)
        ).toEqual(["preview"])
        expect(graphql.unexpected).toEqual([])
    },
}

export const BlockedPreview: Story = {
    args: {blocked: true},
    parameters: {widgets: ["ApplicationTable"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await preview(canvasElement)
        await expect(await canvas.findByTestId("section-tenant-configuration")).toHaveTextContent(
            "Unknown role: synthetic-reviewer"
        )
        const alert = await canvas.findByTestId("package-blocked")
        await expect(within(alert).getByText(i18n.t("configurationPackage.blocked"))).toBeVisible()
        await expect(
            within(alert).getByText(i18n.t("configurationPackage.workflow.missingCensus"))
        ).toBeVisible()
        await expect(canvas.getByTestId("section-trustees")).toHaveTextContent("missing")
        await expect(canvas.getByTestId("package-apply")).toBeDisabled()
        await expect(canvas.getByTestId("package-apply")).not.toBeVisible()
        await expect(canvas.getByTestId("package-upload")).toHaveTextContent(
            i18n.t("configurationPackage.hub.actions.importFixed")
        )
        await expect(canvas.getByTestId("package-change-requirements")).toBeVisible()
        expect(graphql.calls.filter(({name}) => name === "ApplyConfigurationPackage")).toHaveLength(
            1
        )
        expect(graphql.unexpected).toEqual([])
    },
}

export const TaskUnavailable: Story = {
    args: {unavailable: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await preview(canvasElement)
        await waitFor(() =>
            expect(canvas.getByTestId("package-outcome")).toHaveTextContent(
                i18n.t("configurationPackage.notFound")
            )
        )
        await expect(canvas.getByTestId("package-apply")).toBeDisabled()
        await userEvent.click(canvas.getByTestId("package-stop-following"))
        await expect(canvas.queryByTestId("package-run")).not.toBeInTheDocument()
        expect(graphql.calls.filter(({name}) => name === "ApplyConfigurationPackage")).toHaveLength(
            1
        )
        expect(graphql.unexpected).toEqual([])
    },
}

export const PermissionRequired: Story = {
    args: {allowed: false},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("package-not-permitted")).toBeVisible()
        await expect(
            within(canvasElement).queryByText(i18n.t("architectApprovalPolicy.title"))
        ).not.toBeInTheDocument()
        expect(graphql.calls).toEqual([])
    },
}

export const UploadFirst: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(
                canvas.getByTestId("package-upload")
            ).toBeEnabled()
        )
        await expect(
            canvas.getByRole("region", {name: i18n.t("configurationPackage.advanced")})
        ).toBeVisible()
        await expect(canvas.getByTestId("package-apply")).not.toBeVisible()
        await expect(canvas.getByTestId("package-advanced-details")).not.toHaveAttribute("open")
        await showAdvanced(canvasElement)
        await expect(
            canvas.getByRole("button", {name: i18n.t("configurationPackage.legacySetup")})
        ).toHaveAttribute("aria-expanded", "false")
        await userEvent.click(
            canvas.getByTestId("package-upload")
        )
        await expect(within(document.body).getByTestId("package-upload-drawer")).toBeVisible()
    },
}

export const RosterNotInstalled: Story = {
    args: {managedRoster: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByTestId("package-upload")
        ).toBeDisabled()
        const upload = canvas.getByTestId("package-upload")
        const disabled = window.getComputedStyle(upload)
        const reference = window.getComputedStyle(canvas.getByTestId("package-apply"))
        await expect([disabled.backgroundColor, disabled.color, disabled.border]).toEqual([
            reference.backgroundColor,
            reference.color,
            reference.border,
        ])
        await expect(upload).toHaveAttribute("aria-describedby")
        await showAdvanced(canvasElement)
        await expect(
            canvas.getByRole("textbox", {name: i18n.t("configurationPackage.signedDocumentId")})
        ).toBeDisabled()
        await expect(
            canvas.queryByRole("button", {name: i18n.t("configurationPackage.legacySetup")})
        ).not.toBeInTheDocument()
        await userEvent.click(canvas.getByTestId("package-rail-trust"))
        const manage = await canvas.findByRole("button", {
            name: i18n.t("configurationPackage.roster.manage"),
        })
        await expect(canvas.getByText(i18n.t("configurationPackage.roster.noRoster"))).toBeVisible()
        await userEvent.click(manage)
        const dialog = within(
            await within(document.body).findByRole("dialog", {
                name: i18n.t("configurationPackage.roster.manage"),
            })
        )
        await expect(
            dialog.getByRole("button", {name: i18n.t("configurationPackage.roster.browse")})
        ).toBeVisible()
        await expect(
            dialog.getByRole("button", {name: i18n.t("configurationPackage.roster.review")})
        ).toBeDisabled()
        await expect(
            dialog.queryByRole("button", {name: i18n.t("configurationPackage.roster.confirm")})
        ).not.toBeInTheDocument()
        expect(
            graphql.calls.filter(({name}) => /^(Activate|Preview)ArchitectSignerRoster$/.test(name))
        ).toHaveLength(0)
        expect(graphql.unexpected).toEqual([])
    },
}

export const MissingPermissions: Story = {
    args: {missing: ["role-write", "trustee-create"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const missing = await canvas.findByTestId("package-missing-permissions")
        await expect(missing).toHaveTextContent("role-write, trustee-create")
        await expect(missing).not.toHaveTextContent("tenant-write")
        await expect(canvas.queryByTestId("package-upload")).not.toBeInTheDocument()
        expect(graphql.calls).toEqual([])
    },
}

export const AppliedReceipt: Story = {
    args: {applied: true},
    parameters: {widgets: ["ApplicationHistory"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(canvas.getByTestId("package-receipt")).toHaveTextContent(storyId(41, 3))
        )
        await expect(canvas.getByTestId("package-run-card")).not.toHaveTextContent(
            storyId(41, 3)
        )
        await userEvent.click(canvas.getByTestId("package-rail-history"))
        await expect(canvas.getByTestId("package-history")).toHaveTextContent(storyId(41, 3))
    },
}

export const PhoneBlockedPreview: Story = {
    ...atViewport(390),
    args: {blocked: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await preview(canvasElement)
        await expect(await canvas.findByTestId("package-change-requirements")).toBeVisible()
        await expect(canvas.getByTestId("package-upload")).toHaveTextContent(
            i18n.t("configurationPackage.hub.actions.importFixed")
        )
        await expect(canvas.getByTestId("package-apply")).not.toBeVisible()
        // Nothing wider than the phone: the rail and columns stack.
        await expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(390)
        await userEvent.click(canvas.getByTestId("package-change-requirements"))
        await expect(
            within(document.body).getByTestId("package-requirements-picker")
        ).toBeVisible()
        expect(graphql.calls.filter(({name}) => name === "ApplyConfigurationPackage")).toHaveLength(
            1
        )
    },
}

export const StalePreviewAgain: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await preview(canvasElement)
        await waitFor(() => expect(canvas.getByTestId("package-apply")).toBeEnabled())
        await userEvent.click(canvas.getByRole("checkbox", {name: "census"}))
        await expect(canvas.getByTestId("package-apply")).toBeDisabled()
        await userEvent.click(await canvas.findByTestId("package-preview-again"))
        await waitFor(() =>
            expect(
                graphql.calls
                    .filter(({name}) => name === "ApplyConfigurationPackage")
                    .map(({variables}) => [variables.mode, variables.requirements?.census])
            ).toEqual([
                ["preview", "required"],
                ["preview", "optional"],
            ])
        )
        expect(graphql.unexpected).toEqual([])
    },
}
