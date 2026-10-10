// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {EElectionEventCeremoniesPolicy} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {trusteeRecords} from "@/__stories__/fixtures"
import type {Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
} from "@/services/KeyCeremony"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {AdminWizard} from "./AdminWizard"
import {KEYS_CEREMONY_RESOURCE, ceremony, ceremonyEvent} from "./__stories__/KeysCeremonyFixture"

interface Scenario {
    /** The ceremony the wizard opens with; none starts at configuration. */
    current: EStatus | "none"
    /** The ceremony's status when the wizard reads it again. */
    stored: EStatus
    automated: boolean
    /** Whether the election event is still loading. */
    loadingEvent: boolean
    goBack: () => void
    setCurrentCeremony: (ceremony: Sequent_Backend_Keys_Ceremony) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const policyOf = (automated: boolean) =>
    automated
        ? EElectionEventCeremoniesPolicy.AUTOMATED_CEREMONIES
        : EElectionEventCeremoniesPolicy.MANUAL_CEREMONIES

const statusCeremony = (execution: EStatus, automated: boolean) =>
    ceremony(
        execution,
        trusteeRecords.map(() =>
            execution === EStatus.SUCCESS ? TStatus.KEY_CHECKED : TStatus.KEY_GENERATED
        ),
        {settings: {policy: policyOf(automated)}}
    )

/** Keeps the current ceremony as the keys tab does. */
function Fixture({current, automated, loadingEvent, goBack, setCurrentCeremony}: Scenario) {
    const [ceremonyState, setCeremonyState] = useState(
        current === "none" ? null : statusCeremony(current, automated)
    )
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={EStoryPermissions.ADMIN}
        >
            <AdminWizard
                electionEvent={loadingEvent ? undefined : ceremonyEvent(policyOf(automated))}
                currentCeremony={ceremonyState}
                setCurrentCeremony={(next) => {
                    setCurrentCeremony(next)
                    setCeremonyState(next)
                }}
                goBack={goBack}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Keys ceremony/AdminWizard",
    component: AdminWizard,
    args: {
        current: EStatus.IN_PROGRESS,
        stored: EStatus.IN_PROGRESS,
        automated: false,
        loadingEvent: false,
        goBack: fn(),
        setCurrentCeremony: fn(),
    },
    argTypes: {
        current: {control: "select", options: ["none", ...Object.values(EStatus)]},
        stored: {control: "select", options: Object.values(EStatus)},
    },
    parameters: {
        expectedFailure: {
            reason: "The ceremony status chip puts white text on its light status colour, and the progress and logs accordions are two unnamed regions.",
            a11y: ["color-contrast", "landmark-unique"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            [KEYS_CEREMONY_RESOURCE]: [statusCeremony(args.stored, args.automated)],
            sequent_backend_trustee: trusteeRecords,
            sequent_backend_election: [],
        })
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const currentStep = (canvasElement: HTMLElement) =>
    canvasElement.querySelector('[aria-current="step"]')

export const CeremonyInProgress: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Status: IN_PROGRESS")).toBeVisible()
        expect(currentStep(canvasElement)).toHaveTextContent("Ceremony")
        await waitFor(() =>
            expect(data.calls.map(({method, args: [resource]}) => `${method} ${resource}`)).toEqual(
                [`getOne ${KEYS_CEREMONY_RESOURCE}`]
            )
        )
        // A manual ceremony leaves the wizard where it is when the ceremony is read again.
        expect(args.setCurrentCeremony).not.toHaveBeenCalled()
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const Configure: Story = {
    args: {current: "none"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {name: "Create Election Event Key Ceremony"})
        ).toBeVisible()
        expect(currentStep(canvasElement)).toHaveTextContent("Configure")
        await waitFor(() =>
            expect(data.calls.map(({args: [resource]}) => resource)).toContain(
                "sequent_backend_trustee"
            )
        )
        expect(graphql.calls).toEqual([])
    },
}

export const Finished: Story = {
    args: {current: EStatus.SUCCESS, stored: EStatus.SUCCESS},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("Status: SUCCESS")).toBeVisible()
        expect(currentStep(canvasElement)).toHaveTextContent("Finished")
    },
}

export const AutomatedCeremonyAdvances: Story = {
    args: {automated: true, stored: EStatus.SUCCESS},
    play: async ({canvasElement, args}) => {
        await waitFor(() =>
            expect(args.setCurrentCeremony).toHaveBeenCalledWith(
                expect.objectContaining({execution_status: EStatus.SUCCESS})
            )
        )
        await waitFor(() => expect(currentStep(canvasElement)).toHaveTextContent("Finished"))
    },
}

export const LoadingEvent: Story = {
    args: {loadingEvent: true},
    parameters: {
        expectedFailure: {
            reason: "The wizard's loading spinner is a progressbar without an accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(currentStep(canvasElement)).toBeNull()
        expect(data.calls).toEqual([])
    },
}
