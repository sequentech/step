// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {FIXED_TIME, STORY_IDS, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Candidate} from "@/gql/graphql"
import {CreateCandidateData} from "./CreateCandidateData"
import {
    CandidateLayout,
    dataWrites,
    setUpCandidates,
    type CandidateServices,
} from "./__stories__/CandidateFixture"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"

const CAROL_ID = storyId(6, 3)

/** The candidate being created in the council contest. */
const draft: Sequent_Backend_Candidate = {
    id: CAROL_ID,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    contest_id: STORY_IDS.contest,
    type: "candidate",
    is_public: true,
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
    annotations: {},
    labels: {},
    presentation: {i18n: {en: {name: "Carol Example"}}},
}

const meta = {
    title: "Admin/Candidate/CreateCandidateData",
    component: CreateCandidateData,
    args: {reads: "records", empty: false, withImage: false},
    parameters: {
        router: {
            path: "/sequent_backend_candidate/create",
            initialEntries: ["/sequent_backend_candidate/create"],
            layout: CandidateLayout,
        },
    },
    beforeEach: ({args}) => setUpCandidates(args),
    render: () => <CreateCandidateData record={draft} />,
} satisfies WidgetMeta<CandidateServices>
export default meta
type Story = StoryObj<CandidateServices>

const englishName = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: "Name"})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(async () =>
            expect(await englishName(canvasElement)).toHaveValue("Carol Example")
        )
        await expect(canvas.getByRole("button", {name: "Save"})).toBeVisible()
        expect(dataWrites()).toEqual([])
    },
}

export const WithoutCandidateWrite: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        await waitFor(async () =>
            expect(await englishName(canvasElement)).toHaveValue("Carol Example")
        )
        expect(within(canvasElement).queryByRole("button", {name: "Save"})).toBeNull()
    },
}

export const CreateTheCandidate: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const name = await englishName(canvasElement)
        await userEvent.type(canvas.getByRole("textbox", {name: "Alias"}), "Carol")
        await waitFor(() => expect(name).toHaveValue("Carol Example"))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "create",
            resource: "sequent_backend_candidate",
            params: expect.objectContaining({
                data: expect.objectContaining({
                    id: CAROL_ID,
                    contest_id: STORY_IDS.contest,
                    presentation: expect.objectContaining({
                        i18n: {
                            en: expect.objectContaining({name: "Carol Example", alias: "Carol"}),
                        },
                        language_conf: {enabled_language_codes: []},
                    }),
                }),
            }),
        })
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Current location"})).toHaveTextContent(
                `/sequent_backend_candidate/${CAROL_ID}/show`
            )
        )
    },
}
