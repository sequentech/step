// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useQuery} from "@apollo/client"
import {Box, MenuItem, TextField} from "@mui/material"
import {EvaluateApprovalMatrixQuery} from "@/gql/graphql"
import {EVALUATE_APPROVAL_MATRIX} from "@/queries/EvaluateApprovalMatrix"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalOutcomeChip} from "./ApprovalChips"
import {
    EFieldMatch,
    EIdentityMethod,
    FieldLabel,
    IApprovalMatrix,
    ITestEnrollment,
    cleanMatrix,
    defaultEnrollment,
    enrollmentFor,
} from "./approvalMatrix"
import type {RulePointer} from "./ApprovalMatrixRules"
import {Help, Notice, Overline, Segment, SegmentGroup} from "./approvalStyles"

const NOT_REPORTED = ""

const TONE: Record<string, string> = {
    [IApplicationsStatus.ACCEPTED]: "success",
    [IApplicationsStatus.PENDING]: "review",
    [IApplicationsStatus.REJECTED]: "error",
}

export interface ApprovalMatrixTestProps {
    electionEventId: string
    /** The rules on screen, saved or not. */
    matrix: IApprovalMatrix
    validIds: string[]
    fieldLabel: FieldLabel
    /** Tells which rule decides the example: a position from 1, or empty for the last rule. */
    onResult?: (rule: RulePointer) => void
}

