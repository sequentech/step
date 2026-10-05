// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {GraphQLError} from "graphql"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ViewApproval} from "./ViewApproval"
import {CAROL_VOTER_ID} from "./__stories__/ApprovalsFixture"
import {
    APPLICATION_ID,
    ApprovalsScreen,
    MANUAL_APPLICATION_ID,
    REJECTED_APPLICATION_ID,
    SECOND_APPLICATION_ID,
    graphqlCalls,
    listFilters,
    reads,
    setUpApprovals,
    type ApprovalServices,
} from "./__stories__/ApprovalsScreenFixture"

interface Scenario extends ApprovalServices {
    /** The enrollment the administrator opened. */
    applicationId: string
    /** What approving or rejecting answers. */
    decides: "done" | "already-enrolled" | "error"
    goBack: Mock<() => void>
    onViewRule: Mock<(decided: {version: number; rule: number | null}) => void>
}

const meta = {
    title: "Admin/Approvals/ViewApproval",
    component: ViewApproval,
    args: {
        reads: "records",
        registry: "records",
        empty: false,
        applicationId: APPLICATION_ID,
        decides: "done",
        goBack: fn(),
        onViewRule: fn(),
    },
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        registry: {control: "inline-radio", options: ["records", "loading", "error"]},
        decides: {control: "inline-radio", options: ["done", "already-enrolled", "error"]},
    },
    beforeEach: ({args}) =>
        setUpApprovals(args, {
            ChangeApplicationStatus: () =>
                args.decides === "error"
                    ? {errors: [new GraphQLError("Synthetic application status failure")]}
                    : {
                          data: {
                              ApplicationChangeStatus: {
                                  message: args.decides === "done" ? "Success" : null,
                                  error: args.decides === "done" ? null : "Approved_Voter",
                                  signing_request: null,
                              },
                          },
                      },
        }),
    render: ({applicationId, goBack, onViewRule}) => (
        <ApprovalsScreen>
            <ViewApproval
                electionEventId={EVENT_ID}
                currApprovalId={applicationId}
                goBack={goBack}
                onViewRule={onViewRule}
                electionEventRecord={eventRecord() as Sequent_Backend_Election_Event}
            />
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const SEARCH = "Not in the list? Search the registry by name or email"

/** The enrollment once it has loaded. */
const opened = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("heading", {name, level: 2})

const continueOn = (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: "Continue"}))

/** The step the review is on. */
const currentStep = (canvasElement: HTMLElement) =>
    within(within(canvasElement).getByRole("list", {name: "Review steps"}))
        .getAllByRole("listitem")
        .find((step) => step.getAttribute("aria-current") === "step")?.textContent

const voters = async (canvasElement: HTMLElement) =>
    within(await within(canvasElement).findByRole("radiogroup", {name: "Voters in the registry"}))

const decision = async (canvasElement: HTMLElement) =>
    within(await within(canvasElement).findByRole("radiogroup", {name: "Decide"}))

