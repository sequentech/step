// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within, waitFor} from "storybook/test"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ConfigurationSigningSettings} from "./ConfigurationSigningSettings"

// Synthetic placeholders only: not real certificates or keys.
const ROOT = "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n"
const pin = (c: string) => c.repeat(64)
const SIGNING = {
    policy: "required",
    package_roots: ROOT,
    staff_roots: ROOT,
    required_approvals: 2,
    minimum_package_format: "v2",
    final_signer_separation: "separate",
    signers: [
        {
            id: "approver-1",
            name: "Story approver one",
            roles: ["approver"],
            keys: [{spki_sha256: pin("a")}],
        },
        {
            id: "approver-2",
            name: "Story approver two",
            roles: ["approver"],
            keys: [{spki_sha256: pin("b")}],
        },
        {
            id: "custodian",
            name: "Story key custodian",
            roles: ["final-signer"],
            keys: [{spki_sha256: pin("c")}],
        },
    ],
}
const CHECK = {
    anchors: [
        {
            setting: "package_roots",
            subject: "CN=Story Package Root",
            not_before: "2026-01-01T00:00:00Z",
            not_after: "2031-01-01T00:00:00Z",
            fingerprint_sha256: pin("d"),
        },
        {
            setting: "staff_roots",
            subject: "CN=Story Staff Root",
            not_before: "2026-01-01T00:00:00Z",
            not_after: "2031-01-01T00:00:00Z",
            fingerprint_sha256: pin("e"),
        },
    ],
    report: {problems: []},
    policy_sha256: pin("f"),
}

interface Scenario {
    saved: boolean
}
let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
const meta = {
    title: "Admin/Settings/ConfigurationSigningSettings",
    component: ConfigurationSigningSettings,
    args: {saved: true},
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            sequent_backend_tenant: [{id: TENANT_ID, settings: {configuration_signing: SIGNING}}],
        })
        graphql = graphqlBoundary({
            SetConfigurationSigning: () => ({
                data: {set_configuration_signing: {saved: args.saved, check: CHECK}},
            }),
            GetTenantSettings: () => ({
                data: {
                    sequent_backend_tenant_by_pk: {
                        id: TENANT_ID,
                        settings: {configuration_signing: SIGNING},
                    },
                },
            }),
        })
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ConfigurationSigningSettings tenantId={TENANT_ID} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>

export default meta
type Story = StoryObj<Scenario>

/** A format-2 policy as installed: enrolled signers, format 2 and separation. */
export const InstalledFormat2Policy: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByTestId("configuration-signing-signers")).toBeVisible()
        await expect(
            canvas.getByTestId("configuration-signing-signer-custodian")
        ).toHaveTextContent("final-signer")
    },
}

/** Check and save: the trusted roots and the policy digest the platform computed. */
export const CheckedAndSaved: Story = {
    parameters: {widgets: ["CheckResult"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByTestId("configuration-signing-save"))
        await waitFor(() =>
            expect(canvas.getByTestId("configuration-signing-result")).toBeVisible()
        )
        await expect(canvas.getByTestId("configuration-signing-policy-digest")).toHaveTextContent(
            pin("f")
        )
    },
}
