// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren} from "react"
import type {FetchResult, Operation} from "@apollo/client"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME} from "@/__stories__/fixtures"
import {IApplicationsStatus} from "@/types/applications"
import {IPermissions} from "@/types/keycloak"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"
import {
    EDifferingFields,
    EFieldMatch,
    EIdentityMethod,
    EMatrixReason,
    EMatrixSource,
    type IApprovalMatrix,
    type IRuleConditions,
    type ITestEnrollment,
    validateMatrix,
} from "../approvalMatrix"
import {APPROVAL_ATTRIBUTES} from "./ApprovalsFixture"

const {ACCEPTED, PENDING, REJECTED} = IApplicationsStatus

export const VALID_IDS = ["philippinePassport", "philSysID", "seamanBook", "driversLicense", "iBP"]

/** The matrix an overseas voting event uses until it saves its own. */
export const comelecMatrix = (): IApprovalMatrix => ({
    compared_fields: ["firstName", "middleName", "lastName", "dateOfBirth", "embassy"],
    rules: [
        {
            when: {already_enrolled: true, differing: EDifferingFields.AT_MOST_1},
            then: {decision: REJECTED, reason: EMatrixReason.ALREADY_APPROVED},
        },
        {
            when: {identity: EIdentityMethod.MANUAL_ENTRY},
            then: {decision: PENDING, reason: EMatrixReason.IDENTITY_NOT_VERIFIED},
        },
        {when: {differing: EDifferingFields.NONE}, then: {decision: ACCEPTED}},
        {
            when: {differing: EDifferingFields.EXACTLY_1, fields: {embassy: EFieldMatch.DIFFERS}},
            then: {decision: ACCEPTED},
        },
        {
            when: {differing: EDifferingFields.EXACTLY_1, fields: {embassy: EFieldMatch.MATCHES}},
            then: {decision: PENDING, reason: EMatrixReason.NO_VOTER},
        },
        {
            when: {differing: EDifferingFields.EXACTLY_2, fields: {embassy: EFieldMatch.DIFFERS}},
            then: {decision: PENDING, reason: EMatrixReason.NO_VOTER},
        },
        {
            when: {
                differing: EDifferingFields.EXACTLY_2,
                fields: {middleName: EFieldMatch.DIFFERS, lastName: EFieldMatch.DIFFERS},
            },
            then: {decision: PENDING, reason: EMatrixReason.NO_VOTER},
        },
    ],
    otherwise: {decision: REJECTED, reason: EMatrixReason.NO_VOTER},
})

/** The matrix an association saved for its board election. */
export const associationMatrix = (): IApprovalMatrix => ({
    compared_fields: ["firstName", "lastName", "dateOfBirth"],
    rules: [
        {
            when: {already_enrolled: true},
            then: {decision: REJECTED, reason: EMatrixReason.ALREADY_APPROVED},
        },
        {
            when: {identity: EIdentityMethod.MANUAL_ENTRY},
            then: {decision: PENDING, reason: EMatrixReason.IDENTITY_NOT_VERIFIED},
        },
        {when: {differing: EDifferingFields.NONE}, then: {decision: ACCEPTED}},
    ],
    otherwise: {decision: PENDING, reason: EMatrixReason.NO_VOTER},
})

/** What the approval matrix screen's services do. */
export interface MatrixServices {
    /** The version in force: the built-in rules or a saved one. */
    saved: "built-in" | "association"
    /** Whether the administrator may save new versions. */
    canEdit: boolean
    /** Reading the matrix. */
    reads: "matrix" | "loading" | "error"
    /** Saving a version. */
    saves: "saved" | "error"
    /** Trying an example; default "result". */
    evaluates?: "result" | "error"
}

type Handlers = Parameters<typeof graphqlBoundary>[0]

const differing = (enrollment: ITestEnrollment) =>
    Object.values(enrollment.fields).filter((result) => result === EFieldMatch.DIFFERS).length

const counts: Record<EDifferingFields, (count: number) => boolean> = {
    [EDifferingFields.NONE]: (count) => count === 0,
    [EDifferingFields.EXACTLY_1]: (count) => count === 1,
    [EDifferingFields.AT_MOST_1]: (count) => count <= 1,
    [EDifferingFields.EXACTLY_2]: (count) => count === 2,
    [EDifferingFields.AT_MOST_2]: (count) => count <= 2,
    [EDifferingFields.AT_LEAST_3]: (count) => count >= 3,
}