export const ApprovalMatrixTest: React.FC<ApprovalMatrixTestProps> = ({
    electionEventId,
    matrix,
    validIds,
    fieldLabel,
    onResult,
}) => {
    const {t} = useTranslation()
    const [described, setDescribed] = useState<ITestEnrollment>(() =>
        defaultEnrollment(matrix.compared_fields)
    )
    const tested = useMemo(() => cleanMatrix(matrix), [matrix])
    const enrollment = useMemo(
        () => enrollmentFor(described, tested.compared_fields),
        [described, tested.compared_fields]
    )

    const {data, error} = useQuery<EvaluateApprovalMatrixQuery>(EVALUATE_APPROVAL_MATRIX, {
        variables: {electionEventId, matrix: tested, enrollment},
        fetchPolicy: "no-cache",
    })
    const result = data?.evaluate_approval_matrix
    const decided = result?.decision ? (result.rule ?? null) : undefined

    useEffect(() => {
        onResult?.(decided)
    }, [decided, onResult])

    const choice = <T,>(
        label: string,
        options: Array<{value: T; text: string}>,
        value: T,
        onChoose: (value: T) => void,
        disabled = false
    ) => (
        <SegmentGroup role="group" aria-label={label}>
            {options.map((option) => (
                <Segment
                    key={String(option.value)}
                    type="button"
                    aria-pressed={option.value === value}
                    disabled={disabled}
                    onClick={() => onChoose(option.value)}
                >
                    {option.text}
                </Segment>
            ))}
        </SegmentGroup>
    )

    const yesNo = [
        {value: true, text: String(t("approvalsScreen.matrix.dialog.yes"))},
        {value: false, text: String(t("approvalsScreen.matrix.dialog.no"))},
    ]
    const identityLabel = String(t("approvalsScreen.matrix.dialog.identity"))
    const voterFoundLabel = String(t("approvalsScreen.matrix.dialog.voterFound"))
    const alreadyEnrolledLabel = String(t("approvalsScreen.matrix.dialog.alreadyEnrolled"))

    return (
        <>
            <Help>{t("approvalsScreen.matrix.testHelp")}</Help>
            <Overline>{identityLabel}</Overline>
            {choice<EIdentityMethod | null>(
                identityLabel,
                Object.values(EIdentityMethod).map((method) => ({
                    value: method,
                    text: String(t(`approvalsScreen.matrix.identity.${method}`)),
                })),
                enrollment.identity,
                (identity) => setDescribed({...enrollment, identity})
            )}
            <Overline>{voterFoundLabel}</Overline>
            {choice(voterFoundLabel, yesNo, enrollment.voter_found, (voter_found) =>
                setDescribed({...enrollment, voter_found})
            )}
            <Overline>{alreadyEnrolledLabel}</Overline>
            {choice(
                alreadyEnrolledLabel,
                yesNo,
                enrollment.voter_found && enrollment.already_enrolled,
                (already_enrolled) => setDescribed({...enrollment, already_enrolled}),
                !enrollment.voter_found
            )}
            <Overline>{t("approvalsScreen.matrix.dialog.validId")}</Overline>
            <TextField
                select
                fullWidth
                size="small"
                slotProps={{
                    select: {displayEmpty: true},
                    htmlInput: {"aria-label": String(t("approvalsScreen.matrix.dialog.validId"))},
                }}
                value={enrollment.valid_id ?? NOT_REPORTED}
                onChange={(event) =>
                    setDescribed({...enrollment, valid_id: event.target.value || null})
                }
            >
                <MenuItem value={NOT_REPORTED}>
                    {t("approvalsScreen.matrix.dialog.notReported")}
                </MenuItem>
                {validIds.map((id) => (
                    <MenuItem key={id} value={id}>
                        {t(id)}
                    </MenuItem>
                ))}
            </TextField>
            {enrollment.voter_found && tested.compared_fields.length > 0 && (
                <>
                    <Overline>{t("approvalsScreen.matrix.testDetails")}</Overline>
                    <Box sx={{display: "flex", flexDirection: "column", gap: "8px"}}>
                        {tested.compared_fields.map((field) => (
                            <Box
                                key={field}
                                sx={{
                                    display: "flex",
                                    alignItems: "center",
                                    justifyContent: "space-between",
                                    gap: "8px",
                                    fontSize: "14px",
                                }}
                            >
                                <span>{fieldLabel(field)}</span>
                                {choice(
                                    fieldLabel(field),
                                    Object.values(EFieldMatch).map((match) => ({
                                        value: match,
                                        text: String(
                                            t(`approvalsScreen.matrix.fieldMatch.${match}`)
                                        ),
                                    })),
                                    enrollment.fields[field],
                                    (match) =>
                                        setDescribed({
                                            ...enrollment,
                                            fields: {...enrollment.fields, [field]: match},
                                        })
                                )}
                            </Box>
                        ))}
                    </Box>
                </>
            )}
            <Box
                sx={{marginTop: "20px"}}
                role="status"
                aria-label={String(t("approvalsScreen.matrix.test"))}
            >
                {error && (
                    <Notice data-tone="error">{t("approvalsScreen.matrix.testError")}</Notice>
                )}
                {result && result.errors.length > 0 && (
                    <Notice data-tone="warning">
                        <div>
                            {t("approvalsScreen.matrix.testInvalid")}
                            {result.errors.map((problem, index) => {
                                const text = t(`approvalsScreen.matrix.errors.${problem.code}`)
                                return (
                                    <div key={index}>
                                        {problem.rule
                                            ? t("approvalsScreen.matrix.ruleError", {
                                                  number: problem.rule,
                                                  error: text,
                                              })
                                            : text}
                                    </div>
                                )
                            })}
                        </div>
                    </Notice>
                )}
                {result?.decision && (
                    <Notice data-tone={TONE[result.decision] ?? "neutral"}>
                        <div>
                            <Overline sx={{margin: "0 0 8px", color: "inherit"}}>
                                {result.rule
                                    ? t("approvalsScreen.matrix.applies", {number: result.rule})
                                    : t("approvalsScreen.matrix.otherwiseApplies")}
                            </Overline>
                            <ApprovalOutcomeChip decision={result.decision} />
                            {result.reason && (
                                <Box sx={{marginTop: "8px"}}>
                                    {t(`approvalsScreen.matrix.voterText.${result.reason}`)}
                                </Box>
                            )}
                            {result.invariant && (
                                <Box sx={{marginTop: "8px"}}>
                                    {t(`approvalsScreen.matrix.invariants.${result.invariant}`)}
                                </Box>
                            )}
                        </div>
                    </Notice>
                )}
            </Box>
        </>
    )
}
