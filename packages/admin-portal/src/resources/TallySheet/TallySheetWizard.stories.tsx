// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import type {Identifier} from "react-admin"
import {initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaContestRecord, storyId} from "@/__stories__/fixtures"
import {
    AREAS,
    CANDIDATES,
    CONTEST,
    ELECTION,
    SHEET_IDS,
    TALLY_SHEETS,
    sheetContent,
    withAreaNameSearch,
} from "./__stories__/TallySheetFixture"
import {TallySheetWizard, WizardSteps} from "./TallySheetWizard"

interface Scenario {
    /** The step the list opened, and the sheet it opened it for. */
    action: number
    tallySheetId?: string
    /** Whether saving the new version fails. */
    createFails: boolean
    doAction: (action: number, id?: Identifier) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

/** The tally sheet screen: the wizard's steps change the page it shows. */
function Fixture({action, tallySheetId, doAction}: Scenario) {
    const [step, setStep] = useState({action, id: tallySheetId as Identifier | undefined})
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={withAreaNameSearch(data.provider)}>
            {step.action === WizardSteps.List ? (
                <p>Ballot boxes</p>
            ) : (
                <TallySheetWizard
                    election={ELECTION}
                    action={step.action}
                    tallySheetId={step.id}
                    doAction={(next, id) => {
                        doAction(next, id)
                        setStep((current) => ({action: next, id: id ?? current.id}))
                    }}
                />
            )}
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally sheet/TallySheetWizard",
    component: TallySheetWizard,
    args: {
        action: WizardSteps.Edit,
        tallySheetId: SHEET_IDS.latestPaper,
        createFails: false,
        doAction: fn(),
    },
    argTypes: {doAction: {table: {disable: true}}},
    parameters: {
        widgets: ["EditTallySheet"],
    },
    beforeEach: async ({args}) => {
        // The form checks its counts with sequent-core.
        await initCore()
        localStorage.removeItem("tallySheetData")
        data = resourceBoundary({
            sequent_backend_tally_sheet: TALLY_SHEETS,
            sequent_backend_contest: [CONTEST],
            sequent_backend_candidate: CANDIDATES,
            sequent_backend_area: AREAS,
            sequent_backend_area_contest: AREAS.map(({id}, index) =>
                areaContestRecord({id: storyId(7, index), area_id: id})
            ),
        })
        graphql = graphqlBoundary(
            {
                sequent_backend_contest_extended: () => ({
                    data: {
                        sequent_backend_area_contest: AREAS.map(({id, name}) => ({
                            area: {id, name},
                        })),
                    },
                }),
                CreateNewTallySheet: ({variables}) =>
                    args.createFails
                        ? {errors: [new GraphQLError("Synthetic tally sheet rejected")]}
                        : {
                              data: {
                                  create_new_tally_sheet: {
                                      ...TALLY_SHEETS[2],
                                      id: storyId(0, 7),
                                      version: 4,
                                      status: "PENDING",
                                      import_id: undefined,
                                      content: variables.content,
                                  },
                              },
                          },
            },
            {schema: true}
        )
        await graphql.ready
        return () => localStorage.removeItem("tallySheetData")
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const nextButton = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).getByRole("button", {name})

async function editLoaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await waitFor(() =>
        expect(canvas.getByRole("textbox", {name: "Total Valid Votes"})).toHaveValue("55")
    )
    await waitFor(() => expect(nextButton(canvasElement, "Confirm")).toBeEnabled())
    return canvas
}

async function confirmEdit(canvasElement: HTMLElement) {
    await editLoaded(canvasElement)
    await userEvent.click(nextButton(canvasElement, "Confirm"))
    await within(canvasElement).findByRole("button", {name: "Save"}, {timeout: 3000})
}

export const EditAVersion: Story = {
    play: async ({canvasElement}) => {
        const canvas = await editLoaded(canvasElement)
        // The areas load once the contest's areas are known.
        await waitFor(() =>
            expect(canvas.getByRole("combobox", {name: "Search Area"})).toHaveValue(
                "North district"
            )
        )
        expect(graphql.calls).toEqual([])
        expect(data.writes).toEqual([])
    },
}

export const ConfirmAndSaveANewVersion: Story = {
    parameters: {
        widgets: ["EditTallySheet", "ShowTallySheet"],
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await confirmEdit(canvasElement)
        expect(args.doAction).toHaveBeenCalledWith(WizardSteps.Confirm, undefined)
        await canvas.findByText("Alice")
        await userEvent.click(nextButton(canvasElement, "Save"))
        const saved = await within(document.body).findByText("Tally Sheet saved")
        await waitFor(() => expect(saved).toBeVisible())
        expect(
            graphql.calls.map(({name, variables, headers}) => ({name, variables, headers}))
        ).toEqual([
            expect.objectContaining({name: "sequent_backend_contest_extended"}),
            {
                name: "CreateNewTallySheet",
                variables: {
                    electionEventId: EVENT_ID,
                    channel: "PAPER",
                    content: expect.objectContaining({
                        ...sheetContent(STORY_IDS.area, [32, 20]),
                    }),
                    contestId: STORY_IDS.contest,
                    areaId: STORY_IDS.area,
                },
                headers: expect.objectContaining({"x-hasura-role": "tally-sheet-create"}),
            },
        ])
        await waitFor(() =>
            expect(args.doAction).toHaveBeenLastCalledWith(WizardSteps.List, undefined)
        )
        await expect(canvas.getByText("Ballot boxes")).toBeVisible()
    },
}

export const SaveFailure: Story = {
    args: {createFails: true},
    parameters: {
        widgets: ["EditTallySheet", "ShowTallySheet"],
    },
    play: async ({canvasElement, args}) => {
        await confirmEdit(canvasElement)
        await within(canvasElement).findByText("Alice")
        await userEvent.click(nextButton(canvasElement, "Save"))
        const failed = await within(document.body).findByText("Error saving Tally Sheet")
        await waitFor(() => expect(failed).toBeVisible())
        // The wizard returns to the list whether or not the version was saved.
        await waitFor(() =>
            expect(args.doAction).toHaveBeenLastCalledWith(WizardSteps.List, undefined)
        )
    },
}

export const ViewAVersion: Story = {
    args: {action: WizardSteps.View, tallySheetId: SHEET_IDS.firstPaper},
    parameters: {
        widgets: ["ShowTallySheet"],
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Alice")
        await waitFor(() =>
            expect(
                canvasElement.querySelector<HTMLInputElement>(`input[id="${STORY_IDS.candidate}"]`)
                    ?.value
            ).toBe("30")
        )
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        expect(args.doAction).toHaveBeenCalledWith(WizardSteps.List, undefined)
    },
}

export const StartANewBallotBox: Story = {
    args: {action: WizardSteps.Start, tallySheetId: undefined},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("combobox", {name: "Search Area"})).toHaveValue("")
        await expect(canvas.getByRole("button", {name: "Next"})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        expect(args.doAction).toHaveBeenCalledWith(WizardSteps.List, undefined)
    },
}
