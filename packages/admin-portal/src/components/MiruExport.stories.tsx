// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {MenuList} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {TallyStoryContext, tallyData} from "@/resources/Tally/__stories__/TallyFixture"
import {MiruExport} from "./MiruExport"

interface Scenario {
    electionId: string
    /** Whether ResultsDataLoader has loaded the tally's results. */
    loaded: boolean
    handleClose: () => void
    onCreateTransmissionPackage: (value: {area_id: string; election_id: string}) => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/MiruExport",
    component: MiruExport,
    args: {
        electionId: STORY_IDS.election,
        loaded: true,
        handleClose: fn(),
        onCreateTransmissionPackage: fn(),
    },
    argTypes: {
        electionId: {
            control: "select",
            options: [STORY_IDS.election, STORY_IDS.secondElection],
        },
        handleClose: {table: {disable: true}},
        onCreateTransmissionPackage: {table: {disable: true}},
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    // The items are entries of the results' export menu.
    render: ({electionId, loaded, handleClose, onCreateTransmissionPackage}) => (
        <AdminStoryProvider boundary={boundary}>
            <TallyStoryContext data={loaded ? tallyData() : null}>
                <MenuList aria-label="Export results">
                    <MiruExport
                        electionId={electionId}
                        handleClose={handleClose}
                        onCreateTransmissionPackage={onCreateTransmissionPackage}
                    />
                </MenuList>
            </TallyStoryContext>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const areaItem = (name: string) => i18n.t("tally.exportElectionArea", {name})

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const menu = within(within(canvasElement).getByRole("menu", {name: "Export results"}))
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
            areaItem("North district"),
            areaItem("South district"),
        ])
        await userEvent.click(menu.getByRole("menuitem", {name: areaItem("South district")}))
        expect(args.handleClose).toHaveBeenCalledTimes(1)
        expect(args.onCreateTransmissionPackage).toHaveBeenCalledTimes(1)
        expect(args.onCreateTransmissionPackage).toHaveBeenCalledWith({
            area_id: STORY_IDS.secondArea,
            election_id: STORY_IDS.election,
        })
    },
}

export const ElectionWithoutAreaResults: Story = {
    args: {electionId: STORY_IDS.secondElection},
    play: async ({canvasElement, args}) => {
        const menu = within(canvasElement).getByRole("menu", {name: "Export results"})
        expect(within(menu).queryAllByRole("menuitem")).toEqual([])
        expect(args.onCreateTransmissionPackage).not.toHaveBeenCalled()
    },
}

export const ResultsNotLoaded: Story = {
    args: {loaded: false},
    play: async ({canvasElement}) => {
        const menu = within(canvasElement).getByRole("menu", {name: "Export results"})
        expect(within(menu).queryAllByRole("menuitem")).toEqual([])
    },
}
