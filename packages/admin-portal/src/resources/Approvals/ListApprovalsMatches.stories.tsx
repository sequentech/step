// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {Sequent_Backend_Applications} from "@/gql/graphql"
import {ApplicationsError, IApplicationsStatus} from "@/types/applications"
import {ListApprovalsMatches} from "./ListApprovalsMatches"
import {SigningProvider} from "@/components/signing/SigningProvider"
import {fakeApi, makePanel} from "@/components/signing/__stories__/fixtures"
import type {ISigningApi} from "@/lib/signing/api"
import {SigningAction} from "@/lib/signing/types"
import {applyTenantTranslationOverrides} from "@/providers/TenantContextProvider"
import {studentCouncil} from "@/resources/ElectionEvent/Signatures/__stories__/SignaturesFixture"
import englishTranslation from "@/translations/en"
import {applicationRecord} from "./__stories__/ApprovalsFixture"
import {
    APPLICATION_ID,
    ApprovalsScreen,
    graphqlCalls,
    listFilters,
    setUpApprovals,
    type ApprovalServices,
} from "./__stories__/ApprovalsScreenFixture"

interface Scenario extends ApprovalServices {
    status: IApplicationsStatus
    /** What the status service answers to the approval. */
    answer: "approved" | "already-approved"
    /** Approving a voter manually needs signatures (its rule is on). */
    signing: boolean
    /** The organization's own words, as tenant translation overrides. */
    organization: "default" | "student-council"
    goBack: Mock<() => void>
}

const REQUEST_ID = "55555555-5555-4555-8555-555555555555"

/** What the approval signs: the server's subject for this application. */
const VOTER_SUBJECT = {
    application_id: APPLICATION_ID,
    applicant_registry_id: STORY_IDS.user,
    decision: "approve",
    submitted_at: "2028-03-02",
    reason: "No automatic match: the passport was renewed after the registry record",
    registry_record: "Example, Alice (alice@example.test)",
}

/** The student council renames the approval and the registry record. */
const COUNCIL: Record<string, string> = {
    ...studentCouncil().overrides,
    "adminPortal:signing.actions.approve-voter.short": "Student approval",
    "adminPortal:signing.details.registry_record": "Student register entry",
}

/** A text as the organization of the story words it. */
const text = (organization: Scenario["organization"], key: string, fallback: string) =>
    organization === "student-council" ? (COUNCIL[`adminPortal:${key}`] ?? fallback) : fallback

let signingApi: ReturnType<typeof fakeApi>

const meta = {
    title: "Admin/Approvals/ListApprovalsMatches",
    component: ListApprovalsMatches,
    args: {
        reads: "records",
        empty: false,
        status: IApplicationsStatus.PENDING,
        answer: "approved",
        signing: false,
        organization: "default",
        goBack: fn(),
    },
    argTypes: {status: {control: "select", options: Object.values(IApplicationsStatus)}},
    parameters: {
        expectedFailure: {
            reason: "The approve action is an icon button without a name.",
            a11y: ["button-name"],
        },
    },
    beforeEach: async ({args}) => {
        signingApi = fakeApi(
            await makePanel({
                action: SigningAction.ApproveVoter,
                subject: VOTER_SUBJECT,
                required: 2,
            })
        )
        if (args.organization === "student-council") {
            applyTenantTranslationOverrides({i18n: {en: COUNCIL}})
        }
        await setUpApprovals(args, {
            ChangeApplicationStatus: () => ({
                data: {
                    ApplicationChangeStatus: args.signing
                        ? {
                              message: null,
                              error: null,
                              signing_request: {
                                  id: REQUEST_ID,
                                  code: "7F3A-91C2",
                                  required: 2,
                                  expires_at: "2099-05-12T11:30:00Z",
                              },
                          }
                        : {
                              message: "Application approved",
                              error:
                                  args.answer === "already-approved"
                                      ? ApplicationsError.APPROVED_VOTER
                                      : null,
                          },
                },
            }),
        })
        return () => applyTenantTranslationOverrides(undefined)
    },
    render: ({status, goBack}) => (
        <ApprovalsScreen>
            <SigningProvider api={signingApi as ISigningApi}>
                <ListApprovalsMatches
                    electionEventId={EVENT_ID}
                    task={applicationRecord({status}) as Sequent_Backend_Applications}
                    goBack={goBack}
                />
            </SigningProvider>
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const voterRow = (canvasElement: HTMLElement, username: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(username)})

async function approve(canvasElement: HTMLElement) {
    const alice = await voterRow(canvasElement, "alice@example.test")
    await userEvent.click(within(alice).getByRole("button"))
    const dialogElement = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialogElement).toBeVisible())
    await userEvent.click(within(dialogElement).getByRole("button", {name: "Approve"}))
    await waitFor(() => expect(dialogElement).not.toBeInTheDocument())
}

