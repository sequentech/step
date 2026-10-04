// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {Alert, Box, Button, MenuItem, TextField} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {IApplicationsStatus} from "@/types/applications"
import {
    EDifferingFields,
    EFieldMatch,
    EIdentityMethod,
    EMatrixError,
    EMatrixReason,
    FieldLabel,
    IApprovalRule,
    IRuleConditions,
    cleanConditions,
    validateRule,
} from "./approvalMatrix"

const ANY = ""
// A select whose value is empty shows that option, "Any", instead of a blank.
const EMPTY_SHOWN = {select: {displayEmpty: true}, inputLabel: {shrink: true}}
const YES = "true"
const NO = "false"

const fromChoice = (value: string): boolean | undefined =>
    value === ANY ? undefined : value === YES

const toChoice = (value: boolean | undefined): string =>
    value === undefined ? ANY : value ? YES : NO

const member = <T extends string>(values: Record<string, T>, value: string): T | undefined =>
    Object.values(values).find((candidate) => candidate === value)

export interface ApprovalMatrixRuleDialogProps {
    open: boolean
    rule: IApprovalRule
    /** The rule's position starting at 1; empty for a new rule. */
    number: number | null
    /** The last rule has a decision and no conditions. */
    isOtherwise: boolean
    comparedFields: string[]
    validIds: string[]
    fieldLabel: FieldLabel
    /** Receives the edited rule on Apply, or nothing on Cancel. */
    onClose: (rule: IApprovalRule | null) => void
}

