// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react"
import {initCore} from "@sequentech/ui-core"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {
    RecordContextProvider,
    ResourceContextProvider,
    SaveContextProvider,
    type RaRecord,
} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {AuthContext} from "@/providers/AuthContextProvider"
import {ContestDataForm} from "./EditContestDataForm"
import {ElectionDataForm} from "../Election/ElectionDataForm"

const ELECTION_ID = "33333333-3333-4333-8333-333333333333"
const CONTEST_ID = "44444444-4444-4444-8444-444444444444"
const language = {enabled_language_codes: ["en", "es"], default_language_code: "en"}
const event = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    presentation: {language_conf: language, contest_encryption_policy: "multiple-contests"},
}
const election = {
    id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    status: {allow_tally: "allowed"},
    voting_channels: ["ONLINE"],
    presentation: {
        language_conf: language,
        i18n: {en: {name: "Council election"}, es: {name: "Elección municipal"}},
        contests_order: "alphabetical",
    },
}
const contest = {
    id: CONTEST_ID,
    election_id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    min_votes: 0,
    max_votes: 2,
    winning_candidates_num: 1,
    counting_algorithm: "plurality-at-large",
    presentation: {
        columns: 1,
        i18n: {en: {name: "Council members"}, es: {name: "Miembros del consejo"}},
    },
}
interface Scenario {
    kind: "contest" | "election"
    canEdit: boolean
    preferential: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
const save = fn(async (_values: Record<string, unknown>) => undefined)
function Fixture({kind, canEdit, preferential}: Scenario) {
    const auth = useContext(AuthContext)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{
                    ...auth,
                    tenantId: TENANT_ID,
                    isAuthorized: (_super, _tenant, permission) =>
                        canEdit && ["election-write", "contest-write"].includes(String(permission)),
                }}
            >
                <ResourceContextProvider value={`sequent_backend_${kind}`}>
                    <SaveContextProvider value={{save, saving: false, mutationMode: "pessimistic"}}>
                        <RecordContextProvider
                            value={
                                kind === "contest"
                                    ? {
                                          ...contest,
                                          counting_algorithm: preferential
                                              ? "instant-runoff"
                                              : "plurality-at-large",
                                      }
                                    : election
                            }
                        >
                            {kind === "contest" ? <ContestDataForm /> : <ElectionDataForm />}
                        </RecordContextProvider>
                    </SaveContextProvider>
                </ResourceContextProvider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Policy forms",
    component: Fixture,
    args: {kind: "contest", canEdit: true, preferential: false},
    beforeEach: async () => {
        await initCore()
        save.mockClear()
        boundary = graphqlBoundary({})
        data = dataBoundary({
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                const records: Record<string, RaRecord> = {
                    sequent_backend_election_event: event,
                    sequent_backend_election: election,
                    sequent_backend_tenant: {id: TENANT_ID, settings: {languages: ["en", "es"]}},
                    sequent_backend_document: {id: TENANT_ID, name: null},
                }
                const record = records[resource]
                if (!record || record.id !== id) {
                    data.unexpected.push(`${resource}/${id}`)
                    throw new Error("Unexpected record")
                }
                return {data: record as RecordType}
            },
            getList: async <RecordType extends RaRecord>(resource: string) => {
                if (!["sequent_backend_contest", "sequent_backend_candidate"].includes(resource)) {
                    data.unexpected.push(resource)
                    throw new Error("Unexpected list")
                }
                return {data: [] as RecordType[], total: 0}
            },
        })
        const services = boundary
        const records = data
        return () => {
            expect(services.unexpected).toEqual([])
            expect(records.unexpected).toEqual([])
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>
async function choose(canvasElement: HTMLElement, label: string | RegExp, value: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: label}))
    await userEvent.click(await within(document.body).findByRole("option", {name: value}))
}
export const ContestPoliciesSaveWireValues: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Ballot Design"}))
        await waitFor(() => expect(canvas.getByDisplayValue("Council members")).not.toBeVisible())
        await choose(canvasElement, "Under Vote Policy", "Warn and Alert")
        await choose(canvasElement, "Invalid Vote Policy", "Not Allowed")
        await choose(canvasElement, "Blank Vote Policy", "Not Allowed")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toEqual(
            expect.objectContaining({
                id: CONTEST_ID,
                tenant_id: TENANT_ID,
                election_id: ELECTION_ID,
                presentation: expect.objectContaining({
                    under_vote_policy: "warn-and-alert",
                    invalid_vote_policy: "not-allowed",
                    blank_vote_policy: "not-allowed",
                }),
            })
        )
        expect(boundary.calls).toEqual([])
    },
}
export const ContestLanguageTabs: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByDisplayValue("Council members")).toBeVisible()
        await userEvent.click(canvas.getByRole("tab", {name: "Spanish"}))
        await expect(await canvas.findByDisplayValue("Miembros del consejo")).toBeVisible()
        expect(save).not.toHaveBeenCalled()
    },
}
export const ContestReadOnlyHidesSave: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByDisplayValue("Council members")
        expect(canvas.queryByRole("button", {name: "Save"})).not.toBeInTheDocument()
        expect(save).not.toHaveBeenCalled()
    },
}
export const ElectionPolicySave: Story = {
    args: {kind: "election"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByDisplayValue("Council election")
        await userEvent.click(canvas.getByRole("button", {name: "Advanced Configuration"}))
        await waitFor(() => expect(canvas.getByDisplayValue("Council election")).not.toBeVisible())
        await choose(canvasElement, "Allow Tally", "Requires Voting Period End")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toEqual(
            expect.objectContaining({
                id: ELECTION_ID,
                status: {allow_tally: "requires-voting-period-end"},
            })
        )
    },
}

