// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import type {Identifier, RaRecord} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, contestRecord} from "@/__stories__/fixtures"
import {ContestItem} from "./ContestItem"
import {QueryStateProbe, readStatuses} from "./__stories__/QueryStateFixture"
import {EStoryLocale, readStoryGlobals} from "../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** What reading the contest does. */
    reads: ReadState
    /** Whether the contest's presentation has any name or alias. */
    unnamed: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Components/ContestItem",
    component: ContestItem,
    args: {reads: "records", unnamed: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                sequent_backend_contest: [
                    contestRecord(args.unnamed ? {presentation: {i18n: {en: {}}}} : {}),
                ],
            },
            {reads: args.reads}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (_args, {globals}) => (
        <AdminStoryProvider
            key={JSON.stringify(globals)}
            boundary={graphql}
            dataProvider={data.provider}
        >
            <QueryStateProbe />
            {/* Tally sheet lists pass the contest ID, which ContestItem reads as its `record`. */}
            <ContestItem record={STORY_IDS.contest as unknown as RaRecord<Identifier>} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const readContest = () =>
    waitFor(() =>
        expect(data.calls).toEqual([
            {
                method: "getOne",
                args: ["sequent_backend_contest", expect.objectContaining({id: STORY_IDS.contest})],
            },
        ])
    )
const chips = (canvasElement: HTMLElement) => canvasElement.querySelectorAll(".MuiChip-root")

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const {locale} = readStoryGlobals(globals)
        // The alias in the toolbar language, else in English.
        const alias = locale === EStoryLocale.SPANISH ? "Miembros" : "Members"
        await expect(await within(canvasElement).findByText(alias)).toBeVisible()
        await readContest()
    },
}

export const WithoutName: Story = {
    args: {unnamed: true},
    play: async ({canvasElement}) => {
        const chip = await waitFor(() => {
            const only = chips(canvasElement)[0]
            expect(only).toBeDefined()
            return only
        })
        expect(chip).toHaveTextContent(/^-$/)
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await readContest()
        expect(readStatuses()).toEqual(["pending"])
        expect(chips(canvasElement)).toHaveLength(0)
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        await readContest()
        await waitFor(() => expect(readStatuses()).toEqual(["error"]))
        // The failure is silent: no chip and no message.
        expect(chips(canvasElement)).toHaveLength(0)
        expect(within(canvasElement).queryByText(/unavailable/)).not.toBeInTheDocument()
    },
}