const applies = (when: IRuleConditions, enrollment: ITestEnrollment) =>
    (!when.identity || when.identity === enrollment.identity) &&
    (when.voter_found === undefined || when.voter_found === enrollment.voter_found) &&
    (when.already_enrolled === undefined ||
        when.already_enrolled === enrollment.already_enrolled) &&
    (!when.valid_id || when.valid_id === enrollment.valid_id) &&
    (!(when.differing || when.fields) || enrollment.voter_found) &&
    (!when.differing || counts[when.differing](differing(enrollment))) &&
    Object.entries(when.fields ?? {}).every(
        ([field, result]) => (enrollment.fields[field] ?? EFieldMatch.DIFFERS) === result
    )

/** Answers the test panel as Harvest does: the first rule that applies decides. */
export function evaluate(matrix: IApprovalMatrix, enrollment: ITestEnrollment) {
    const errors = validateMatrix(matrix).map(({code, rule}) => ({code, rule, field: null}))
    if (errors.length > 0) {
        return {rule: null, decision: null, reason: null, invariant: null, errors}
    }
    const index = matrix.rules.findIndex((rule) => applies(rule.when, enrollment))
    const outcome = index < 0 ? matrix.otherwise : matrix.rules[index].then
    return {
        rule: index < 0 ? null : index + 1,
        decision: outcome.decision,
        reason: outcome.reason ?? null,
        invariant: null,
        errors,
    }
}

const version = (
    number: number,
    source: EMatrixSource,
    matrix: IApprovalMatrix,
    saved: boolean
) => ({
    version: number,
    source,
    matrix,
    sha256: saved ? "9f2c6d1e".repeat(8) : null,
    created_at: saved ? FIXED_TIME : null,
    created_by: saved ? "admin" : null,
    next_version: number + 1,
    valid_ids: VALID_IDS,
})

/** The approval matrix operations of an event. */
export function matrixHandlers({
    saved = "built-in",
    reads = "matrix",
    saves = "saved",
    evaluates = "result",
}: Partial<MatrixServices> = {}): Handlers {
    return {
        getUserProfileAttributes: () => ({
            data: {get_user_profile_attributes: APPROVAL_ATTRIBUTES},
        }),
        GetApprovalMatrix: () => {
            if (reads === "loading") return new Promise<FetchResult>(() => undefined)
            if (reads === "error") {
                return {errors: [new GraphQLError("Synthetic approval matrix read failure")]}
            }
            return {
                data: {
                    get_approval_matrix:
                        saved === "association"
                            ? version(3, EMatrixSource.SAVED, associationMatrix(), true)
                            : version(1, EMatrixSource.BUILT_IN, comelecMatrix(), false),
                },
            }
        },
        EvaluateApprovalMatrix: ({variables}: Operation) =>
            evaluates === "error"
                ? {errors: [new GraphQLError("Synthetic approval matrix evaluation failure")]}
                : {
                      data: {
                          evaluate_approval_matrix: evaluate(
                              variables.matrix,
                              variables.enrollment
                          ),
                      },
                  },
        SaveApprovalMatrix: ({variables}: Operation) => {
            if (saves === "error") {
                return {errors: [new GraphQLError("Synthetic approval matrix save failure")]}
            }
            return {
                data: {
                    save_approval_matrix: version(
                        saved === "association" ? 4 : 2,
                        EMatrixSource.SAVED,
                        variables.matrix,
                        true
                    ),
                },
            }
        },
    }
}

let graphql: ReturnType<typeof graphqlBoundary>

export async function setUpMatrix(services: Partial<MatrixServices> = {}) {
    graphql = graphqlBoundary(matrixHandlers(services), {schema: true})
    await graphql.ready
}

/** The matrix screens as an administrator who reads Approvals and may or may not save. */
export function MatrixScreen({canEdit, children}: PropsWithChildren<{canEdit: boolean}>) {
    const {tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            roles={[
                IPermissions.APPLICATION_READ,
                ...(canEdit ? [IPermissions.APPROVAL_MATRIX_WRITE] : []),
            ]}
            tenant={tenant}
        >
            {children}
        </AdminStoryProvider>
    )
}

export const matrixCalls = (name: string) => graphql.calls.filter((call) => call.name === name)
export const unexpectedCalls = () => graphql.unexpected