async function openPolicies(canvasElement: HTMLElement, kind: "contest" | "election") {
    const canvas = within(canvasElement)
    const name = await canvas.findByDisplayValue(
        kind === "contest" ? "Council members" : "Council election"
    )
    // Keep a real edited field so saving a policy's original/default value also
    // exercises submission instead of clicking a pristine disabled Save button.
    await userEvent.type(name, " (policy check)")
    await userEvent.click(canvas.getByRole("button", {name: "Ballot Design"}))
    await waitFor(() => expect(name).not.toBeVisible())
    return canvas
}

function contestPolicyChoices(
    label: string | RegExp,
    field: string,
    choices: ReadonlyArray<readonly [string, string]>
): Story {
    return {
        play: async ({canvasElement}) => {
            const canvas = await openPolicies(canvasElement, "contest")
            for (const [option, wireValue] of choices) {
                save.mockClear()
                await choose(canvasElement, label, option)
                await userEvent.click(canvas.getByRole("button", {name: "Save"}))
                await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
                expect(save.mock.calls[0][0]).toMatchObject({
                    id: CONTEST_ID,
                    tenant_id: TENANT_ID,
                    election_id: ELECTION_ID,
                    presentation: {[field]: wireValue},
                })
            }
        },
    }
}

export const UnderVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Under Vote Policy",
    "under_vote_policy",
    [
        ["Warn", "warn"],
        ["Warn in Review", "warn-only-in-review"],
        ["Warn and Alert", "warn-and-alert"],
        ["Allowed", "allowed"],
    ]
)

export const InvalidVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Invalid Vote Policy",
    "invalid_vote_policy",
    [
        ["Not Allowed", "not-allowed"],
        ["Warn", "warn"],
        ["Warn Invalid Implicit And Explicit", "warn-invalid-implicit-and-explicit"],
        ["Allowed With Exclusive Explicit", "allowed-with-exclusive-explicit"],
        ["Allowed", "allowed"],
    ]
)

export const BlankVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Blank Vote Policy",
    "blank_vote_policy",
    [
        ["Not Allowed", "not-allowed"],
        ["Warn", "warn"],
        ["Warn in Review", "warn-only-in-review"],
        ["Allowed", "allowed"],
    ]
)

export const OverVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Over Vote Policy",
    "over_vote_policy",
    [
        ["Allowed with Warning Message", "allowed-with-msg"],
        ["Allowed with Warning message and Alert", "allowed-with-msg-and-alert"],
        ["Not Allowed with Warning message and Alert", "not-allowed-with-msg-and-alert"],
        [
            "Not Allowed with Warning message and Disable further selections",
            "not-allowed-with-msg-and-disable",
        ],
        ["Allowed", "allowed"],
    ]
)

export const PreferentialRankPoliciesSaveBothChoices: Story = {
    args: {preferential: true},
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "contest")
        for (const [option, wireValue] of [
            [
                "Show Warning and Dialog (voter not allowed to proceed)",
                "not-allowed-warn-and-dialog",
            ],
            ["Show Warning and Dialog (voter can proceed)", "allowed-warn-and-dialog"],
        ]) {
            save.mockClear()
            await choose(canvasElement, "Invalid Vote - Duplicate Rank Policy", option)
            await choose(canvasElement, "Invalid Vote - Skipped Ranks Policy", option)
            await userEvent.click(canvas.getByRole("button", {name: "Save"}))
            await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
            expect(save.mock.calls[0][0]).toMatchObject({
                counting_algorithm: "instant-runoff",
                presentation: {
                    duplicated_rank_policy: wireValue,
                    preference_gaps_policy: wireValue,
                },
            })
        }
    },
}

export const AuditPoliciesSaveEveryChoice: Story = {
    args: {kind: "election"},
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "election")
        for (const [option, wireValue] of [
            ["Not Show", "not-show"],
            ["Show In Help Dialog", "show-in-help"],
            ["Show", "show"],
        ]) {
            save.mockClear()
            await choose(canvasElement, "Audit Button Display Options", option)
            await userEvent.click(canvas.getByRole("button", {name: "Save"}))
            await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
            expect(save.mock.calls[0][0]).toMatchObject({
                id: ELECTION_ID,
                tenant_id: TENANT_ID,
                presentation: {audit_button_cfg: wireValue},
            })
        }
    },
}

export const CheckableListsSaveEveryChoice = contestPolicyChoices(
    /checkable lists/i,
    "enable_checkable_lists",
    [
        ["Lists Only", "allow-selecting-lists"],
        ["Candidates Only", "allow-selecting-candidates"],
        ["Disabled", "disabled"],
        ["Candidates And Lists", "allow-selecting-candidates-and-lists"],
    ]
)

