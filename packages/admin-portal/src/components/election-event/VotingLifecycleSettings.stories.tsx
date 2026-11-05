// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {SimpleForm} from "react-admin"
import {
    EInitializationScope,
    EUnsignedScheduledClosePolicy,
    i18n,
    type ILifecyclePolicies,
} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {runsUnsigned, refusedLooser} from "@/components/timezones/__fixtures__/explanations"
import {VotingLifecycleSettings} from "./VotingLifecycleSettings"

interface Scenario {
    /** The policies as saved now. */
    saved: ILifecyclePolicies
    /** The newest publication's policies; null when nothing is published. */
    published: ILifecyclePolicies | null
}

const DEFAULTS: ILifecyclePolicies = {
    initialization_scope: EInitializationScope.POST,
    unsigned_scheduled_close: EUnsignedScheduledClosePolicy.REFUSE,
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Election event/VotingLifecycleSettings",
    component: VotingLifecycleSettings,
    args: {saved: DEFAULTS, published: DEFAULTS},
    beforeEach: ({args}) => {
        // The lifecycle actions aren't in the generated schema yet: no schema check.
        boundary = graphqlBoundary({
            GetLifecycleSnapshots: () => ({
                data: {
                    get_lifecycle_snapshots: {
                        snapshots: args.published
                            ? [
                                  {
                                      election_id: null,
                                      publication_id: "99999999-9999-4999-8999-999999999990",
                                      published_at: "2028-03-01T00:00:00Z",
                                      approval_request_id: null,
                                      approval_code: null,
                                      signed: false,
                                      snapshot: {policies: args.published, schedule: []},
                                  },
                              ]
                            : [],
                    },
                },
            }),
            PreviewScheduledOutcomeChange: ({variables}) => {
                const policies = (variables.change as {policies: ILifecyclePolicies}).policies
                const loosens =
                    policies.unsigned_scheduled_close ===
                    EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM
                return {
                    data: {
                        preview_scheduled_outcome_change: {
                            applies: loosens ? "loosens" : "tightens",
                            applies_message_key: loosens
                                ? "scheduledOutcome.applies.loosens"
                                : "scheduledOutcome.applies.tightens",
                            changes: loosens
                                ? []
                                : [
                                      {
                                          scheduled_event_id: "close",
                                          election_id: null,
                                          before: runsUnsigned(),
                                          after: refusedLooser(),
                                      },
                                  ],
                        },
                    },
                }
            },
        })
    },
    render: ({saved}) => (
        <AdminStoryProvider boundary={boundary}>
            <SimpleForm
                record={{id: EVENT_ID, presentation: {lifecycle_policies: saved}}}
                onSubmit={fn()}
                toolbar={false}
            >
                <VotingLifecycleSettings electionEventId={EVENT_ID} saved={saved} />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const policies = (key: string, options?: Record<string, unknown>) =>
    i18n.t(`lifecycle.policies.${key}`, options)

/** The defaults, published as they are. */
export const PublishedDefaults: Story = {
    parameters: {widgets: ["PublishedValue"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvasElement.querySelector(`input[type="radio"][value="${EInitializationScope.POST}"]`)
        ).toBeChecked()
        const published = await canvas.findAllByText(
            policies("publishedValue", {value: policies("close.refuse.label")})
        )
        await expect(published[0]).toBeVisible()
        expect(canvas.queryByTestId("changed-since-published")).toBeNull()
        expect(canvas.queryByTestId("lifecycle-save-notice")).toBeNull()
    },
}

/** Nothing published: scheduled transitions use the defaults until the first publication. */
export const NothingPublished: Story = {
    args: {published: null},
    play: async ({canvasElement}) => {
        const texts = await within(canvasElement).findAllByText(policies("nothingPublished"))
        expect(texts).toHaveLength(2)
    },
}

/** Loosening the close: a warning, the note that it waits for the next approved publication. */
export const LoosenTheClose: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            canvas.getByRole("radio", {name: new RegExp(policies("close.runAsSystem.label"))})
        )
        await expect(canvas.getByText(policies("close.runAsSystem.warning"))).toBeVisible()
        await expect(await canvas.findByText(policies("changedSincePublished"))).toBeVisible()
        const notice = within(canvas.getByTestId("lifecycle-save-notice"))
        await waitFor(() =>
            expect(notice.getByText(policies("onSave.outcomes", {count: 0}))).toBeVisible()
        )
        await expect(notice.getByText(i18n.t("scheduledOutcome.applies.loosens"))).toBeVisible()
    },
}

/** The published close runs unsigned; refusing it tightens and applies now. */
export const TightenTheClose: Story = {
    args: {
        saved: {...DEFAULTS, unsigned_scheduled_close: EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM},
        published: {
            ...DEFAULTS,
            unsigned_scheduled_close: EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            canvas.getByRole("radio", {name: new RegExp(policies("close.refuse.label"))})
        )
        await expect(await canvas.findByText(policies("changedSincePublished"))).toBeVisible()
        const notice = within(canvas.getByTestId("lifecycle-save-notice"))
        await waitFor(() =>
            expect(notice.getByText(policies("onSave.outcomes", {count: 1}))).toBeVisible()
        )
    },
}

/** Whole event and per country: neither includes the other, so the change is mixed. */
export const ScopeChangeIsMixed: Story = {
    args: {
        saved: {...DEFAULTS, initialization_scope: EInitializationScope.EVENT},
        published: {...DEFAULTS, initialization_scope: EInitializationScope.EVENT},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            canvas.getByRole("radio", {name: new RegExp(policies("scope.postAndCountry.label"))})
        )
        await expect(canvas.getByText(policies("scope.postAndCountry.warning"))).toBeVisible()
        await expect(await canvas.findByText(policies("changedSincePublished"))).toBeVisible()
    },
}