export const ApprovalMatrixRuleDialog: React.FC<ApprovalMatrixRuleDialogProps> = ({
    open,
    rule,
    number,
    isOtherwise,
    comparedFields,
    validIds,
    fieldLabel,
    onClose,
}) => {
    const {t} = useTranslation()
    const [draft, setDraft] = useState<IApprovalRule>(rule)
    const [errors, setErrors] = useState<EMatrixError[]>([])

    useEffect(() => {
        if (open) {
            setDraft(rule)
            setErrors([])
        }
    }, [open, rule])

    const setCondition = (change: Partial<IRuleConditions>) =>
        setDraft((current) => ({...current, when: {...current.when, ...change}}))

    const apply = () => {
        const edited: IApprovalRule = {
            when: isOtherwise ? {} : cleanConditions(draft.when, comparedFields),
            then:
                draft.then.decision === IApplicationsStatus.ACCEPTED
                    ? {decision: draft.then.decision}
                    : draft.then,
        }
        const found = validateRule(edited, isOtherwise)
        setErrors(found)
        if (found.length === 0) {
            onClose(edited)
        }
    }

    const title = isOtherwise
        ? t("approvalsScreen.matrix.dialog.otherwiseTitle")
        : number === null
          ? t("approvalsScreen.matrix.dialog.newTitle")
          : t("approvalsScreen.matrix.dialog.editTitle", {number})
    const any = <MenuItem value={ANY}>{t("approvalsScreen.matrix.dialog.any")}</MenuItem>
    const yesNo = [
        <MenuItem key={YES} value={YES}>
            {t("approvalsScreen.matrix.dialog.yes")}
        </MenuItem>,
        <MenuItem key={NO} value={NO}>
            {t("approvalsScreen.matrix.dialog.no")}
        </MenuItem>,
    ]

    return (
        <Dialog
            variant="info"
            open={open}
            title={String(title)}
            cancel={String(t("common.label.cancel"))}
            // The dialog's own OK button stays disabled once clicked, and Apply
            // can be clicked again after correcting the rule.
            middleActions={[
                <Button key="apply" onClick={apply} sx={{minWidth: "unset", flexGrow: 2}}>
                    {t("approvalsScreen.matrix.dialog.apply")}
                </Button>,
            ]}
            fullWidth={true}
            maxWidth="md"
            handleClose={() => onClose(null)}
        >
            <Box
                sx={{
                    display: "grid",
                    gap: "1rem",
                    gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
                    paddingTop: "0.5rem",
                }}
            >
                {!isOtherwise && (
                    <>
                        <TextField
                            select
                            slotProps={EMPTY_SHOWN}
                            label={t("approvalsScreen.matrix.dialog.identity")}
                            value={draft.when.identity ?? ANY}
                            onChange={(event) =>
                                setCondition({
                                    identity: member(EIdentityMethod, event.target.value),
                                })
                            }
                        >
                            {any}
                            {Object.values(EIdentityMethod).map((method) => (
                                <MenuItem key={method} value={method}>
                                    {t(`approvalsScreen.matrix.identity.${method}`)}
                                </MenuItem>
                            ))}
                        </TextField>
                        <TextField
                            select
                            slotProps={EMPTY_SHOWN}
                            label={t("approvalsScreen.matrix.dialog.voterFound")}
                            value={toChoice(draft.when.voter_found)}
                            onChange={(event) =>
                                setCondition({voter_found: fromChoice(event.target.value)})
                            }
                        >
                            {any}
                            {yesNo}
                        </TextField>
                        <TextField
                            select
                            slotProps={EMPTY_SHOWN}
                            label={t("approvalsScreen.matrix.dialog.alreadyEnrolled")}
                            value={toChoice(draft.when.already_enrolled)}
                            onChange={(event) =>
                                setCondition({already_enrolled: fromChoice(event.target.value)})
                            }
                        >
                            {any}
                            {yesNo}
                        </TextField>
                        <TextField
                            select
                            slotProps={EMPTY_SHOWN}
                            label={t("approvalsScreen.matrix.dialog.validId")}
                            value={draft.when.valid_id ?? ANY}
                            onChange={(event) =>
                                setCondition({valid_id: event.target.value || undefined})
                            }
                        >
                            {any}
                            {[
                                ...validIds,
                                ...(draft.when.valid_id && !validIds.includes(draft.when.valid_id)
                                    ? [draft.when.valid_id]
                                    : []),
                            ].map((id) => (
                                <MenuItem key={id} value={id}>
                                    {t(id)}
                                </MenuItem>
                            ))}
                        </TextField>
                        <TextField
                            select
                            slotProps={EMPTY_SHOWN}
                            label={t("approvalsScreen.matrix.dialog.differing")}
                            value={draft.when.differing ?? ANY}
                            onChange={(event) =>
                                setCondition({
                                    differing: member(EDifferingFields, event.target.value),
                                })
                            }
                        >
                            {any}
                            {Object.values(EDifferingFields).map((count) => (
                                <MenuItem key={count} value={count}>
                                    {t(`approvalsScreen.matrix.differing.${count}`)}
                                </MenuItem>
                            ))}
                        </TextField>
                        {comparedFields.map((field) => (
                            <TextField
                                key={field}
                                select
                                slotProps={EMPTY_SHOWN}
                                label={fieldLabel(field)}
                                value={draft.when.fields?.[field] ?? ANY}
                                onChange={(event) => {
                                    const fields = {...draft.when.fields}
                                    const result = member(EFieldMatch, event.target.value)
                                    if (result) {
                                        fields[field] = result
                                    } else {
                                        delete fields[field]
                                    }
                                    setCondition({fields})
                                }}
                            >
                                {any}
                                {Object.values(EFieldMatch).map((result) => (
                                    <MenuItem key={result} value={result}>
                                        {t(`approvalsScreen.matrix.fieldMatch.${result}`)}
                                    </MenuItem>
                                ))}
                            </TextField>
                        ))}
                    </>
                )}
            </Box>
            <Box
                sx={{
                    display: "grid",
                    gap: "1rem",
                    gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
                    marginTop: "1rem",
                }}
            >
                <TextField
                    select
                    slotProps={EMPTY_SHOWN}
                    label={t("approvalsScreen.matrix.dialog.decision")}
                    value={draft.then.decision}
                    onChange={(event) =>
                        setDraft((current) => ({
                            ...current,
                            then: {
                                ...current.then,
                                decision:
                                    member(IApplicationsStatus, event.target.value) ??
                                    current.then.decision,
                            },
                        }))
                    }
                >
                    {[
                        IApplicationsStatus.ACCEPTED,
                        IApplicationsStatus.PENDING,
                        IApplicationsStatus.REJECTED,
                    ].map((decision) => (
                        <MenuItem key={decision} value={decision}>
                            {t(`approvalsScreen.matrix.decisions.${decision}`)}
                        </MenuItem>
                    ))}
                </TextField>
                <TextField
                    select
                    slotProps={EMPTY_SHOWN}
                    label={t("approvalsScreen.matrix.dialog.reason")}
                    value={
                        draft.then.decision === IApplicationsStatus.ACCEPTED
                            ? ANY
                            : (draft.then.reason ?? ANY)
                    }
                    disabled={draft.then.decision === IApplicationsStatus.ACCEPTED}
                    onChange={(event) =>
                        setDraft((current) => ({
                            ...current,
                            then: {
                                decision: current.then.decision,
                                reason: member(EMatrixReason, event.target.value),
                            },
                        }))
                    }
                >
                    <MenuItem value={ANY}>-</MenuItem>
                    {Object.values(EMatrixReason).map((reason) => (
                        <MenuItem key={reason} value={reason}>
                            {t(`approvalsScreen.matrix.reasons.${reason}`)}
                        </MenuItem>
                    ))}
                </TextField>
            </Box>
            {errors.map((error) => (
                <Alert key={error} severity="error" sx={{marginTop: "1rem"}}>
                    {t(`approvalsScreen.matrix.errors.${error}`)}
                </Alert>
            ))}
        </Dialog>
    )
}
