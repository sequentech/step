// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useQuery} from "@apollo/client"
import {Alert, Box, MenuItem, TextField, Typography} from "@mui/material"
import {EvaluateApprovalMatrixQuery} from "@/gql/graphql"
import {EVALUATE_APPROVAL_MATRIX} from "@/queries/EvaluateApprovalMatrix"
import {StatusApplicationChip} from "@/components/StatusApplicationChip"
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

const NOT_REPORTED = ""
// A select whose value is empty shows that option instead of a blank.
const EMPTY_SHOWN = {select: {displayEmpty: true}, inputLabel: {shrink: true}}
const YES = "true"
const NO = "false"

const GRID = {
    display: "grid",
    gap: "1rem",
    gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
    marginTop: "1rem",
}

export interface ApprovalMatrixTestProps {
    electionEventId: string
    /** The rules on screen, saved or not. */
    matrix: IApprovalMatrix
    validIds: string[]
    fieldLabel: FieldLabel
}

export const ApprovalMatrixTest: React.FC<ApprovalMatrixTestProps> = ({
    electionEventId,
    matrix,
    validIds,
    fieldLabel,
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

    const yesNo = [
        <MenuItem key={YES} value={YES}>
            {t("approvalsScreen.matrix.dialog.yes")}
        </MenuItem>,
        <MenuItem key={NO} value={NO}>
            {t("approvalsScreen.matrix.dialog.no")}
        </MenuItem>,
    ]

    return (
        <>
            <Typography variant="body2">{t("approvalsScreen.matrix.testHelp")}</Typography>
            <Box sx={GRID}>
                <TextField
                    select
                    slotProps={EMPTY_SHOWN}
                    label={t("approvalsScreen.matrix.dialog.identity")}
                    value={enrollment.identity ?? NOT_REPORTED}
                    onChange={(event) =>
                        setDescribed({
                            ...enrollment,
                            identity:
                                Object.values(EIdentityMethod).find(
                                    (method) => method === event.target.value
                                ) ?? null,
                        })
                    }
                >
                    {Object.values(EIdentityMethod).map((method) => (
                        <MenuItem key={method} value={method}>
                            {t(`approvalsScreen.matrix.identity.${method}`)}
                        </MenuItem>
                    ))}
                    <MenuItem value={NOT_REPORTED}>
                        {t("approvalsScreen.matrix.dialog.notReported")}
                    </MenuItem>
                </TextField>
                <TextField
                    select
                    slotProps={EMPTY_SHOWN}
                    label={t("approvalsScreen.matrix.dialog.voterFound")}
                    value={enrollment.voter_found ? YES : NO}
                    onChange={(event) =>
                        setDescribed({...enrollment, voter_found: event.target.value === YES})
                    }
                >
                    {yesNo}
                </TextField>
                <TextField
                    select
                    slotProps={EMPTY_SHOWN}
                    label={t("approvalsScreen.matrix.dialog.alreadyEnrolled")}
                    value={enrollment.voter_found && enrollment.already_enrolled ? YES : NO}
                    disabled={!enrollment.voter_found}
                    onChange={(event) =>
                        setDescribed({...enrollment, already_enrolled: event.target.value === YES})
                    }
                >
                    {yesNo}
                </TextField>
                <TextField
                    select
                    slotProps={EMPTY_SHOWN}
                    label={t("approvalsScreen.matrix.dialog.validId")}
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
            </Box>
            {enrollment.voter_found && (
                <Box sx={GRID}>
                    {tested.compared_fields.map((field) => (
                        <TextField
                            key={field}
                            select
                            slotProps={EMPTY_SHOWN}
                            label={fieldLabel(field)}
                            value={enrollment.fields[field]}
                            onChange={(event) =>
                                setDescribed({
                                    ...enrollment,
                                    fields: {
                                        ...enrollment.fields,
                                        [field]:
                                            event.target.value === EFieldMatch.DIFFERS
                                                ? EFieldMatch.DIFFERS
                                                : EFieldMatch.MATCHES,
                                    },
                                })
                            }
                        >
                            {Object.values(EFieldMatch).map((match) => (
                                <MenuItem key={match} value={match}>
                                    {t(`approvalsScreen.matrix.fieldMatch.${match}`)}
                                </MenuItem>
                            ))}
                        </TextField>
                    ))}
                </Box>
            )}
            <Box
                sx={{marginTop: "1.5rem"}}
                role="status"
                aria-label={String(t("approvalsScreen.matrix.test"))}
            >
                {error && <Alert severity="error">{t("approvalsScreen.matrix.testError")}</Alert>}
                {result && result.errors.length > 0 && (
                    <Alert severity="warning">
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
                    </Alert>
                )}
                {result?.decision && (
                    <Box sx={{display: "flex", flexWrap: "wrap", alignItems: "center", gap: "8px"}}>
                        <Typography component="span" sx={{fontWeight: 500}}>
                            {result.rule
                                ? t("approvalsScreen.matrix.applies", {number: result.rule})
                                : t("approvalsScreen.matrix.otherwiseApplies")}
                        </Typography>
                        <StatusApplicationChip status={result.decision} />
                        {result.reason && (
                            <span>{t(`approvalsScreen.matrix.reasons.${result.reason}`)}</span>
                        )}
                    </Box>
                )}
                {result?.invariant && (
                    <Typography variant="body2" sx={{marginTop: "0.5rem"}}>
                        {t(`approvalsScreen.matrix.invariants.${result.invariant}`)}
                    </Typography>
                )}
            </Box>
        </>
    )
}
