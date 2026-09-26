// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EEarlyVotingPolicy} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {
    STORY_IDS,
    areaRecords,
    contestRecord,
    eventRecord,
    storyId,
    type StoryRecord,
} from "@/__stories__/fixtures"
import type {Sequent_Backend_Area} from "@/gql/graphql"
import {pending} from "../../../../../ui-essentials/.storybook/screens"

/** What the area form's services do. */
export interface AreaFormServices {
    /** Upserting the area succeeds or the service fails. */
    upsert: "success" | "failure"
    /** Reading the area of the edit route. */
    area: "record" | "loading"
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

/** The contest the story's council area already takes part in. */
export const deputies = contestRecord({
    id: STORY_IDS.secondContest,
    presentation: {i18n: {en: {name: "Deputy council", alias: "Deputies"}}},
})

/** North district, with early voting allowed. */
export const northDistrict: StoryRecord<Sequent_Backend_Area> = {
    ...areaRecords()[0],
    presentation: {allow_early_voting: EEarlyVotingPolicy.ALLOW_EARLY_VOTING},
}

/** A second event, offered when the form does not come from an event. */
export const secondEvent = eventRecord(undefined, {
    id: storyId(2, 2),
    presentation: {i18n: {en: {name: "Referendum event", alias: "Referendum"}}},
})

export async function setUpAreaForm({upsert, area}: AreaFormServices) {
    data = resourceBoundary(
        {
            sequent_backend_area: [northDistrict, areaRecords()[1]],
            sequent_backend_contest: [contestRecord(), deputies],
            sequent_backend_election_event: [eventRecord(), secondEvent],
        },
        {reads: {sequent_backend_area: area === "loading" ? "loading" : "records"}}
    )
    graphql = graphqlBoundary(
        {
            UpsertArea: ({variables}) => {
                if (upsert === "failure") throw new Error("Synthetic area service unavailable")
                return {data: {upsert_area: {id: variables.id ?? storyId(7, 3)}}}
            },
            sequent_backend_area_extended: ({variables}) =>
                variables.areaId === STORY_IDS.area
                    ? {data: {sequent_backend_area_contest: [{contest: deputies}]}}
                    : pending(),
        },
        {schema: true}
    )
    await graphql.ready
}

export function AreaFormFixture({children}: PropsWithChildren) {
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            {children}
        </AdminStoryProvider>
    )
}

/** Upsert mutations the form sent. */
export const upserts = () => graphql.calls.filter(({name}) => name === "UpsertArea")
/** Reads of the area's contests, including the refetch after saving. */
export const contestReads = () =>
    graphql.calls.filter(({name}) => name === "sequent_backend_area_extended")
/** React-admin reads and writes of the story. */
export const dataCalls = () => data.calls
export const dataWrites = () => data.writes

/** Chooses an option of one of the form's autocomplete or select inputs. */
export async function choose(input: HTMLElement, option: string) {
    await userEvent.click(input)
    await userEvent.click(await within(document.body).findByRole("option", {name: option}))
    await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull())
}
