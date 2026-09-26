// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {keysCeremonyRecord, trusteeRecords} from "@/__stories__/fixtures"
import {ITallyTrusteeStatus} from "@/types/ceremonies"
import {
    TallyStoryContext,
    executionStatus,
    tallyExecution,
    tallySession,
} from "./__stories__/TallyFixture"
import {TallyTrusteesList} from "./TallyTrusteesList"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Trustees whose key fragment the tally has restored; the others wait. */
    restored: string[]
    /** Whether the event has a keys ceremony, which names the tally's trustees. */
    ceremony: boolean
    update: (selectedTrustees: boolean) => void
}

let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({restored, update}: Scenario) {
    const {workflow} = useStoryGlobals()
    const execution = tallyExecution(workflow, {
        status: {
            ...executionStatus(workflow),
            trustees: trusteeRecords.map(({name}) => ({
                name: String(name),
                status: restored.includes(String(name))
                    ? ITallyTrusteeStatus.KEY_RESTORED
                    : ITallyTrusteeStatus.WAITING,
            })),
        },
    })
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <TallyStoryContext>
                <TallyTrusteesList
                    tally={tallySession(workflow)}
                    tallySessionExecutions={[execution]}
                    update={update}
                />
            </TallyStoryContext>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyTrusteesList",
    component: TallyTrusteesList,
    args: {restored: ["trustee1", "trustee2"], ceremony: true, update: fn()},
    argTypes: {
        restored: {control: "inline-check", options: trusteeRecords.map(({name}) => name)},
        update: {table: {disable: true}},
    },
    beforeEach: ({args}) => {
        boundary = graphqlBoundary({})
        data = resourceBoundary({
            sequent_backend_keys_ceremony: args.ceremony ? [keysCeremonyRecord()] : [],
            sequent_backend_trustee: trusteeRecords,
        })
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const trusteeRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await trusteeRow(canvasElement, "trustee3")
        await expect(canvas.getByText(`2/3${i18n.t("tally.common.imported")}`)).toBeVisible()
        await expect(canvas.getByText(`2${i18n.t("tally.common.needed")}`)).toBeVisible()
        const waiting = await trusteeRow(canvasElement, "trustee3")
        expect(within(waiting).getByTestId("CachedIcon")).toBeInTheDocument()
        const restored = await trusteeRow(canvasElement, "trustee1")
        expect(within(restored).getByTestId("CheckCircleIcon")).toBeInTheDocument()
        // The threshold of two fragments is met, so the tally may continue.
        await waitFor(() => expect(args.update).toHaveBeenLastCalledWith(true))
    },
}

export const WaitingForFragments: Story = {
    args: {restored: ["trustee2"]},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await trusteeRow(canvasElement, "trustee1")
        await expect(canvas.getByText(`1/3${i18n.t("tally.common.imported")}`)).toBeVisible()
        await waitFor(() => expect(args.update).toHaveBeenLastCalledWith(false))
        expect(args.update).not.toHaveBeenCalledWith(true)
    },
}

export const Empty: Story = {
    args: {ceremony: false},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("tally.common.noTrustees"), {exact: false})
        ).toBeVisible()
        expect(canvas.queryByRole("grid")).toBeNull()
        expect(args.update).not.toHaveBeenCalledWith(true)
    },
}
