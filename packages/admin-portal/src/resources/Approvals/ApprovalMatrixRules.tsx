// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {Box, Button, IconButton} from "@mui/material"
import {styled} from "@mui/material/styles"
import AddIcon from "@mui/icons-material/Add"
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward"
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward"
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline"
import EditIcon from "@mui/icons-material/Edit"
import {ApprovalOutcomeChip} from "./ApprovalChips"
import {
    EMatrixError,
    FieldLabel,
    IApprovalMatrix,
    IRuleConditions,
    IRuleOutcome,
    conditionLabels,
} from "./approvalMatrix"
import {AccentTag, Muted, Notice, NumberBadge, Tag, TagRow} from "./approvalStyles"

const RuleCard = styled("li")(({theme}) => ({
    "display": "flex",
    "gap": "16px",
    "padding": "16px",
    "border": "1px solid #E3E7EF",
    "borderRadius": "10px",
    "background": theme.palette.white,
    "listStyle": "none",
    "&[data-highlighted='true']": {
        borderColor: theme.palette.brandSuccess,
        boxShadow: `inset 0 0 0 1px ${theme.palette.brandSuccess}`,
        background: "#F0FBF6",
    },
}))

const RuleList = styled("ol")({
    display: "flex",
    flexDirection: "column",
    gap: "12px",
    margin: 0,
    padding: 0,
})

const Line = styled("div")({
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "8px",
    minHeight: "32px",
})

const Word = styled("span")({
    minWidth: "40px",
    color: "#3D4353",
    fontSize: "14px",
    fontWeight: 600,
})

const Actions = styled("div")({
    display: "flex",
    alignItems: "flex-start",
    marginLeft: "auto",
    flexShrink: 0,
})

/** The rule that an example or an enrollment points at. `null` is the last rule. */
export type RulePointer = number | null | undefined

export interface ApprovalMatrixRulesProps {
    matrix: IApprovalMatrix
    canEdit: boolean
    fieldLabel: FieldLabel
    /** The position, starting at 1, of the rule that decides the example on screen. */
    appliesTo?: RulePointer
    /** The rule that decided the enrollment the administrator came from. */
    cameFrom?: RulePointer
    /** Why each rule can't be saved, by its position starting at 1. */
    problems?: Array<{code: EMatrixError; rule: number | null}>
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
    appliesTo,
    cameFrom,
    problems = [],
    onEdit,
    onMove,
    onDelete,
    onAdd,
}) => {
    const {t} = useTranslation()

    const conditions = (when: IRuleConditions) => {
        const labels = conditionLabels(when, t, fieldLabel)
        return labels.map((label, index) => (
            <React.Fragment key={label}>
                <Tag>{label}</Tag>
                {index < labels.length - 1 && <Muted>{t("approvalsScreen.matrix.andWord")}</Muted>}
            </React.Fragment>
        ))
    }

    const outcome = (then: IRuleOutcome) => (
        <Line>
            <Word>{t("approvalsScreen.matrix.then")}</Word>
            <ApprovalOutcomeChip decision={then.decision} />
            {then.reason && (
                <Muted>
                    {t("approvalsScreen.matrix.voterIsTold", {
                        reason: t(`approvalsScreen.matrix.reasons.${then.reason}`),
                    })}
                </Muted>
            )}
        </Line>
    )

    const pointers = (position: number | null) =>
        (appliesTo === position || cameFrom === position) && (
            <TagRow sx={{marginTop: "8px"}}>
                {appliesTo === position && (
                    <AccentTag>{t("approvalsScreen.matrix.appliesToExample")}</AccentTag>
                )}
                {cameFrom === position && (
                    <AccentTag>{t("approvalsScreen.matrix.cameFrom")}</AccentTag>
                )}
            </TagRow>
        )

    const errors = (position: number | null) => {
        const found = problems.filter(
            ({code, rule}) => rule === position && code !== EMatrixError.NO_COMPARED_FIELDS
        )
        return (
            found.length > 0 && (
                <Notice data-tone="error" role="alert" sx={{marginTop: "8px", padding: "8px 12px"}}>
                    <div>
                        {found.map(({code}) => (
                            <div key={code}>{t(`approvalsScreen.matrix.errors.${code}`)}</div>
                        ))}
                    </div>
                </Notice>
            )
        )
    }

    return (
        <>
            <RuleList aria-label={String(t("approvalsScreen.matrix.rules"))}>
                {matrix.rules.map((rule, index) => {
                    const number = index + 1
                    return (
                        <RuleCard
                            key={index}
                            data-highlighted={appliesTo === number || cameFrom === number}
                        >
                            <NumberBadge>{number}</NumberBadge>
                            <Box sx={{flexGrow: 1, minWidth: 0}}>
                                <Line>
                                    <Word>{t("approvalsScreen.matrix.when")}</Word>
                                    {conditions(rule.when)}
                                </Line>
                                {outcome(rule.then)}
                                {pointers(number)}
                                {errors(number)}
                            </Box>
                            {canEdit && (
                                <Actions>
                                    <IconButton
                                        size="small"
                                        aria-label={String(
                                            t("approvalsScreen.matrix.actions.moveUp", {number})
                                        )}
                                        disabled={index === 0}
                                        onClick={() => onMove(index, -1)}
                                    >
                                        <ArrowUpwardIcon fontSize="small" />
                                    </IconButton>
                                    <IconButton
                                        size="small"
                                        aria-label={String(
                                            t("approvalsScreen.matrix.actions.moveDown", {number})
                                        )}
                                        disabled={index === matrix.rules.length - 1}
                                        onClick={() => onMove(index, 1)}
                                    >
                                        <ArrowDownwardIcon fontSize="small" />
                                    </IconButton>
                                    <IconButton
                                        size="small"
                                        aria-label={String(
                                            t("approvalsScreen.matrix.actions.edit", {number})
                                        )}
                                        onClick={() => onEdit(index)}
                                    >
                                        <EditIcon fontSize="small" />
                                    </IconButton>
                                    <IconButton
                                        size="small"
                                        aria-label={String(
                                            t("approvalsScreen.matrix.actions.delete", {number})
                                        )}
                                        onClick={() => onDelete(index)}
                                    >
                                        <DeleteOutlineIcon fontSize="small" />
                                    </IconButton>
                                </Actions>
                            )}
                        </RuleCard>
                    )
                })}
                <RuleCard data-highlighted={appliesTo === null || cameFrom === null}>
                    <NumberBadge data-muted="true" aria-hidden>
                        •
                    </NumberBadge>
                    <Box sx={{flexGrow: 1, minWidth: 0}}>
                        <Line>
                            <Word>{t("approvalsScreen.matrix.otherwise")}</Word>
                            <span>{t("approvalsScreen.matrix.noneApply")}</span>
                        </Line>
                        {outcome(matrix.otherwise)}
                        {pointers(null)}
                        {errors(null)}
                    </Box>
                    {canEdit && (
                        <Actions>
                            <IconButton
                                size="small"
                                aria-label={String(
                                    t("approvalsScreen.matrix.actions.editOtherwise")
                                )}
                                onClick={() => onEdit(null)}
                            >
                                <EditIcon fontSize="small" />
                            </IconButton>
                        </Actions>
                    )}
                </RuleCard>
            </RuleList>
            {canEdit && (
                <Box sx={{marginTop: "16px"}}>
                    <Button variant="secondary" onClick={onAdd} startIcon={<AddIcon />}>
                        {t("approvalsScreen.matrix.addRule")}
                    </Button>
                </Box>
            )}
        </>
    )
}