const dialog = async (name: string) => {
    const element = await within(document.body).findByRole("dialog", {name})
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

const notified = async (message: string) => {
    const notification = await within(document.body).findByText(message)
    await waitFor(() => expect(notification).toBeVisible())
}

const statusChanges = () => graphqlCalls("ChangeApplicationStatus").map((call) => call.variables)

export const OneDetailDiffers: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await opened(canvasElement, "Alice Example")).toBeVisible()
        await expect(canvas.getByText(/^alice@example\.test · Applied Jan 15, 2026/)).toBeVisible()
        await expect(canvas.getByText(/ · Waiting \d+ (day|days)$/)).toBeVisible()
        await expect(canvas.getByText("Needs review")).toBeVisible()
        expect(reads("getOne", "sequent_backend_applications")[0].args[1]).toMatchObject({
            id: APPLICATION_ID,
        })

        // Why the rules left it to a person, with the values of the closest registry voter.
        await expect(canvas.getByText("Why this needs a person")).toBeVisible()
        await expect(
            await canvas.findByText(
                "One detail doesn't match the registry: the date of birth is “1990-05-17” on the " +
                    "enrollment and “1990-05-07” in the registry. The approval rules ask a person " +
                    "to check this enrollment."
            )
        ).toBeVisible()
        await expect(canvas.getByText("Rule 5 of matrix version 1")).toBeVisible()
        await expect(canvas.getByText("Exactly 1 detail differs")).toBeVisible()
        await expect(canvas.getByText("Embassy matches")).toBeVisible()

        // Step 1: how the identity was established, and what the applicant wrote.
        expect(currentStep(canvasElement)).toBe("1Check the identity")
        await expect(canvas.getByText("The enrollment flow verified the voter's ID")).toBeVisible()
        await expect(canvas.getByRole("row", {name: /Date of birth May 17, 1990/})).toBeVisible()
        await expect(canvas.getByRole("row", {name: /Embassy Madrid/})).toBeVisible()
        await expect(
            canvas.getByRole("row", {name: new RegExp(`Application ID ${APPLICATION_ID}`)})
        ).toBeVisible()
        await continueOn(canvasElement)

        // Step 2: the registry voters closest to the enrollment.
        expect(currentStep(canvasElement)).toBe("2Find the voter")
        const registry = await voters(canvasElement)
        const alice = await registry.findByRole("radio", {
            name: /^Alice Example.*May 7, 1990 · Madrid.*Best match · 3 of 4 details match$/,
        })
        expect(registry.getAllByRole("radio")).toHaveLength(2)
        await expect(
            canvas.getByText(
                "We looked for voters with the same first name, last name, date of birth and " +
                    "embassy. Choose the one this enrollment belongs to."
            )
        ).toBeVisible()
        await expect(
            canvas.getByRole("heading", {name: "Compared with Alice Example in the registry"})
        ).toBeVisible()
        await expect(
            canvas.getByRole("row", {name: "Date of birth May 17, 1990 May 7, 1990 Different"})
        ).toBeVisible()
        await expect(canvas.getByRole("row", {name: "Embassy Madrid Madrid Same"})).toBeVisible()
        // One lookup with every compared detail, and one leaving each of them out.
        expect(listFilters("user")).toHaveLength(5)
        expect(listFilters("user")[0]).toEqual({
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            election_id: undefined,
            first_name: {IsLike: "Alice"},
            last_name: {IsLike: "Example"},
            attributes: {dateOfBirth: "1990-05-17", embassy: "Madrid"},
        })
        expect(listFilters("user")[3]).toMatchObject({attributes: {embassy: "Madrid"}})
        expect(listFilters("user")[3].attributes).not.toHaveProperty("dateOfBirth")
        await userEvent.click(alice)
        await continueOn(canvasElement)

        // Step 3: approving links the enrollment to the chosen voter.
        expect(currentStep(canvasElement)).toBe("3Decide")
        const verdict = await decision(canvasElement)
        await expect(verdict.getByRole("radio", {name: /^Approve/})).toBeChecked()
        await expect(
            verdict.getByText(/^Link this enrollment to Alice Example in the registry\./)
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Approve enrollment"}))
        const confirmation = await dialog("Approve Alice Example?")
        await expect(confirmation.getByText("3 of 4 details match")).toBeVisible()
        await expect(confirmation.getByText("This can't be undone.")).toBeVisible()
        expect(statusChanges()).toEqual([])
        await userEvent.click(confirmation.getByRole("button", {name: "Approve"}))

        await waitFor(() =>
            expect(statusChanges()).toEqual([
                {
                    tenant_id: TENANT_ID,
                    id: APPLICATION_ID,
                    user_id: STORY_IDS.user,
                    area_id: STORY_IDS.area,
                    election_event_id: EVENT_ID,
                },
            ])
        )
        await notified("Alice Example approved. The voter has been told.")
        await waitFor(() => expect(args.goBack).toHaveBeenCalledTimes(1))
    },
}

export const SeeTheRule: Story = {
    play: async ({canvasElement, args}) => {
        await opened(canvasElement, "Alice Example")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "See the rule"}))
        expect(args.onViewRule).toHaveBeenCalledWith({version: 1, rule: 5})
    },
}

