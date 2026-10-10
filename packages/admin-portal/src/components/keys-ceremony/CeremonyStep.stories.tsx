// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {EElectionEventCeremoniesPolicy} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
} from "@/services/KeyCeremony"
import {CeremonyStep} from "./CeremonyStep"
import {KEYS_CEREMONY_RESOURCE, ceremony, ceremonyEvent} from "./__stories__/KeysCeremonyFixture"

interface Scenario {
    /** What reading the ceremony does. */
    reads: ReadState
    execution: EStatus
    /** Whether both the event and the ceremony follow the automated policy. */
    automated: boolean
    /** Whether the caller offers the next step and private key verification. */
    withNext: boolean
    isNextDisabled: boolean
    goBack: () => void
    goNext: () => void
    verifyPrivateKey: () => void
    setCurrentCeremony: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const policyOf = (automated: boolean) =>
    automated
        ? EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES
        : EElectionEventCeremoniesPolicy.MANUAL_CEREMONIES

const storyCeremony = ({execution, automated}: Pick<Scenario, "execution" | "automated">) =>
    ceremony(
        execution,
        // Listed out of order: the table sorts the trustees by name.
        execution === EStatus.SUCCESS
            ? [TStatus.KEY_CHECKED, TStatus.KEY_CHECKED, TStatus.KEY_CHECKED]
            : [TStatus.KEY_RETRIEVED, TStatus.WAITING, TStatus.KEY_CHECKED],
        {settings: {policy: policyOf(automated)}}
    )

const meta = {
    title: "Admin/Keys ceremony/CeremonyStep",
    component: CeremonyStep,
    args: {
        reads: "records",
        execution: EStatus.IN_PROGRESS,
        automated: false,
        withNext: true,
        isNextDisabled: false,
        goBack: fn(),
        goNext: fn(),
        verifyPrivateKey: fn(),
        setCurrentCeremony: fn(),
    },
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        execution: {control: "select", options: Object.values(EStatus)},
    },
    parameters: {
        expectedFailure: {
            reason: "The ceremony status chip puts white text on its light status colour, and the progress and logs accordions are two unnamed regions.",
            a11y: ["color-contrast", "landmark-unique"],
        },
    },
    beforeEach: async ({args}) => {
        const record = storyCeremony(args)
        data = resourceBoundary({[KEYS_CEREMONY_RESOURCE]: [record]}, {reads: args.reads})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({reads: _reads, execution: _execution, automated, withNext, ...args}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <CeremonyStep
                currentCeremonyId={STORY_IDS.keysCeremony}
                electionEvent={ceremonyEvent(policyOf(automated))}
                message={<p>Waiting for the trustees</p>}
                goBack={args.goBack}
                setCurrentCeremony={args.setCurrentCeremony}
                isNextDisabled={args.isNextDisabled}
                goNext={withNext ? args.goNext : undefined}
                verifyPrivateKey={withNext ? args.verifyPrivateKey : undefined}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** Name and icon of each trustee column, e.g. "trustee1 done done waiting". */
const trusteeRows = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .getAllByRole("row")
        .filter((row) => /^trustee/.test(row.textContent ?? ""))
        .map((row) =>
            [
                row.textContent,
                ...Array.from(row.querySelectorAll("svg")).map((icon) =>
                    icon.getAttribute("data-testid") === "HourglassEmptyIcon" ? "waiting" : "done"
                ),
            ].join(" ")
        )

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Status: IN_PROGRESS")).toBeVisible()
        await waitFor(() =>
            expect(trusteeRows(canvasElement)).toEqual([
                "trustee1 done done waiting",
                "trustee2 waiting waiting waiting",
                "trustee3 done done done",
            ])
        )
        await expect(canvas.getByText("Created keys ceremony")).toBeVisible()
        // Without a public key yet, the caller's message is shown.
        await expect(canvas.getByText("Waiting for the trustees")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled()
        expect(data.calls).toEqual([
            {
                method: "getOne",
                args: [
                    KEYS_CEREMONY_RESOURCE,
                    expect.objectContaining({id: STORY_IDS.keysCeremony}),
                ],
            },
        ])
        await waitFor(() =>
            expect(args.setCurrentCeremony).toHaveBeenCalledWith(
                expect.objectContaining({id: STORY_IDS.keysCeremony})
            )
        )
    },
}

export const Completed: Story = {
    args: {execution: EStatus.SUCCESS},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Status: SUCCESS")).toBeVisible()
        await waitFor(() =>
            expect(trusteeRows(canvasElement)).toEqual([
                "trustee1 done done done",
                "trustee2 done done done",
                "trustee3 done done done",
            ])
        )
        expect(canvas.queryByText("Waiting for the trustees")).not.toBeInTheDocument()
    },
}

export const AutomatedCeremony: Story = {
    args: {automated: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(trusteeRows(canvasElement)).toEqual([
                "trustee1 done",
                "trustee2 waiting",
                "trustee3 done",
            ])
        )
        // Trustees neither download nor check a key, and the ceremony advances by itself.
        expect(canvas.queryByText("Private Key Fragment Downloaded")).not.toBeInTheDocument()
        expect(canvas.queryByRole("button", {name: "Next"})).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Verify key"})).toBeVisible()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(data.calls).toHaveLength(1))
        await expect(canvas.getByText("Status: IN_PROGRESS")).toBeVisible()
        expect(trusteeRows(canvasElement)).toEqual([])
        await expect(canvas.getByText("No logs yet.")).toBeVisible()
    },
}

export const NextDisabled: Story = {
    args: {isNextDisabled: true},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("button", {name: "Next"})
        ).toBeDisabled()
    },
}

export const WithoutNextStep: Story = {
    args: {withNext: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Status: IN_PROGRESS")
        expect(canvas.queryByRole("button", {name: "Next"})).not.toBeInTheDocument()
        expect(canvas.queryByRole("button", {name: "Verify key"})).not.toBeInTheDocument()
    },
}

export const Navigation: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Verify key"}))
        expect(args.verifyPrivateKey).toHaveBeenCalledTimes(1)
        await userEvent.click(canvas.getByRole("button", {name: "Next"}))
        expect(args.goNext).toHaveBeenCalledTimes(1)
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const CollapseProgress: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const summary = await canvas.findByRole("button", {name: /Key Ceremony Progress/})
        expect(summary).toHaveAttribute("aria-expanded", "true")
        await userEvent.click(summary)
        await waitFor(() => expect(summary).toHaveAttribute("aria-expanded", "false"))
    },
}
