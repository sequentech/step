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
        const items = new Map<string, string>()
        const storage = {
            getItem: (key: string) => items.get(key) ?? null,
            setItem: (key: string, value: string) => items.set(key, value),
            removeItem: (key: string) => items.delete(key),
        } as unknown as Storage
        const pending = {tenantId: "t1", electionEventId: "e1", applicationId: "a1", voterId: "v1"}
        savePendingApproval(pending, storage)
        expect(readPendingApproval("t1", "e2", storage)).toBeNull()
        expect(readPendingApproval("t2", "e1", storage)).toBeNull()
        expect(readPendingApproval(null, "e1", storage)).toBeNull()
        expect(readPendingApproval("t1", "e1", storage)).toEqual(pending)
        clearPendingApproval(storage)
        expect(readPendingApproval("t1", "e1", storage)).toBeNull()
    })

    it("reads nothing when the storage can't be used", () => {
        const blocked = {
            getItem: () => {
                throw new Error("blocked")
            },
            removeItem: () => {
                throw new Error("blocked")
            },
        } as unknown as Storage
        expect(readPendingApproval("t1", "e1", blocked)).toBeNull()
        expect(() => clearPendingApproval(blocked)).not.toThrow()
    })
})
