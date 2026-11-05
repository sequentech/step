// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MonitoringDataTableDialog} from "./MonitoringDataTableDialog"
import {turnoutSummary, turnoutSummaryTables, turnoutTable} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringDataTableDialog",
    component: MonitoringDataTableDialog,
    args: {
        open: true,
        onClose: fn(),
        title: "Turnout by group",
        scope: "North · All authorized Posts · All countries",
        table: turnoutTable,
        notices: ["UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY", "SOMETHING_NEW"],
    },
} satisfies Meta<typeof MonitoringDataTableDialog>
export default meta
type Story = StoryObj<typeof meta>

/** The open dialog, once its fade-in has finished. */
const dialog = async () => {
    const element = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

export const ViewData: Story = {
    play: async ({args}) => {
        const body = await dialog()
        await expect(body.getByText("Turnout by group · data")).toBeVisible()
        await expect(body.getByText("North · All authorized Posts · All countries")).toBeVisible()
        await expect(
            body.getByText(
                "Attempts by unregistered usernames belong to no Post, so they are counted for the whole event only."
            )
        ).toBeVisible()
        // A notice this build does not know is shown as words, never as its code.
        await expect(body.getByText("Something new")).toBeVisible()
        await expect(body.getByRole("table", {name: "Turnout by group · data"})).toBeVisible()
        await userEvent.click(body.getByRole("button", {name: "Close"}))
        await expect(args.onClose).toHaveBeenCalled()
    },
}

/** An older backend sends only the first query's rows: one table, headed in the viewer's words. */
export const OlderBackendOneTable: Story = {
    play: async () => {
        const body = await dialog()
        const table = body.getByRole("table", {name: "Turnout by group · data"})
        const headers = within(table)
            .getAllByRole("columnheader")
            .map((header) => header.textContent)
        expect(headers).toEqual(["Group", "Voted", "Pre-enrolled", "Percentage"])
        expect(body.queryByRole("heading", {level: 3})).toBeNull()
    },
}

/** Every query a widget reads, in widget order, each under its name. */
export const EveryQuery: Story = {
    args: {
        title: "Voter turnout",
        table: turnoutSummaryTables[0].table,
        tables: turnoutSummaryTables,
        queries: turnoutSummary.queries,
        notices: [],
    },
    play: async () => {
        const body = await dialog()
        expect(
            body.getAllByRole("heading", {level: 3}).map((heading) => heading.textContent)
        ).toEqual(["totals", "voted_reg", "voted_pre"])
        const headers = (name: string) =>
            within(body.getByRole("table", {name}))
                .getAllByRole("columnheader")
                .map((header) => header.textContent)
        const cells = (name: string) =>
            within(body.getByRole("table", {name}))
                .getAllByRole("cell")
                .map((cell) => cell.textContent)
        expect(headers("Voter turnout · data · totals")).toEqual([
            "Registered",
            "Pre-enrolled",
            "Voted",
        ])
        // A ratio's parts are named by their measures.
        expect(headers("Voter turnout · data · voted_reg")).toEqual([
            "Voted",
            "Registered",
            "Percentage",
            "Percentage as shown",
        ])
        expect(cells("Voter turnout · data · voted_reg")).toEqual(["3", "8", "37.5%", "37.5%"])
        // Nobody pre-enrolled: no ratio, the same dash the card shows.
        expect(headers("Voter turnout · data · voted_pre")[1]).toBe("Pre-enrolled")
        expect(cells("Voter turnout · data · voted_pre")).toEqual(["3", "0", "—", "—"])
        await expect(body.getByRole("region", {name: "voted_reg"})).toBeVisible()
    },
}
