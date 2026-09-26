// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import type {Sequent_Backend_Area} from "@/gql/graphql"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {areaRecords} from "@/__stories__/fixtures"
import {MiruSignatures} from "./MiruSignatures"
import {AREA_TRUSTEES_ANNOTATION, miruDocuments} from "./__stories__/MiruFixture"

const area = {...areaRecords()[0], annotations: AREA_TRUSTEES_ANNOTATION} as Sequent_Backend_Area

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/MiruSignatures",
    component: MiruSignatures,
    args: {area, signatures: miruDocuments(["sbei-1", "sbei-3"])[1].signatures},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MiruSignatures {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MiruSignatures>
export default meta
type Story = StoryObj<typeof meta>

/** Whether a member's row shows the signed icon; the icons have no text alternative. */
const hasSigned = (canvasElement: HTMLElement, member: string) => {
    const row = within(canvasElement).getByRole("row", {name: new RegExp(member)})
    return !!within(row).queryByTestId("DoneOutlineIcon")
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("columnheader", {
                name: i18n.t("tally.transmissionPackage.signatures.table.trusteeName"),
            })
        ).toBeVisible()
        expect(canvas.getAllByRole("row")).toHaveLength(4)
        expect(hasSigned(canvasElement, "sbei-1")).toBe(true)
        expect(hasSigned(canvasElement, "sbei-2")).toBe(false)
        expect(hasSigned(canvasElement, "sbei-3")).toBe(true)
    },
}

export const NobodySigned: Story = {
    args: {signatures: []},
    play: async ({canvasElement}) => {
        for (const member of ["sbei-1", "sbei-2", "sbei-3"]) {
            expect(hasSigned(canvasElement, member)).toBe(false)
        }
    },
}

export const AreaWithoutMembers: Story = {
    args: {area: {...area, annotations: {}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("columnheader", {
                name: i18n.t("tally.transmissionPackage.signatures.table.signed"),
            })
        ).toBeVisible()
        expect(canvas.getAllByRole("row")).toHaveLength(1)
    },
}

export const MalformedMembersAnnotation: Story = {
    args: {area: {...area, annotations: {"miru:area-trustee-users": "[not json"}}},
    play: async ({canvasElement}) => {
        // An unreadable member list shows no members instead of breaking the table.
        expect(within(canvasElement).getAllByRole("row")).toHaveLength(1)
    },
}
