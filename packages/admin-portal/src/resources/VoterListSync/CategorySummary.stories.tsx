// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {CategorySummary} from "./CategorySummary"
import {HIGHLIGHTED_CATEGORIES} from "./constants"
import {ESyncChangeCategory} from "./types"

const category = (value: ESyncChangeCategory) => i18n.t(`reconciliation.categories.${value}`)

/** The box of one category: its caption and count. */
const box = (canvasElement: HTMLElement, value: ESyncChangeCategory) =>
    within(canvasElement).getByText(category(value)).parentElement as HTMLElement

const meta = {
    title: "Admin/Voter list sync/CategorySummary",
    component: CategorySummary,
    args: {
        counts: {
            [ESyncChangeCategory.VOTED_OTHER_CHANNEL]: 3,
            [ESyncChangeCategory.PROFILE_UPDATE]: 12,
            [ESyncChangeCategory.VOTER_ADDED]: 0,
        },
    },
} satisfies Meta<typeof CategorySummary>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            within(box(canvasElement, ESyncChangeCategory.VOTED_OTHER_CHANNEL)).getByText("3")
        ).toBeVisible()
        await expect(
            within(box(canvasElement, ESyncChangeCategory.PROFILE_UPDATE)).getByText("12")
        ).toBeVisible()
        // Categories without changes are left out.
        expect(canvas.queryByText(category(ESyncChangeCategory.VOTER_ADDED))).toBeNull()
    },
}

export const HighlightsSensitiveCategories: Story = {
    args: {highlighted: HIGHLIGHTED_CATEGORIES},
    play: async ({canvasElement}) => {
        const sensitive = getComputedStyle(
            box(canvasElement, ESyncChangeCategory.VOTED_OTHER_CHANNEL)
        )
        const plain = getComputedStyle(box(canvasElement, ESyncChangeCategory.PROFILE_UPDATE))
        expect(sensitive.borderTopColor).not.toBe(plain.borderTopColor)
    },
}

export const NoChanges: Story = {
    args: {counts: {}},
    play: async ({canvasElement}) => {
        for (const value of Object.values(ESyncChangeCategory)) {
            expect(within(canvasElement).queryByText(category(value))).toBeNull()
        }
    },
}