export const CollapsibleListsSaveEveryChoice = contestPolicyChoices(
    "Collapsible Lists",
    "collapsible_lists",
    [
        ["Enabled (starts collapsed)", "enabled-collapsed"],
        ["Enabled (starts expanded)", "enabled-expanded"],
        ["Disabled", "disabled"],
    ]
)

export const CheckboxShapeSavesBothChoices = contestPolicyChoices(
    "Candidates checkbox icon shape",
    "candidates_icon_checkbox_policy",
    [
        ["Round Checkbox", "round-checkbox"],
        ["Square Checkbox", "square-checkbox"],
    ]
)

export const ContestSelectionAndDisplaySettingsSave: Story = {
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "contest")
        expect(
            canvas.queryByRole("combobox", {name: "Invalid Vote - Duplicate Rank Policy"})
        ).not.toBeInTheDocument()
        await userEvent.click(canvas.getByRole("switch", {name: "Allow Write-Ins"}))
        await userEvent.click(canvas.getByRole("switch", {name: "Decided by acclamation"}))
        for (const [label, value] of [
            [/min votes/i, "1"],
            [/max votes/i, "6"],
            [/columns/i, "2"],
            [/winning candidates num/i, "2"],
            [/max selections per type/i, "1"],
        ] as const) {
            const input = canvas.getByRole("spinbutton", {name: label})
            await userEvent.clear(input)
            await userEvent.type(input, value)
        }
        await userEvent.type(canvas.getByRole("textbox", {name: "Page Name"}), "Council page")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toMatchObject({
            is_acclaimed: true,
            min_votes: 1,
            max_votes: 6,
            winning_candidates_num: 2,
            presentation: {
                allow_writeins: true,
                columns: 2,
                max_selections_per_type: 1,
                pagination_policy: "Council page",
            },
        })
    },
}

export const ElectionAdvancedPoliciesSaveAndRestore: Story = {
    args: {kind: "election"},
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "election")
        await userEvent.click(canvas.getByRole("button", {name: "Advanced Configuration"}))
        await userEvent.click(canvas.getByRole("switch", {name: "Cast Vote Confirmation Modal"}))
        const allowedVotes = canvas.getByRole("spinbutton", {name: "Number of allowed votes"})
        await userEvent.clear(allowedVotes)
        await userEvent.type(allowedVotes, "3")
        for (const [label, option] of [
            ["Gold level Authentication Policy", "Gold level Authentication"],
            ["Start Screen Title Policy", "Election event title"],
            ["Security Confirmation Checkbox Policy", "Mandatory"],
            ["Grace Period Policy", "Grace period without alert"],
            ["Voting Screen Back Button Policy", "Go to the election start screen"],
            ["Blank Ballots Policy", "Enabled"],
            ["Consolidated Report Policy", "Generate"],
            ["Initialize Report Policy", "Required"],
            ["Allow Tally", "Disallowed"],
        ])
            await choose(canvasElement, label, option)
        const grace = canvas.getByRole("spinbutton", {name: "Grace period in seconds"})
        await expect(grace).toBeEnabled()
        await userEvent.clear(grace)
        await userEvent.type(grace, "45")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toMatchObject({
            num_allowed_revotes: 3,
            status: {allow_tally: "disallowed"},
            presentation: {
                cast_vote_confirm: true,
                cast_vote_gold_level: "gold-level",
                start_screen_title_policy: "election-event",
                security_confirmation_policy: "mandatory",
                grace_period_policy: "grace-period-without-alert",
                grace_period_secs: 45,
                voting_screen_back_policy: "start-screen",
                blank_ballots_policy: "enabled",
                consolidated_report_policy: "generate",
                initialization_report_policy: "required",
            },
        })
        save.mockClear()
        for (const [label, option] of [
            ["Gold level Authentication Policy", "No Gold level Authentication"],
            ["Start Screen Title Policy", "Election title"],
            ["Security Confirmation Checkbox Policy", "None"],
            ["Grace Period Policy", "No grace period"],
            ["Voting Screen Back Button Policy", "Go to the election selection screen"],
            ["Blank Ballots Policy", "Disabled"],
            ["Consolidated Report Policy", "Do Not Generate"],
            ["Initialize Report Policy", "Not Required"],
            ["Allow Tally", "Allowed"],
        ])
            await choose(canvasElement, label, option)
        await expect(grace).toBeDisabled()
        await userEvent.click(canvas.getByRole("switch", {name: "Cast Vote Confirmation Modal"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toMatchObject({
            status: {allow_tally: "allowed"},
            presentation: {
                cast_vote_confirm: false,
                cast_vote_gold_level: "no-gold-level",
                start_screen_title_policy: "election",
                security_confirmation_policy: "none",
                grace_period_policy: "no-grace-period",
                voting_screen_back_policy: "election-selection-screen",
                blank_ballots_policy: "disabled",
                consolidated_report_policy: "do-not-generate",
                initialization_report_policy: "not-required",
            },
        })
    },
}
