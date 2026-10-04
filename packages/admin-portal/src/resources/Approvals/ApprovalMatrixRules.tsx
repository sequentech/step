// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {Box, Button, IconButton} from "@mui/material"
import Table from "@mui/material/Table"
import TableBody from "@mui/material/TableBody"
import TableCell from "@mui/material/TableCell"
import TableContainer from "@mui/material/TableContainer"
import TableHead from "@mui/material/TableHead"
import TableRow from "@mui/material/TableRow"
import Paper from "@mui/material/Paper"
import AddIcon from "@mui/icons-material/Add"
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward"
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward"
import DeleteIcon from "@mui/icons-material/Delete"
import EditIcon from "@mui/icons-material/Edit"
import {StatusApplicationChip} from "@/components/StatusApplicationChip"
import {FieldLabel, IApprovalMatrix, IRuleOutcome, conditionLabels} from "./approvalMatrix"

export interface ApprovalMatrixRulesProps {
    matrix: IApprovalMatrix
    canEdit: boolean
    fieldLabel: FieldLabel
    /** Opens the editor for the rule at this position, or for the last rule when empty. */
    onEdit: (index: number | null) => void
    onMove: (index: number, offset: -1 | 1) => void
    onDelete: (index: number) => void
    onAdd: () => void
}

export const ApprovalMatrixRules: React.FC<ApprovalMatrixRulesProps> = ({
    matrix,
    canEdit,
    fieldLabel,
    onEdit,
    onMove,
    onDelete,
    onAdd,
}) => {
    const {t} = useTranslation()

    const outcomeCells = (outcome: IRuleOutcome) => (
        <>
            <TableCell>
                <StatusApplicationChip status={outcome.decision} />
            </TableCell>
            <TableCell>
                {outcome.reason ? t(`approvalsScreen.matrix.reasons.${outcome.reason}`) : "-"}
            </TableCell>
        </>
    )

    return (
        <>
            <TableContainer component={Paper}>
                <Table aria-label={String(t("approvalsScreen.matrix.rules"))}>
                    <TableHead>
                        <TableRow>
                            <TableCell>{t("approvalsScreen.matrix.columns.number")}</TableCell>
                            <TableCell>{t("approvalsScreen.matrix.columns.conditions")}</TableCell>
                            <TableCell>{t("approvalsScreen.matrix.columns.decision")}</TableCell>
                            <TableCell>{t("approvalsScreen.matrix.columns.reason")}</TableCell>
                            {canEdit && (
                                <TableCell>{t("approvalsScreen.matrix.columns.actions")}</TableCell>
                            )}
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {matrix.rules.map((rule, index) => {
                            const number = index + 1
                            return (
                                <TableRow key={index}>
                                    <TableCell>{number}</TableCell>
                                    <TableCell>
                                        {conditionLabels(rule.when, t, fieldLabel).map((label) => (
                                            <div key={label}>{label}</div>
                                        ))}
                                    </TableCell>
                                    {outcomeCells(rule.then)}
                                    {canEdit && (
                                        <TableCell sx={{whiteSpace: "nowrap"}}>
                                            <IconButton
                                                size="small"
                                                aria-label={String(
                                                    t("approvalsScreen.matrix.actions.edit", {
                                                        number,
                                                    })
                                                )}
                                                onClick={() => onEdit(index)}
                                            >
                                                <EditIcon />
                                            </IconButton>
                                            <IconButton
                                                size="small"
                                                aria-label={String(
                                                    t("approvalsScreen.matrix.actions.moveUp", {
                                                        number,
                                                    })
                                                )}
                                                disabled={index === 0}
                                                onClick={() => onMove(index, -1)}
                                            >
                                                <ArrowUpwardIcon />
                                            </IconButton>
                                            <IconButton
                                                size="small"
                                                aria-label={String(
                                                    t("approvalsScreen.matrix.actions.moveDown", {
                                                        number,
                                                    })
                                                )}
                                                disabled={index === matrix.rules.length - 1}
                                                onClick={() => onMove(index, 1)}
                                            >
                                                <ArrowDownwardIcon />
                                            </IconButton>
                                            <IconButton
                                                size="small"
                                                aria-label={String(
                                                    t("approvalsScreen.matrix.actions.delete", {
                                                        number,
                                                    })
                                                )}
                                                onClick={() => onDelete(index)}
                                            >
                                                <DeleteIcon />
                                            </IconButton>
                                        </TableCell>
                                    )}
                                </TableRow>
                            )
                        })}
                        <TableRow>
                            <TableCell />
                            <TableCell>{t("approvalsScreen.matrix.otherwise")}</TableCell>
                            {outcomeCells(matrix.otherwise)}
                            {canEdit && (
                                <TableCell>
                                    <IconButton
                                        size="small"
                                        aria-label={String(
                                            t("approvalsScreen.matrix.actions.editOtherwise")
                                        )}
                                        onClick={() => onEdit(null)}
                                    >
                                        <EditIcon />
                                    </IconButton>
                                </TableCell>
                            )}
                        </TableRow>
                    </TableBody>
                </Table>
            </TableContainer>
            {canEdit && (
                <Box sx={{marginTop: "1rem"}}>
                    <Button onClick={onAdd} startIcon={<AddIcon />}>
                        {t("approvalsScreen.matrix.addRule")}
                    </Button>
                </Box>
            )}
        </>
    )
}
