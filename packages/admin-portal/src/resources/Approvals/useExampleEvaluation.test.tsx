/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook} from "@testing-library/react"
import {IApplicationsStatus} from "@/types/applications"
import {EExampleResult, decidingRule, useExampleEvaluation} from "./ApprovalMatrixTest"
import {EDifferingFields, EFieldMatch, EIdentityMethod, IApprovalMatrix} from "./approvalMatrix"
import {EMatrixEntry, sampleEnrollment} from "./approvalExample"

interface QueryResult {
    data?: unknown
    loading?: boolean
    error?: Error
}
let mockResult: QueryResult = {}
const mockUseQuery = jest.fn((_query: unknown, options: {skip: boolean}) =>
    options.skip ? {data: undefined, loading: false} : {loading: false, ...mockResult}
)
jest.mock("@apollo/client", () => ({
    ...jest.requireActual("@apollo/client"),
    useQuery: (query: unknown, options: {skip: boolean}) => mockUseQuery(query, options),
}))

const MATRIX: IApprovalMatrix = {
    compared_fields: ["firstName", "embassy"],
    rules: [
        {when: {differing: EDifferingFields.NONE}, then: {decision: IApplicationsStatus.ACCEPTED}},
    ],
    otherwise: {decision: IApplicationsStatus.PENDING},
}

const answer = (overrides: Record<string, unknown>) => ({
    data: {
        evaluate_approval_matrix: {
            rule: 1,
            decision: IApplicationsStatus.ACCEPTED,
            reason: null,
            invariant: null,
            errors: [],
            ...overrides,
        },
    },
})

const evaluate = (matrix: IApprovalMatrix | null = MATRIX, entry = EMatrixEntry.TOOLBAR) =>
    renderHook(() => useExampleEvaluation("event-1", matrix, sampleEnrollment(), entry, false))
        .result.current

beforeEach(() => {
    mockResult = {}
    mockUseQuery.mockClear()
})

describe("useExampleEvaluation", () => {
    it("decides the applied example with the rules on screen", () => {
        mockResult = answer({rule: 1})
        const evaluation = evaluate()
        expect(evaluation.state).toBe(EExampleResult.DECIDED)
        expect(decidingRule(evaluation)).toBe(1)
        const [, options] = mockUseQuery.mock.calls[0] as unknown as [
            unknown,
            {variables: Record<string, unknown>; fetchPolicy: string},
        ]
        expect(options.fetchPolicy).toBe("no-cache")
        expect(options.variables).toEqual({
            electionEventId: "event-1",
            matrix: MATRIX,
            enrollment: {
                identity: EIdentityMethod.VERIFIED,
                voter_found: true,
                already_enrolled: false,
                valid_id: null,
                fields: {firstName: EFieldMatch.MATCHES, embassy: EFieldMatch.MATCHES},
            },
        })
    })

    it("names the last rule as empty", () => {
        mockResult = answer({rule: null, decision: IApplicationsStatus.PENDING})
        expect(decidingRule(evaluate())).toBeNull()
    })

    it("sends an enrollment's details without results as they were recorded", () => {
        mockResult = answer({})
        evaluate(MATRIX, EMatrixEntry.ENROLLMENT)
        const [, options] = mockUseQuery.mock.calls[0] as unknown as [
            unknown,
            {variables: {enrollment: {fields: unknown}}},
        ]
        expect(options.variables.enrollment.fields).toEqual({})
    })

    it.each([
        ["while the rules are loading", null, {}],
        ["while it is being decided", MATRIX, {loading: true}],
        ["before an answer", MATRIX, {}],
    ])("selects no rule %s", (_, matrix, result) => {
        mockResult = result
        const evaluation = evaluate(matrix)
        expect(evaluation.state).toBe(EExampleResult.PENDING)
        expect(decidingRule(evaluation)).toBeUndefined()
    })

    it("selects no rule from an earlier answer while the current rules are decided", () => {
        // Should a previous answer still be in `data` while new variables load, it selects nothing.
        mockResult = {...answer({rule: 1}), loading: true}
        const evaluation = evaluate()
        expect(evaluation.state).toBe(EExampleResult.PENDING)
        expect(decidingRule(evaluation)).toBeUndefined()
    })

    it("selects no rule when trying fails", () => {
        mockResult = {error: new Error("Synthetic failure")}
        expect(evaluate().state).toBe(EExampleResult.ERROR)
    })

    it("selects no rule for an answer without a known decision", () => {
        mockResult = answer({decision: "MAYBE"})
        expect(decidingRule(evaluate())).toBeUndefined()
        expect(evaluate().state).toBe(EExampleResult.ERROR)
    })

    it("selects no rule while the rules can't be saved", () => {
        mockResult = answer({decision: null, errors: [{code: "NO_CONDITIONS", rule: 1}]})
        const evaluation = evaluate()
        expect(evaluation.state).toBe(EExampleResult.INVALID)
        expect(decidingRule(evaluation)).toBeUndefined()
    })
})
