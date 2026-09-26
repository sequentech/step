// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME} from "@/__stories__/fixtures"
import {MiruServers} from "./MiruServers"
import {MIRU_SERVERS} from "./__stories__/MiruFixture"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/MiruServers",
    component: MiruServers,
    args: {
        servers: MIRU_SERVERS,
        serversSentTo: [
            {name: "ccs-north", sent_at: FIXED_TIME, status: "SUCCESS"},
            {name: "ccs-south", sent_at: FIXED_TIME, status: "FAILED"},
        ],
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MiruServers {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MiruServers>
export default meta
type Story = StoryObj<typeof meta>

/** The sending status of a server's row; the icons have no text alternative. */
const sendStatus = (canvasElement: HTMLElement, server: string) => {
    const row = within(canvasElement).getByRole("row", {name: new RegExp(server)})
    return within(row).queryByTestId("DoneOutlineIcon") ? "sent" : "pending"
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("columnheader", {
                name: i18n.t("tally.transmissionPackage.destinationServers.table.serverName"),
            })
        ).toBeVisible()
        expect(canvas.getAllByRole("row")).toHaveLength(4)
        // Only a successful send counts: a failed attempt still shows as pending.
        expect(sendStatus(canvasElement, "ccs-north")).toBe("sent")
        expect(sendStatus(canvasElement, "ccs-south")).toBe("pending")
        expect(sendStatus(canvasElement, "ccs-east")).toBe("pending")
    },
}

export const NotSentYet: Story = {
    args: {serversSentTo: []},
    play: async ({canvasElement}) => {
        for (const server of ["ccs-north", "ccs-south", "ccs-east"]) {
            expect(sendStatus(canvasElement, server)).toBe("pending")
        }
    },
}

export const Empty: Story = {
    args: {servers: [], serversSentTo: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("columnheader", {
                name: i18n.t("tally.transmissionPackage.destinationServers.table.sendStatus"),
            })
        ).toBeVisible()
        expect(canvas.getAllByRole("row")).toHaveLength(1)
    },
}
