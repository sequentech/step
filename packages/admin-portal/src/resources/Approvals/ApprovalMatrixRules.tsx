// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {Box, Button, IconButton} from "@mui/material"
import {styled} from "@mui/material/styles"
import AddIcon from "@mui/icons-material/Add"
import CheckCircleIcon from "@mui/icons-material/CheckCircle"
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
    [theme.breakpoints.down("sm")]: {"&[data-actions='true']": {flexWrap: "wrap", rowGap: "8px"}},
    "&:focus-visible": {outline: `2px solid ${theme.palette.brandColor}`, outlineOffset: "2px"},
    "&[data-highlighted='true']": {
        borderColor: theme.palette.brandSuccess,
        boxShadow: `inset 0 0 0 1px ${theme.palette.brandSuccess}`,
        background: "#F0FBF6",
    },
}))

/** A rule's conditions and outcome; on a phone, below its number and its actions, if any. */
const RuleBody = styled("div")(({theme}) => ({
    flexGrow: 1,
    minWidth: 0,
    [theme.breakpoints.down("sm")]: {"[data-actions='true'] > &": {order: 1, flexBasis: "100%"}},
}))

/** A rule's tags wrap between words when a card is narrow. */
const WRAPPING_TAG = {whiteSpace: "normal", maxWidth: "100%"}

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

/** A rule: its position from 1, `null` for the last rule, or none. */
export type RulePointer = number | null | undefined

/** The id of a rule's card, by its position from 1 or `null` for the last rule. */
export const ruleCardId = (rule: number | null) =>
    `approval-matrix-rule-${rule === null ? "otherwise" : rule}`

export interface ApprovalMatrixRulesProps {
    matrix: IApprovalMatrix
    canEdit: boolean
    fieldLabel: FieldLabel
    /** The rule that decides the applied example; the only rule highlighted. */
    appliesTo?: RulePointer
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
                <Tag sx={WRAPPING_TAG}>{label}</Tag>
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

    const pointer = (position: number | null) =>
        appliesTo === position && (
            <TagRow sx={{marginTop: "8px"}}>
                <AccentTag sx={WRAPPING_TAG}>
                    <CheckCircleIcon sx={{fontSize: "16px"}} aria-hidden />
                    {t("approvalsScreen.matrix.appliesToExample")}
                </AccentTag>
            </TagRow>
        )
    const highlight = (position: number | null) => ({
        "id": ruleCardId(position),
        "tabIndex": -1,
        "data-highlighted": appliesTo === position,
        "aria-current": appliesTo === position ? ("true" as const) : undefined,
    })

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
                        <RuleCard key={index} data-actions={canEdit} {...highlight(number)}>
                            <NumberBadge>{number}</NumberBadge>
                            <RuleBody>
                                <Line>
                                    <Word>{t("approvalsScreen.matrix.when")}</Word>
                                    {conditions(rule.when)}
                                </Line>
                                {outcome(rule.then)}
                                {pointer(number)}
                                {errors(number)}
                            </RuleBody>
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
                <RuleCard data-actions={canEdit} {...highlight(null)}>
                    <NumberBadge data-muted="true" aria-hidden>
                        •
                    </NumberBadge>
                    <RuleBody>
                        <Line>
                            <Word>{t("approvalsScreen.matrix.otherwise")}</Word>
                            <span>{t("approvalsScreen.matrix.noneApply")}</span>
                        </Line>
                        {outcome(matrix.otherwise)}
                        {pointer(null)}
                        {errors(null)}
                    </RuleBody>
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
