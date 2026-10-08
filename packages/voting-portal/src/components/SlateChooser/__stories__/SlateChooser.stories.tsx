// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {EMobileCandidateLists, IContest, ISlatesConfig} from "@sequentech/ui-core"

import {resolveSlates} from "../../../services/Slates"
import {SlateChooser} from "../SlateChooser"

const contest = (id: string, name: string, candidates: Array<[string, string]>): IContest =>
    ({
        id,
        name,
        candidates: candidates.map(([candidateId, candidateName]) => ({
            id: candidateId,
            contest_id: id,
            name: candidateName,
        })),
    }) as unknown as IContest

const CONTESTS: Array<IContest> = [
    contest("president", "President", [
        ["f-president", "Jordan Ellis"],
        ["m-president", "Morgan Hayes"],
        ["i-president", "Avery Brooks"],
    ]),
    contest("vp", "Vice President", [
        ["f-vp", "Taylor Morgan"],
        ["m-vp", "Casey Rivera"],
    ]),
    contest("trustees", "Trustees", [
        ["f-t1", "Rowan Scott"],
        ["f-t2", "Charlie Kim"],
        ["f-t3", "Dakota Reed"],
        ["m-t1", "Skyler James"],
        ["m-t2", "Finley Ross"],
        ["v-t1", "Harper Lane"],
        ["v-t2", "Sage Murphy"],
        ["i-t1", "Blair Lewis"],
    ]),
]

const CONFIG: ISlatesConfig = {
    version: 1,
    mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
    slates: [
        {
            id: "forward",
            name: {en: "Forward Together", es: "Adelante Juntos"},
            members: {
                president: ["f-president"],
                vp: ["f-vp"],
                trustees: ["f-t1", "f-t2", "f-t3"],
            },
        },
        {
            id: "members",
            name: {en: "Members First: a voice for every member of the association"},
            members: {president: ["m-president"], vp: ["m-vp"], trustees: ["m-t1", "m-t2"]},
        },
        {
            id: "voices",
            name: {en: "Independent Voices"},
            members: {trustees: ["v-t1", "v-t2"]},
        },
    ],
}

const meta = {
    title: "Voting Portal/Ballot/SlateChooser",
    component: SlateChooser,
    args: {slates: resolveSlates(CONFIG, CONTESTS), defaultLanguage: "en"},
} satisfies Meta<typeof SlateChooser>

export default meta
type Story = StoryObj<typeof meta>

export const NamedSlates: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const names = canvas.getAllByRole("heading", {level: 3}).map((h) => h.textContent)
        await expect(names).toEqual([
            "Forward Together",
            "Members First: a voice for every member of the association",
            "Independent Voices",
        ])
        await expect(
            canvas.getByRole("list", {name: "Independent Voices candidates for Trustees"})
        ).toBeVisible()
        await expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth)
    },
}

export const Phone: Story = {
    ...NamedSlates,
    globals: {viewport: {value: "mobile1"}},
}
