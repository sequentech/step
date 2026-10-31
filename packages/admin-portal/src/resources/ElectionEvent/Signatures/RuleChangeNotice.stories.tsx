// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {refusedEdited, runsAuthorized} from "@/components/timezones/__fixtures__/explanations"
import {RuleChangeNotice} from "./RuleChangeNotice"

interface Scenario {
    /** How the server classifies the rule edit. */
    applies: "tightens" | "loosens" | "tightens-and-loosens"
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Election event/Signatures/RuleChangeNotice",
    component: RuleChangeNotice,
    args: {applies: "tightens"},
    beforeEach: () => {
        graphql = graphqlBoundary({})
    },
    render: ({applies}) => (
        <AdminStoryProvider boundary={graphql}>
            <RuleChangeNotice
                preview={{
                    applies,
                    applies_message_key: null,
                    changes:
                        applies === "tightens"
                            ? [
                                  {
                                      scheduled_event_id: "opening",
                                      election_id: null,
                                      before: runsAuthorized(),
                                      after: refusedEdited(),
                                  },
                              ]
                            : [],
                }}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const notice = async (canvasElement: HTMLElement) =>
    within(await within(canvasElement).findByTestId("rule-change-notice"))

/** Requiring signatures applies now; one scheduled opening changes outcome. */
export const Tightens: Story = {
    play: async ({canvasElement}) => {
        const view = await notice(canvasElement)
        await expect(view.getByText(i18n.t("scheduledOutcome.applies.tightens"))).toBeVisible()
        await expect(
            view.getByText(i18n.t("lifecycle.policies.onSave.outcomes", {count: 1}))
        ).toBeVisible()
    },
}

/** Dropping signatures waits for the next approved publication. */
export const Loosens: Story = {
    args: {applies: "loosens"},
    play: async ({canvasElement}) => {
        const view = await notice(canvasElement)
        await expect(view.getByText(i18n.t("scheduledOutcome.applies.loosens"))).toBeVisible()
    },
}

/** Stricter in one value and looser in another. */
export const TightensAndLoosens: Story = {
    args: {applies: "tightens-and-loosens"},
    play: async ({canvasElement}) => {
        const view = await notice(canvasElement)
        await expect(
            view.getByText(i18n.t("scheduledOutcome.applies.tightensAndLoosens"))
        ).toBeVisible()
    },
}