export const NoneOfTheseIsTheVoter: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        await continueOn(canvasElement)
        const registry = await voters(canvasElement)
        await registry.findByRole("radio", {name: /^Alice Example/})
        await userEvent.click(registry.getByRole("radio", {name: /^None of these is the voter/}))
        // Without a chosen voter there is nothing to compare with.
        await waitFor(() =>
            expect(canvas.queryByRole("heading", {name: /^Compared with/})).toBeNull()
        )
        await continueOn(canvasElement)

        // The enrollment can only be rejected, for no matching voter.
        const verdict = await decision(canvasElement)
        await expect(verdict.getByRole("radio", {name: /^Approve/})).toBeDisabled()
        await expect(
            verdict.getByText(
                "You found no matching voter, so this enrollment can only be rejected."
            )
        ).toBeVisible()
        await expect(verdict.getByRole("radio", {name: /^Reject/})).toBeChecked()
        const reasons = within(canvas.getByRole("radiogroup", {name: "Reason for rejecting"}))
        await expect(reasons.getByRole("radio", {name: /^No matching voter/})).toBeChecked()
        await expect(canvas.getByText("The voter will see")).toBeVisible()
        await expect(
            canvas.getByText(
                /^We couldn't find a voter in the registry that matches your details\./
            )
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Reject enrollment"}))

        await waitFor(() =>
            expect(statusChanges()).toEqual([
                {
                    tenant_id: TENANT_ID,
                    id: APPLICATION_ID,
                    user_id: "",
                    area_id: STORY_IDS.area,
                    election_event_id: EVENT_ID,
                    rejection_reason: "no-matching-voter",
                    rejection_message: undefined,
                },
            ])
        )
        await notified("Alice Example rejected. The voter has been told.")
        await waitFor(() => expect(args.goBack).toHaveBeenCalledTimes(1))
    },
}

export const RejectWithAMessage: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        // The steps can be opened in any order.
        await userEvent.click(canvas.getByRole("button", {name: /Decide/}))
        const verdict = await decision(canvasElement)
        await expect(
            verdict.getByText("Choose the matching voter in step 2 to approve.")
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Approve enrollment"})).toBeDisabled()
        await userEvent.click(verdict.getByRole("radio", {name: /^Reject/}))
        const reasons = within(
            await canvas.findByRole("radiogroup", {name: "Reason for rejecting"})
        )
        expect(reasons.getAllByRole("radio")).toHaveLength(4)
        await userEvent.click(reasons.getByRole("radio", {name: /^Other/}))

        // The reason Other needs a message for the voter.
        const message = await canvas.findByRole("textbox", {name: "Message to the voter"})
        await expect(
            canvas.getByText("Write a message for the voter when the reason is Other.")
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Reject enrollment"})).toBeDisabled()
        await userEvent.type(message, "  Your passport has expired.  ")
        await userEvent.click(canvas.getByRole("button", {name: "Reject enrollment"}))

        await waitFor(() =>
            expect(statusChanges()).toEqual([
                {
                    tenant_id: TENANT_ID,
                    id: APPLICATION_ID,
                    user_id: "",
                    area_id: STORY_IDS.area,
                    election_event_id: EVENT_ID,
                    rejection_reason: "other",
                    rejection_message: "Your passport has expired.",
                },
            ])
        )
        await notified("Alice Example rejected. The voter has been told.")
        await waitFor(() => expect(args.goBack).toHaveBeenCalledTimes(1))
    },
}

