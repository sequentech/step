// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import type {FetchResult} from "@apollo/client"
import {GraphQLError} from "graphql"
import {expect, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords, contestRecord} from "@/__stories__/fixtures"
import {AreaContestItems} from "./AreaContestItems"
import {EStoryLocale, readStoryGlobals} from "../../../ui-essentials/.storybook/globals"
import {pending} from "../../../ui-essentials/.storybook/screens"

interface Scenario {
    /** What the area's contests query answers. */
    reply: "contests" | "none" | "loading" | "error"
}

let graphql: ReturnType<typeof graphqlBoundary>

const deputies = contestRecord({
    id: STORY_IDS.secondContest,
    presentation: {i18n: {en: {name: "Deputy council", alias: "Deputies"}}},
})

const meta = {
    title: "Admin/Components/AreaContestItems",
    component: AreaContestItems,
    args: {reply: "contests"},
    argTypes: {reply: {control: "inline-radio", options: ["contests", "none", "loading", "error"]}},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                get_area_with_area_contests: (): FetchResult | Promise<FetchResult> => {
                    if (args.reply === "loading") return pending()
                    if (args.reply === "error") {
                        return {errors: [new GraphQLError("Synthetic area contests unavailable")]}
                    }
                    return {
                        data: {
                            sequent_backend_area_contest:
                                args.reply === "none"
                                    ? []
                                    : [
                                          {id: STORY_IDS.areaContest, contest: contestRecord()},
                                          {
                                              id: "78787878-7878-4787-8787-787878787872",
                                              contest: deputies,
                                          },
                                      ],
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (_args, {globals}) => (
        <AdminStoryProvider key={JSON.stringify(globals)} boundary={graphql}>
            <AreaContestItems record={areaRecords()[0]} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const requested = () =>
    waitFor(() =>
        expect(graphql.calls).toEqual([
            {
                name: "get_area_with_area_contests",
                variables: {electionEventId: EVENT_ID, areaId: STORY_IDS.area},
                headers: {},
            },
        ])
    )

/** What the widget's query holds once its reply has arrived. */
const queryResults = () =>
    Array.from(graphql.client.getObservableQueries("active").values()).map((query) =>
        query.getCurrentResult()
    )
const chips = (canvasElement: HTMLElement) => canvasElement.querySelectorAll(".MuiChip-root")

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        const {locale} = readStoryGlobals(globals)
        // Each chip shows the contest alias in the toolbar language, else in English.
        await expect(
            await canvas.findByText(locale === EStoryLocale.SPANISH ? "Miembros" : "Members")
        ).toBeVisible()
        await expect(canvas.getByText("Deputies")).toBeVisible()
        await requested()
    },
}

export const Empty: Story = {
    args: {reply: "none"},
    play: async ({canvasElement}) => {
        await requested()
        await waitFor(() =>
            expect(queryResults().map(({data}) => data)).toEqual([
                {sequent_backend_area_contest: []},
            ])
        )
        expect(chips(canvasElement)).toHaveLength(0)
    },
}

export const Loading: Story = {
    args: {reply: "loading"},
    play: async ({canvasElement}) => {
        await requested()
        expect(queryResults().map(({loading}) => loading)).toEqual([true])
        expect(chips(canvasElement)).toHaveLength(0)
    },
}

export const LoadError: Story = {
    args: {reply: "error"},
    play: async ({canvasElement}) => {
        await requested()
        await waitFor(() =>
            expect(queryResults().map(({error}) => error?.message)).toEqual([
                "Synthetic area contests unavailable",
            ])
        )
        // The failure is silent: the cell shows neither chips nor a message.
        expect(chips(canvasElement)).toHaveLength(0)
        expect(within(canvasElement).queryByText(/unavailable/)).not.toBeInTheDocument()
    },
}
