// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useId, useState} from "react"
import {useTranslation} from "react-i18next"
import {Box, Button, Drawer, IconButton, Menu, MenuItem, Radio, TextField} from "@mui/material"
import {styled} from "@mui/material/styles"
import AddIcon from "@mui/icons-material/Add"
import CloseIcon from "@mui/icons-material/Close"
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
    ruleSentence,
    validateRule,
} from "./approvalMatrix"
import {
    Help,
    Muted,
    Notice,
    OptionCard,
    OptionTitle,
    Overline,
    Segment,
    SegmentGroup,
} from "./approvalStyles"

const DECISIONS = [
    IApplicationsStatus.ACCEPTED,
    IApplicationsStatus.PENDING,
    IApplicationsStatus.REJECTED,
]

/** The errors that no later choice in the editor can fix, shown as soon as they appear. */
const NEVER_ALLOWED = [
    EMatrixError.ACCEPTS_MANUAL_ENTRY,
    EMatrixError.ACCEPTS_ALREADY_ENROLLED,
    EMatrixError.ACCEPTS_WITHOUT_VOTER,
    EMatrixError.OTHERWISE_ACCEPTS,
]

const Panel = styled("div")({
    display: "flex",
    flexDirection: "column",
    width: "min(720px, 100vw)",
    height: "100%",
})

const Header = styled("div")({
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "16px 24px",
    borderBottom: "1px solid #E3E7EF",
})

const Body = styled("div")({
    flexGrow: 1,
    overflowY: "auto",
    padding: "24px",
})

const Footer = styled("div")({
    display: "flex",
    justifyContent: "flex-end",
    gap: "12px",
    padding: "16px 24px",
    borderTop: "1px solid #E3E7EF",
})

const Heading = styled("h3")({
    margin: "24px 0 4px",
    fontSize: "16px",
    fontWeight: 600,
})

const ConditionRow = styled("div")({
    display: "flex",
    alignItems: "center",
    gap: "16px",
    padding: "12px 16px",
    marginBottom: "10px",
    border: "1px solid #E3E7EF",
    borderRadius: "10px",
})

const ConditionName = styled("span")({
    width: "164px",
    flexShrink: 0,
    fontSize: "14px",
    fontWeight: 600,
})

/** A condition of the editor: one of the fixed ones, or a compared field. */
type ConditionKey =
    | {kind: "identity" | "voter_found" | "already_enrolled" | "valid_id" | "differing"}
    | {kind: "field"; field: string}

