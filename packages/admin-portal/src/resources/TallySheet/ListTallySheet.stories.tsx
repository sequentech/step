// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import type {Identifier} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {FIXED_TIME} from "@/__stories__/fixtures"
import {IPermissions} from "@/types/keycloak"
import {EStatus} from "@/types/TallySheets"
import {
    AREAS,
    CONTEST,
    ELECTION,
    SHEET_IDS,
    SHEET_IMPORT,
    TALLY_SHEETS,
    withDistinctBallotBoxes,
} from "./__stories__/TallySheetFixture"
import {ListTallySheet} from "./ListTallySheet"
import {WizardSteps} from "./TallySheetWizard"

interface Scenario {
    roles: string[]
    /** Whether the election has any ballot box. */
    sheets: boolean
    /** Whether the review service rejects the review. */
    reviewFails: boolean
    doAction: (action: number, id?: Identifier) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const ALL_ROLES = [
    IPermissions.TALLY_SHEET_VIEW,
    IPermissions.TALLY_SHEET_CREATE,
    IPermissions.TALLY_SHEET_REVIEW,
]

const meta = {
    title: "Admin/Tally sheet/ListTallySheet",
    component: ListTallySheet,
    args: {roles: ALL_ROLES, sheets: true, reviewFails: false, doAction: fn()},
    argTypes: {doAction: {table: {disable: true}}},
    parameters: {
        expectedFailure: {
            reason:
                "The ballot box actions are icon buttons named only by a tooltip on their " +
                "hidden icon.",
            a11y: ["button-name"],
        },
    },
    beforeEach: async ({args}) => {
        localStorage.setItem("tallySheetData", JSON.stringify(TALLY_SHEETS[0]))
        data = resourceBoundary({
            sequent_backend_tally_sheet: args.sheets ? TALLY_SHEETS : [],
            sequent_backend_tally_sheet_import: [SHEET_IMPORT],
            sequent_backend_area: AREAS,
            sequent_backend_contest: [CONTEST],
        })
        graphql = graphqlBoundary(
            {
                ReviewTallySheet: ({variables}) => {
                    if (args.reviewFails) {
                        return {errors: [new GraphQLError("Synthetic review rejected")]}
                    }
                    const {import_id: _importId, ...sheet} = TALLY_SHEETS[2]
                    return {
                        data: {
                            review_tally_sheet: {
                                ...sheet,
                                status: String(variables.newStatus),
                                reviewed_at: FIXED_TIME,
                                reviewed_by_user_id: "reviewer.north",
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
        return () => localStorage.removeItem("tallySheetData")
    },
    render: ({roles, doAction}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={withDistinctBallotBoxes(data.provider)}
            roles={roles}
        >
            <ListTallySheet election={ELECTION} doAction={doAction} reload={null} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const ballotBox = (canvasElement: HTMLElement, channel: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(`^${channel} `)})

const cells = (row: HTMLElement) =>
    within(row)
        .getAllByRole("cell")
        .map((cell) => cell.textContent)

/** The row action whose tooltip has this title. */
const action = (row: HTMLElement, title: string) =>
    within(row).getByLabelText(title).closest("button") as HTMLButtonElement

async function reviewLatestPaperVersion(canvasElement: HTMLElement, decision: string) {
    await userEvent.click(action(await ballotBox(canvasElement, "PAPER"), "Versions"))
    const latest = await within(canvasElement).findByRole("row", {name: /^3 /})
    await userEvent.click(within(latest).getByLabelText(decision).closest("button")!)
    const dialog = await within(document.body).findByRole("dialog")
    await userEvent.click(within(dialog).getByRole("button", {name: decision}))
    await waitFor(() => expect(dialog).not.toBeInTheDocument())
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const paper = await ballotBox(canvasElement, "PAPER")
        await within(paper).findByText("North district")
        await within(paper).findByText("Members")
        // One row per ballot box: its latest version and its latest approved one.
        expect(cells(paper).slice(0, 7)).toEqual([
            "PAPER",
            "Members",
            "North district",
            "3",
            "1",
            '{"source":"ess"}',
            "{}",
        ])
        const postal = await ballotBox(canvasElement, "POSTAL")
        await within(postal).findByText("South district")
        expect(cells(postal).slice(3, 5)).toEqual(["1", "1"])
        expect(within(canvasElement).getAllByRole("row")).toHaveLength(3)
        // A draft left by the wizard is discarded.
        expect(localStorage.getItem("tallySheetData")).toBeNull()
        expect(data.writes).toEqual([])
        expect(graphql.calls).toEqual([])
    },
}

export const AddAVersion: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(action(await ballotBox(canvasElement, "PAPER"), "Add"))
        expect(args.doAction).toHaveBeenCalledWith(WizardSteps.Edit, SHEET_IDS.latestPaper)
    },
}

export const AddABallotBox: Story = {
    play: async ({canvasElement, args}) => {
        await ballotBox(canvasElement, "PAPER")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add"}))
        expect(args.doAction).toHaveBeenCalledWith(WizardSteps.Start)
    },
}

export const ApproveAVersion: Story = {
    parameters: {widgets: ["ListTallySheetVersions"]},
    play: async ({canvasElement}) => {
        await reviewLatestPaperVersion(canvasElement, "Approve")
        await waitFor(() =>
            expect(within(document.body).getByText("Tally sheet reviewed")).toBeVisible()
        )
        expect(graphql.calls).toEqual([
            {
                name: "ReviewTallySheet",
                variables: {
                    electionEventId: EVENT_ID,
                    tallySheetId: SHEET_IDS.latestPaper,
                    newStatus: EStatus.APPROVED,
                },
                headers: expect.objectContaining({"x-hasura-role": "tally-sheet-review"}),
            },
        ])
    },
}

export const DisapprovalFails: Story = {
    args: {reviewFails: true},
    parameters: {widgets: ["ListTallySheetVersions"]},
    play: async ({canvasElement}) => {
        await reviewLatestPaperVersion(canvasElement, "Disapprove")
        await waitFor(() =>
            expect(within(document.body).getByText("Error reviewing tally sheet")).toBeVisible()
        )
        expect(graphql.calls.map(({variables}) => variables.newStatus)).toEqual([
            EStatus.DISAPPROVED,
        ])
    },
}

export const CancelAReview: Story = {
    parameters: {widgets: ["ListTallySheetVersions"]},
    play: async ({canvasElement}) => {
        await userEvent.click(action(await ballotBox(canvasElement, "PAPER"), "Versions"))
        const latest = await within(canvasElement).findByRole("row", {name: /^3 /})
        await userEvent.click(within(latest).getByLabelText("Approve").closest("button")!)
        const dialog = await within(document.body).findByRole("dialog")
        await userEvent.click(within(dialog).getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(graphql.calls).toEqual([])
    },
}

export const Empty: Story = {
    args: {sheets: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No Tally Sheet Yet.")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Generate Tally Sheet"}))
        expect(args.doAction).toHaveBeenCalledWith(WizardSteps.Start)
    },
}

export const WithoutViewPermission: Story = {
    args: {roles: []},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("No Tally Sheet Yet.")).toBeVisible()
        expect(canvas.queryByRole("button")).toBeNull()
        expect(canvas.queryByRole("row")).toBeNull()
    },
}