export const TypedByHand: Story = {
    args: {applicationId: MANUAL_APPLICATION_ID},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Carol Sample")
        await expect(
            canvas.getByText(/^The voter typed their details by hand instead of scanning an ID\./)
        ).toBeVisible()
        await expect(canvas.getByText("Rule 2 of matrix version 1")).toBeVisible()
        await expect(canvas.getByText("Identity typed by hand")).toBeVisible()
        await expect(canvas.getByText("Check them face to face before approving")).toBeVisible()
        const checked = canvas.getByRole("checkbox", {
            name: "I checked the voter's ID in person or by video call, and it matches this enrollment.",
        })
        await expect(checked).not.toBeChecked()

        await continueOn(canvasElement)
        const registry = await voters(canvasElement)
        await userEvent.click(
            await registry.findByRole("radio", {
                name: /^Carol Sample.*Best match · 4 of 4 details match$/,
            })
        )
        await continueOn(canvasElement)

        // Approving waits for the face-to-face check of step 1.
        const verdict = await decision(canvasElement)
        await expect(verdict.getByRole("radio", {name: /^Approve/})).toBeDisabled()
        await expect(
            verdict.getByText("Confirm the face-to-face check in step 1 to approve.")
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Approve enrollment"})).toBeDisabled()
        await userEvent.click(
            canvas.getByRole("button", {
                name: /Check the identity.*Face-to-face check not confirmed yet/,
            })
        )
        await userEvent.click(checked)
        await userEvent.click(canvas.getByRole("button", {name: /Decide/}))
        await expect(
            canvas.getByRole("button", {
                name: /Check the identity.*Face-to-face check confirmed/,
            })
        ).toBeVisible()
        await waitFor(() => expect(verdict.getByRole("radio", {name: /^Approve/})).toBeChecked())
        await userEvent.click(canvas.getByRole("button", {name: "Approve enrollment"}))

        const confirmation = await dialog("Approve Carol Sample?")
        await expect(
            confirmation.getByText("You checked the voter's ID face to face.")
        ).toBeVisible()
        await userEvent.click(confirmation.getByRole("button", {name: "Approve"}))
        await waitFor(() =>
            expect(statusChanges()).toEqual([
                {
                    tenant_id: TENANT_ID,
                    id: MANUAL_APPLICATION_ID,
                    user_id: CAROL_VOTER_ID,
                    area_id: STORY_IDS.secondArea,
                    election_event_id: EVENT_ID,
                },
            ])
        )
        await waitFor(() => expect(args.goBack).toHaveBeenCalledTimes(1))
    },
}

export const SearchTheRegistry: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        await continueOn(canvasElement)
        const registry = await voters(canvasElement)
        await registry.findByRole("radio", {name: /^Alice Example/})
        await userEvent.type(canvas.getByRole("textbox", {name: SEARCH}), "example")

        // The voters found by first name, last name or email, the closest first.
        const alicia = await registry.findByRole("radio", {
            name: /^Alicia Example.*Jun 18, 1991 · Lisbon.*1 of 4 details match$/,
        })
        await expect(
            canvas.getByText(
                "These are the voters in the registry that match your search. Choose the one " +
                    "this enrollment belongs to."
            )
        ).toBeVisible()
        expect(
            registry.getAllByRole("radio").map((radio) => radio.closest("label")?.textContent)
        ).toEqual([
            expect.stringMatching(/^AEAlice Example.*3 of 4 details match$/),
            expect.stringMatching(/^AEAlicia Example/),
            expect.stringMatching(/^RERobert Example.*Already enrolled$/),
            expect.stringMatching(/^None of these is the voter/),
        ])
        // A voter who is already enrolled can't be chosen.
        await expect(registry.getByRole("radio", {name: /^Robert Example/})).toBeDisabled()
        expect(listFilters("user").slice(5)).toEqual(
            ["first_name", "last_name", "email"].map((column) => ({
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                election_id: undefined,
                [column]: {IsLike: "example"},
            }))
        )

        await userEvent.click(alicia)
        await expect(
            await canvas.findByRole("heading", {
                name: "Compared with Alicia Example in the registry",
            })
        ).toBeVisible()
        await expect(
            canvas.getByRole("row", {name: "First Name Alice Alicia Different"})
        ).toBeVisible()
        await continueOn(canvasElement)
        await expect(
            canvas.getByRole("button", {
                name: /Find the voter.*Alicia Example · 1 of 4 details match/,
            })
        ).toBeVisible()
        await expect(
            (await decision(canvasElement)).getByText(
                /^Link this enrollment to Alicia Example in the registry\./
            )
        ).toBeVisible()
    },
}

export const NobodyInTheRegistry: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        await continueOn(canvasElement)
        const registry = await voters(canvasElement)
        await registry.findByRole("radio", {name: /^Alice Example/})
        await userEvent.type(canvas.getByRole("textbox", {name: SEARCH}), "zzz")
        await expect(
            await canvas.findByText(
                "No voter in the registry matches. Try searching by name or email."
            )
        ).toBeVisible()
        expect(registry.getAllByRole("radio")).toHaveLength(1)
    },
}

export const RegistryFails: Story = {
    args: {registry: "error"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        // Without the registry voter, the banner names the details that differ.
        await expect(
            canvas.getByText(
                "One detail doesn't match the registry: the date of birth. The approval rules " +
                    "ask a person to check this enrollment."
            )
        ).toBeVisible()
        await continueOn(canvasElement)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            "The registry could not be searched."
        )
        expect((await voters(canvasElement)).getAllByRole("radio")).toHaveLength(1)
    },
}

