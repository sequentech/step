// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {GraphQLError} from "graphql"
import {
    APPROVAL_STEP_UP_REQUIRED,
    clearPendingApproval,
    isStepUpRefusal,
    readPendingApproval,
    requiresApprovalStepUp,
    savePendingApproval,
    stepUpReturnUrl,
} from "./approvalStepUp"

describe("approval step-up", () => {
    it("is off unless the event enables it", () => {
        expect(requiresApprovalStepUp(undefined)).toBe(false)
        expect(requiresApprovalStepUp(null)).toBe(false)
        expect(requiresApprovalStepUp({})).toBe(false)
        expect(requiresApprovalStepUp({approval_step_up_policy: null})).toBe(false)
        expect(requiresApprovalStepUp({approval_step_up_policy: "disabled"})).toBe(false)
        expect(requiresApprovalStepUp({approval_step_up_policy: "enabled"})).toBe(true)
        // As Harvest does, a value that isn't a policy requires it.
        expect(requiresApprovalStepUp({approval_step_up_policy: "Enabled"})).toBe(true)
    })

    it("recognises Harvest's refusal wherever Hasura puts the code", () => {
        const direct = {
            graphQLErrors: [
                new GraphQLError("no", {extensions: {code: APPROVAL_STEP_UP_REQUIRED}}),
            ],
        }
        const wrapped = {
            graphQLErrors: [
                new GraphQLError("no", {
                    extensions: {
                        internal: {
                            response: {
                                body: JSON.stringify({
                                    extensions: {code: APPROVAL_STEP_UP_REQUIRED},
                                }),
                            },
                        },
                    },
                }),
            ],
        }
        const other = {
            graphQLErrors: [new GraphQLError("no", {extensions: {code: "Unauthorized"}})],
        }
        expect(isStepUpRefusal(direct)).toBe(true)
        expect(isStepUpRefusal(wrapped)).toBe(true)
        expect(isStepUpRefusal(other)).toBe(false)
        expect(isStepUpRefusal(new Error("network"))).toBe(false)
    })

    it("returns to the approvals tab of the same page", () => {
        const page = "https://admin.example/sequent_backend_election_event/e1"
        expect(stepUpReturnUrl(`${page}?tabId=data`)).toBe(`${page}?tabId=approvals`)
    })

    it("resumes only the approval of the same tenant and event", () => {
        const pending = {tenantId: "t1", electionEventId: "e1", applicationId: "a1", voterId: "v1"}
        savePendingApproval(pending)
        expect(readPendingApproval("t1", "e2")).toBeNull()
        expect(readPendingApproval("t2", "e1")).toBeNull()
        expect(readPendingApproval("t1", "e1")).toEqual(pending)
        clearPendingApproval()
        expect(readPendingApproval("t1", "e1")).toBeNull()
    })
})