const approvals = () => graphqlCalls().filter(({name}) => name === "ChangeApplicationStatus")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await voterRow(canvasElement, "alice@example.test")).toBeVisible()
        // The search attributes of the application preload the name filters.
        await waitFor(() =>
            expect(listFilters("user").at(-1)).toMatchObject({
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                first_name: {IsLike: "Alice"},
                last_name: {IsLike: "Example"},
            })
        )
        expect(approvals()).toEqual([])
    },
}

export const ApproveTheVoter: Story = {
    play: async ({canvasElement, args}) => {
        await approve(canvasElement)
        await waitFor(() => expect(approvals()).toHaveLength(1))
        expect(approvals()[0].variables).toEqual({
            tenant_id: TENANT_ID,
            id: APPLICATION_ID,
            user_id: STORY_IDS.user,
            area_id: STORY_IDS.area,
            election_event_id: EVENT_ID,
        })
        const message = await within(document.body).findByText("Voter approved")
        await waitFor(() => expect(message).toBeVisible())
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const VoterAlreadyApproved: Story = {
    args: {answer: "already-approved"},
    play: async ({canvasElement, args}) => {
        await approve(canvasElement)
        const message = await within(document.body).findByText("Voter is already approved.")
        await waitFor(() => expect(message).toBeVisible())
        expect(approvals()).toHaveLength(1)
        expect(args.goBack).not.toHaveBeenCalled()
    },
}

export const AcceptedApplication: Story = {
    args: {status: IApplicationsStatus.ACCEPTED},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const alice = await voterRow(canvasElement, "alice@example.test")
        // An accepted application offers no voter to approve.
        expect(within(alice).queryByRole("button")).toBeNull()
        expect(approvals()).toEqual([])
    },
}

/** The signing panel the approval opened, once it loaded the request. */
async function signingPanel() {
    await waitFor(() => expect(signingApi.getRequest).toHaveBeenCalledWith(REQUEST_ID))
    const drawer = await waitFor(() => {
        const paper = document.querySelector(".MuiDrawer-paper") as HTMLElement | null
        expect(paper).not.toBeNull()
        return paper as HTMLElement
    })
    await waitFor(() => expect(drawer).toBeVisible())
    return within(drawer)
}

const approvalStory = (organization: Scenario["organization"]): Story => ({
    args: {signing: true, organization},
    // The panel opens over the list, which leaves the accessibility tree.
    parameters: {expectedFailure: null},
    play: async ({canvasElement, args}) => {
        await approve(canvasElement)
        await waitFor(() => expect(approvals()).toHaveLength(1))
        const panel = await signingPanel()
        const signing = englishTranslation.translations.signing
        const short = text(
            organization,
            "signing.actions.approve-voter.short",
            signing.actions["approve-voter"].short
        )
        await expect(panel.getByRole("heading", {name: new RegExp(`^${short} · `)})).toBeVisible()
        const row = (key: keyof typeof signing.details) =>
            panel.getByRole("rowheader", {
                name: text(organization, `signing.details.${key}`, signing.details[key]),
            }).nextElementSibling?.textContent
        expect(row("registry_record")).toBe(VOTER_SUBJECT.registry_record)
        expect(row("reason")).toBe(VOTER_SUBJECT.reason)
        expect(row("submitted_at")).toBe(VOTER_SUBJECT.submitted_at)
        expect(row("application_id")).toBe(VOTER_SUBJECT.application_id)
        // It is not approved yet: no success message.
        expect(within(document.body).queryByText("Voter approved")).toBeNull()
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
})

/** Approving a voter manually opens its signing request (draft 29). */
export const ApprovalWaitsForSignatures: Story = approvalStory("default")

/** The same, in the words of a student council that renamed the approval. */
export const StudentCouncilApprovalWaitsForSignatures: Story = approvalStory("student-council")