export const RegistryLoading: Story = {
    args: {registry: "loading"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        await continueOn(canvasElement)
        await expect(
            await canvas.findByRole("progressbar", {name: "Looking in the registry"})
        ).toBeVisible()
    },
}

export const TheVoterIsAlreadyEnrolled: Story = {
    args: {decides: "already-enrolled"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        await continueOn(canvasElement)
        await userEvent.click(
            await (await voters(canvasElement)).findByRole("radio", {name: /^Alice Example/})
        )
        await continueOn(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Approve enrollment"}))
        const confirmation = await dialog("Approve Alice Example?")
        await userEvent.click(confirmation.getByRole("button", {name: "Approve"}))
        await notified("This voter is already enrolled.")
        // The review stays open.
        expect(args.goBack).not.toHaveBeenCalled()
        await waitFor(() =>
            expect(canvas.getByRole("button", {name: "Approve enrollment"})).toBeEnabled()
        )
    },
}

export const ApprovingFails: Story = {
    args: {decides: "error"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Alice Example")
        await continueOn(canvasElement)
        await userEvent.click(
            await (await voters(canvasElement)).findByRole("radio", {name: /^Alice Example/})
        )
        await continueOn(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Approve enrollment"}))
        const confirmation = await dialog("Approve Alice Example?")
        await userEvent.click(confirmation.getByRole("button", {name: "Approve"}))
        await notified("The enrollment could not be approved")
        expect(args.goBack).not.toHaveBeenCalled()
    },
}

export const ApprovedEnrollment: Story = {
    args: {applicationId: SECOND_APPLICATION_ID},
    parameters: {
        expectedFailure: {
            reason:
                "Without the steps, whose titles are level 3 headings, the level 4 heading " +
                '"ID check" follows the applicant\'s name, a level 2 heading.',
            a11y: ["heading-order"],
        },
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Bob Example")
        await expect(canvas.getByText("Approved")).toBeVisible()
        await expect(canvas.getByText("How this was decided")).toBeVisible()
        await expect(
            canvas.getByText("admin approved this enrollment on Jan 13, 2026.")
        ).toBeVisible()
        await expect(canvas.getByText("Rule 5 of matrix version 1")).toBeVisible()
        // A decided enrollment shows what the applicant wrote, without the steps.
        await expect(canvas.getByRole("row", {name: /Embassy Lisbon/})).toBeVisible()
        expect(canvas.queryByRole("list", {name: "Review steps"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Continue"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Approve enrollment"})).toBeNull()
        expect(canvas.queryByText(/Waiting/)).toBeNull()
        expect(listFilters("user")).toEqual([])
        await userEvent.click(canvas.getByRole("button", {name: "Back to Approvals"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const RejectedEnrollment: Story = {
    args: {applicationId: REJECTED_APPLICATION_ID},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await opened(canvasElement, "Dan Nobody")
        await expect(canvas.getByText("Rejected")).toBeVisible()
        await expect(canvas.getByText("How this was decided")).toBeVisible()
        await expect(
            canvas.getByText(
                "The approval rules rejected this enrollment automatically: no matching voter."
            )
        ).toBeVisible()
        await expect(canvas.getByText("Last rule of matrix version 1")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "See the rule"}))
        expect(args.onViewRule).toHaveBeenCalledWith({version: 1, rule: null})

        // A rejected enrollment can still be approved, once its voter is found.
        await continueOn(canvasElement)
        await expect(
            await canvas.findByText(
                "No voter in the registry matches. Try searching by name or email."
            )
        ).toBeVisible()
        await continueOn(canvasElement)
        const verdict = await decision(canvasElement)
        expect(verdict.getAllByRole("radio")).toHaveLength(1)
        await expect(verdict.getByRole("radio", {name: /^Approve/})).toBeDisabled()
        await expect(canvas.getByRole("button", {name: "Approve enrollment"})).toBeDisabled()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_applications")).toHaveLength(1))
        await expect(
            within(canvasElement).getByRole("progressbar", {name: "Approvals"})
        ).toBeVisible()
    },
}

export const LoadFails: Story = {
    args: {reads: "error"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            "The enrollment could not be loaded."
        )
        await userEvent.click(canvas.getByRole("button", {name: "Back to Approvals"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}