const keyId = (key: ConditionKey): string =>
    key.kind === "field" ? `field:${key.field}` : key.kind

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
    const id = useId()
    const [draft, setDraft] = useState<IApprovalRule>(rule)
    const [tried, setTried] = useState(false)
    const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null)

    useEffect(() => {
        if (open) {
            setDraft(rule)
            setTried(false)
        }
    }, [open, rule])

    const edited: IApprovalRule = {
        when: isOtherwise ? {} : cleanConditions(draft.when, comparedFields),
        then:
            draft.then.decision === IApplicationsStatus.ACCEPTED
                ? {decision: draft.then.decision}
                : draft.then,
    }
    const found = validateRule(edited, isOtherwise)
    const blocking = found.filter((error) => NEVER_ALLOWED.includes(error))
    const shown = tried ? found : blocking

    const setCondition = (change: Partial<IRuleConditions>) =>
        setDraft((current) => ({...current, when: {...current.when, ...change}}))

    const setField = (field: string, match: EFieldMatch | undefined) => {
        const fields = {...draft.when.fields}
        if (match) {
            fields[field] = match
        } else {
            delete fields[field]
        }
        setCondition({fields})
    }

    const apply = () => {
        setTried(true)
        if (found.length === 0) {
            onClose(edited)
        }
    }

    const title = isOtherwise
        ? t("approvalsScreen.matrix.dialog.otherwiseTitle")
        : number === null
          ? t("approvalsScreen.matrix.dialog.newTitle")
          : t("approvalsScreen.matrix.dialog.editTitle", {number})

    const name = (key: ConditionKey): string =>
        key.kind === "field"
            ? fieldLabel(key.field)
            : String(
                  t(
                      {
                          identity: "approvalsScreen.matrix.dialog.identity",
                          voter_found: "approvalsScreen.matrix.dialog.voterFound",
                          already_enrolled: "approvalsScreen.matrix.dialog.alreadyEnrolled",
                          valid_id: "approvalsScreen.matrix.dialog.validId",
                          differing: "approvalsScreen.matrix.dialog.differing",
                      }[key.kind]
                  )
              )

    const all: ConditionKey[] = [
        {kind: "identity"},
        {kind: "voter_found"},
        {kind: "already_enrolled"},
        {kind: "valid_id"},
        {kind: "differing"},
        ...comparedFields.map((field): ConditionKey => ({kind: "field", field})),
    ]
    const isSet = (key: ConditionKey): boolean =>
        key.kind === "field"
            ? draft.when.fields?.[key.field] !== undefined
            : draft.when[key.kind] !== undefined
    const present = all.filter(isSet)
    const absent = all.filter((key) => !isSet(key))

    const add = (key: ConditionKey) => {
        setMenuAnchor(null)
        if (key.kind === "field") {
            setField(key.field, EFieldMatch.DIFFERS)
        } else if (key.kind === "identity") {
            setCondition({identity: EIdentityMethod.VERIFIED})
        } else if (key.kind === "valid_id") {
            setCondition({valid_id: validIds[0] ?? ""})
        } else if (key.kind === "differing") {
            setCondition({differing: EDifferingFields.EXACTLY_1})
        } else {
            setCondition({[key.kind]: true})
        }
    }

    const remove = (key: ConditionKey) =>
        key.kind === "field"
            ? setField(key.field, undefined)
            : setCondition({[key.kind]: undefined})

    const segments = <T,>(
        label: string,
        options: Array<{value: T; text: string}>,
        value: T | undefined,
        onChoose: (value: T) => void
    ) => (
        <SegmentGroup role="group" aria-label={label}>
            {options.map((option) => (
                <Segment
                    key={String(option.value)}
                    type="button"
                    aria-pressed={option.value === value}
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

    const control = (key: ConditionKey) => {
        const label = name(key)
        switch (key.kind) {
            case "identity":
                return segments(
                    label,
                    Object.values(EIdentityMethod).map((method) => ({
                        value: method,
                        text: String(t(`approvalsScreen.matrix.identity.${method}`)),
                    })),
                    draft.when.identity,
                    (identity) => setCondition({identity})
                )
            case "voter_found":
                return segments(label, yesNo, draft.when.voter_found, (voter_found) =>
                    setCondition({voter_found})
                )
            case "already_enrolled":
                return segments(label, yesNo, draft.when.already_enrolled, (already_enrolled) =>
                    setCondition({already_enrolled})
                )
            case "valid_id":
                return (
                    <TextField
                        select
                        fullWidth
                        size="small"
                        slotProps={{htmlInput: {"aria-label": label}}}
                        value={draft.when.valid_id ?? ""}
                        onChange={(event) => setCondition({valid_id: event.target.value})}
                    >
                        {[
                            ...validIds,
                            ...(draft.when.valid_id && !validIds.includes(draft.when.valid_id)
                                ? [draft.when.valid_id]
                                : []),
                        ].map((validId) => (
                            <MenuItem key={validId} value={validId}>
                                {t(validId)}
                            </MenuItem>
                        ))}
                    </TextField>
                )
            case "differing":
                return (
                    <TextField
                        select
                        fullWidth
                        size="small"
                        slotProps={{htmlInput: {"aria-label": label}}}
                        value={draft.when.differing ?? ""}
                        onChange={(event) =>
                            setCondition({
                                differing: Object.values(EDifferingFields).find(
                                    (count) => count === event.target.value
                                ),
                            })
                        }
                    >
                        {Object.values(EDifferingFields).map((count) => (
                            <MenuItem key={count} value={count}>
                                {t(`approvalsScreen.matrix.differing.${count}`)}
                            </MenuItem>
                        ))}
                    </TextField>
                )
            case "field":
                return segments(
                    label,
                    Object.values(EFieldMatch).map((match) => ({
                        value: match,
                        text: String(t(`approvalsScreen.matrix.fieldMatch.${match}`)),
                    })),
                    draft.when.fields?.[key.field],
                    (match) => setField(key.field, match)
                )
        }
    }

    const reasonLabel = String(t("approvalsScreen.matrix.dialog.reason"))

    return (
        <Drawer
            anchor="right"
            open={open}
            onClose={() => onClose(null)}
            // The drawer is the dialog: its title names it.
            slotProps={{paper: {"aria-labelledby": `${id}-title`}}}
        >
            <Panel>
                <Header>
                    <Box component="h2" id={`${id}-title`} sx={{margin: 0, fontSize: "22px"}}>
                        {title}
                    </Box>
                    <IconButton
                        aria-label={String(t("approvalsScreen.matrix.dialog.close"))}
                        onClick={() => onClose(null)}
                    >
                        <CloseIcon />
                    </IconButton>
                </Header>
                <Body>
                    <Notice data-tone="neutral">
                        <div>
                            <Overline sx={{margin: "0 0 4px"}}>
                                {t("approvalsScreen.matrix.dialog.summary")}
                            </Overline>
                            <Box sx={{fontSize: "16px"}} data-testid="rule-summary">
                                {ruleSentence(edited, isOtherwise, t, fieldLabel)}
                            </Box>
                        </div>
                    </Notice>

                    {!isOtherwise && (
                        <>
                            <Heading>{t("approvalsScreen.matrix.when")}</Heading>
                            <Help>{t("approvalsScreen.matrix.dialog.whenHelp")}</Help>
                            {present.map((key) => (
                                <ConditionRow key={keyId(key)}>
                                    <ConditionName>{name(key)}</ConditionName>
                                    <Box sx={{flexGrow: 1}}>{control(key)}</Box>
                                    <IconButton
                                        size="small"
                                        aria-label={String(
                                            t("approvalsScreen.matrix.dialog.remove", {
                                                condition: name(key),
                                            })
                                        )}
                                        onClick={() => remove(key)}
                                    >
                                        <CloseIcon fontSize="small" />
                                    </IconButton>
                                </ConditionRow>
                            ))}
                            <Button
                                variant="secondary"
                                startIcon={<AddIcon />}
                                disabled={absent.length === 0}
                                aria-haspopup="menu"
                                onClick={(event) => setMenuAnchor(event.currentTarget)}
                            >
                                {t("approvalsScreen.matrix.dialog.addCondition")}
                            </Button>
                            <Menu
                                anchorEl={menuAnchor}
                                open={menuAnchor !== null}
                                onClose={() => setMenuAnchor(null)}
                            >
                                {absent.map((key) => (
                                    <MenuItem key={keyId(key)} onClick={() => add(key)}>
                                        {name(key)}
                                    </MenuItem>
                                ))}
                            </Menu>
                        </>
                    )}

                    <Heading>
                        {isOtherwise
                            ? t("approvalsScreen.matrix.dialog.otherwiseHelp")
                            : t("approvalsScreen.matrix.then")}
                    </Heading>
                    <Box
                        role="radiogroup"
                        aria-label={String(t("approvalsScreen.matrix.dialog.decision"))}
                        sx={{display: "flex", flexDirection: "column", gap: "10px"}}
                    >
                        {DECISIONS.map((decision) => (
                            <OptionCard
                                key={decision}
                                data-selected={draft.then.decision === decision}
                            >
                                <Radio
                                    sx={{padding: "2px"}}
                                    name={`${id}-decision`}
                                    checked={draft.then.decision === decision}
                                    onChange={() =>
                                        setDraft((current) => ({
                                            ...current,
                                            // The reason is kept while trying Approve, which
                                            // has none; a rule that stops approving starts
                                            // from the reason the last rule gives.
                                            then: {
                                                decision,
                                                reason:
                                                    current.then.reason ??
                                                    (decision !== IApplicationsStatus.ACCEPTED &&
                                                    rule.then.decision ===
                                                        IApplicationsStatus.ACCEPTED
                                                        ? EMatrixReason.NO_VOTER
                                                        : undefined),
                                            },
                                        }))
                                    }
                                />
                                <div>
                                    <OptionTitle>
                                        {t(`approvalsScreen.matrix.decisions.${decision}`)}
                                    </OptionTitle>
                                    <Muted>
                                        {t(`approvalsScreen.matrix.outcomeHelp.${decision}`)}
                                    </Muted>
                                </div>
                            </OptionCard>
                        ))}
                    </Box>

                    {draft.then.decision !== IApplicationsStatus.ACCEPTED && (
                        <>
                            <Heading>{reasonLabel}</Heading>
                            <TextField
                                select
                                fullWidth
                                size="small"
                                slotProps={{
                                    select: {displayEmpty: true},
                                    htmlInput: {"aria-label": reasonLabel},
                                }}
                                value={draft.then.reason ?? ""}
                                onChange={(event) =>
                                    setDraft((current) => ({
                                        ...current,
                                        then: {
                                            decision: current.then.decision,
                                            reason: Object.values(EMatrixReason).find(
                                                (reason) => reason === event.target.value
                                            ),
                                        },
                                    }))
                                }
                            >
                                <MenuItem value="">-</MenuItem>
                                {Object.values(EMatrixReason).map((reason) => (
                                    <MenuItem key={reason} value={reason}>
                                        {t(`approvalsScreen.matrix.reasons.${reason}`)}
                                    </MenuItem>
                                ))}
                            </TextField>
                            {draft.then.reason && (
                                <Notice data-tone="neutral" sx={{marginTop: "12px"}}>
                                    <div>
                                        <Overline sx={{margin: "0 0 4px"}}>
                                            {t("approvalsScreen.matrix.dialog.voterSees")}
                                        </Overline>
                                        {t(`approvalsScreen.matrix.voterText.${draft.then.reason}`)}
                                    </div>
                                </Notice>
                            )}
                        </>
                    )}

                    {shown.map((error) => (
                        <Notice key={error} data-tone="error" role="alert" sx={{marginTop: "16px"}}>
                            {t(`approvalsScreen.matrix.errors.${error}`)}
                        </Notice>
                    ))}
                </Body>
                <Footer>
                    <Button variant="cancel" onClick={() => onClose(null)}>
                        {t("common.label.cancel")}
                    </Button>
                    <Button disabled={blocking.length > 0} onClick={apply}>
                        {t("approvalsScreen.matrix.dialog.apply")}
                    </Button>
                </Footer>
            </Panel>
        </Drawer>
    )
}
