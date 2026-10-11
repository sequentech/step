// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EMessageChannel} from "@/types/messaging"
import type {IMonthlyVolume} from "@/queries/GetMessagingMonthlyVolumes"
import {MessagingMonthlyVolumes} from "./MessagingMonthlyVolumes"

interface Scenario {
    roles: string[]
    /** The month's rows, or `"FAIL"` when the volumes can't be read. */
    rows: IMonthlyVolume[] | "FAIL"
}

const WHATSAPP_ACCOUNT = storyId(9, 40)
const VIBER_ACCOUNT = storyId(9, 41)

/**
 * A second organization's October: WhatsApp and Viber by country, email with
 * no account (the deployment's sender or a deleted account), and the
 * attempts of a WhatsApp account deleted since. Categories as the backend
 * records them.
 */
const OCTOBER: IMonthlyVolume[] = [
    {
        channel: EMessageChannel.WHATSAPP,
        account_id: WHATSAPP_ACCOUNT,
        account_name: "Staff Association WhatsApp",
        destination_country: "ES",
        billing_category: "AUTHENTICATION",
        billable: true,
        messages: 1820,
    },
    {
        channel: EMessageChannel.WHATSAPP,
        account_id: WHATSAPP_ACCOUNT,
        account_name: "Staff Association WhatsApp",
        destination_country: "ES",
        billing_category: "UTILITY",
        billable: false,
        messages: 312,
    },
    {
        channel: EMessageChannel.VIBER,
        account_id: VIBER_ACCOUNT,
        account_name: "Staff Association Viber",
        destination_country: "PT",
        billing_category: "TRANSACTIONAL",
        billable: null,
        messages: 45,
    },
    {
        channel: EMessageChannel.EMAIL,
        account_id: null,
        account_name: null,
        destination_country: null,
        billing_category: "EMAIL",
        billable: null,
        messages: 2400,
    },
    {
        channel: EMessageChannel.WHATSAPP,
        account_id: null,
        account_name: null,
        destination_country: "ES",
        billing_category: null,
        billable: null,
        messages: 3,
    },
]

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Settings/MessagingMonthlyVolumes",
    component: MessagingMonthlyVolumes,
    args: {roles: [IPermissions.MESSAGING_ACCOUNT_READ], rows: OCTOBER},
    beforeEach: async ({args}) => {
        // The messaging operations are not in the generated schema yet.
        graphql = graphqlBoundary(
            {
                GetMessagingMonthlyVolumes: ({variables}) => {
                    if (args.rows === "FAIL") throw new Error("Synthetic volumes failure")
                    return {
                        data: {
                            messaging_monthly_volumes: {
                                month: variables.month,
                                rows: variables.month === "2026-10" ? args.rows : [],
                            },
                        },
                    }
                },
            },
            {schema: false}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} roles={roles}>
            <MessagingMonthlyVolumes initialMonth="2026-10" />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const OneMonth: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const table = within(await canvas.findByRole("table", {name: "Monthly volumes"}))
        expect(table.getAllByRole("columnheader").map((header) => header.textContent)).toEqual([
            "Channel",
            "Sent from",
            "Destination country",
            "Billing category",
            "Billable",
            "Attempts",
        ])
        expect(
            table
                .getAllByRole("row")
                .slice(1, -1)
                .map((row) => row.getAttribute("data-channel"))
        ).toEqual([
            EMessageChannel.WHATSAPP,
            EMessageChannel.WHATSAPP,
            EMessageChannel.VIBER,
            EMessageChannel.EMAIL,
            EMessageChannel.WHATSAPP,
        ])
        expect(table.getAllByText("Spain (ES)")).toHaveLength(3)
        // Categories in the viewer's language, never the raw value.
        for (const category of ["Authentication", "Utility", "Transactional"]) {
            expect(table.getByRole("cell", {name: category})).toBeVisible()
        }
        expect(canvasElement.textContent).not.toMatch(/AUTHENTICATION|UTILITY|TRANSACTIONAL/)
        // No account: on email the deployment's sender or a deleted account,
        // on WhatsApp only a deleted one; never the deployment's alone.
        expect(table.getByText("Deployment sender or a deleted account")).toBeVisible()
        expect(table.getByText("Deleted account")).toBeVisible()
        expect(table.queryByText("The deployment's default sender")).toBeNull()
        expect(table.getAllByText("Not reported").length).toBeGreaterThan(0)
        expect(table.getByText("4,580")).toBeVisible()
        expect(graphql.calls[0]).toMatchObject({
            name: "GetMessagingMonthlyVolumes",
            variables: {month: "2026-10"},
            headers: {"x-hasura-role": IPermissions.MESSAGING_ACCOUNT_READ},
        })
        expect(canvasElement.textContent).not.toMatch(/COMELEC/i)
    },
}

/** Choosing another month reads it; a month with nothing sent says so. */
export const AMonthWithNothingSent: Story = {
    args: {roles: [IPermissions.MESSAGING_CONFIG_READ]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("table", {name: "Monthly volumes"})
        const month = canvas.getByLabelText("Month")
        await userEvent.clear(month)
        await userEvent.type(month, "2026-09")
        await expect(
            await canvas.findByText("No attempt was handed to a provider in 2026-09.")
        ).toBeVisible()
        await waitFor(() =>
            expect(graphql.calls.at(-1)).toMatchObject({
                variables: {month: "2026-09"},
                headers: {"x-hasura-role": IPermissions.MESSAGING_CONFIG_READ},
            })
        )
    },
}

export const CouldNotBeLoaded: Story = {
    args: {rows: "FAIL"},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("The volumes could not be loaded.")
        ).toBeVisible()
    },
}

export const WithoutAReadPermission: Story = {
    args: {roles: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(/needs the messaging-account-read or messaging-config-read/)
        ).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

export const FluidDescription: Story = {
    play: async ({canvasElement}) => {
        const intro = await within(canvasElement).findByText(
            /Attempts handed to a provider each month/
        )
        const container = intro.parentElement as HTMLElement
        const originalWidth = container.style.width
        try {
            for (const width of [1200, 600]) {
                container.style.width = `${width}px`
                await waitFor(() => {
                    const heading = within(container).getByRole("heading", {
                        name: "Monthly volumes",
                    })
                    expect(intro.getBoundingClientRect().width).toBeCloseTo(
                        heading.getBoundingClientRect().width,
                        0
                    )
                })
            }
        } finally {
            container.style.width = originalWidth
        }
    },
}
