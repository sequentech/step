// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useMemo, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {useQuery} from "@apollo/client"
import {Box, Button, MenuItem, TextField} from "@mui/material"
import {styled} from "@mui/material/styles"
import ArrowBackIcon from "@mui/icons-material/ArrowBack"
import EditIcon from "@mui/icons-material/Edit"
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
} from "./approvalMatrix"
import {
    EFieldState,
    EMatrixEntry,
    IRecordedDecision,
    customizedSample,
    enrollmentFor,
    fieldState,
    historyComparison,
    sameEnrollment,
    sampleEnrollment,
} from "./approvalExample"
import {CARD_PADDING, Help, Muted, Notice, Overline, Segment, SegmentGroup} from "./approvalStyles"

const NOT_REPORTED = ""
const FIRST_NAME = "firstName"
const KEY = "approvalsScreen.matrix.example"
/**
 * The room the matrix screen's sticky save bar takes at the bottom of the window, as
 * measured on screen; none while there is no save bar.
 */
export const SAVE_BAR_ROOM = "--approval-matrix-save-bar"

const TONE: Record<string, string> = {
    [IApplicationsStatus.ACCEPTED]: "success",
    [IApplicationsStatus.PENDING]: "review",
    [IApplicationsStatus.REJECTED]: "error",
}

const MATCH: Partial<Record<EFieldState, EFieldMatch>> = {
    [EFieldState.MATCHES]: EFieldMatch.MATCHES,
    [EFieldState.DIFFERS]: EFieldMatch.DIFFERS,
}

const isStatus = (value: unknown): value is IApplicationsStatus =>
    Object.values(IApplicationsStatus).some((status) => status === value)

/** Where the decision of the applied example stands. */
export enum EExampleResult {
    PENDING = "PENDING",
    ERROR = "ERROR",
    INVALID = "INVALID",
    DECIDED = "DECIDED",
}

export type ExampleResult = EvaluateApprovalMatrixQuery["evaluate_approval_matrix"]

export interface IExampleEvaluation {
    state: EExampleResult
    result?: ExampleResult
}

/**
 * Decides the applied example with the rules on screen. The result belongs to
 * the current rules and example only: while another request is on its way,
 * or none can be made, there is no decision.
 */
export function useExampleEvaluation(
    electionEventId: string,
    matrix: IApprovalMatrix | null,
    applied: ITestEnrollment,
    entry: EMatrixEntry,
    joint: boolean
): IExampleEvaluation {
    const tested = useMemo(() => (matrix ? cleanMatrix(matrix) : null), [matrix])
    const enrollment = useMemo(
        () => enrollmentFor(applied, tested?.compared_fields ?? [], entry, joint),
        [applied, tested, entry, joint]
    )
    const {data, loading, error} = useQuery<EvaluateApprovalMatrixQuery>(EVALUATE_APPROVAL_MATRIX, {
        variables: {electionEventId, matrix: tested, enrollment},
        skip: !tested,
        fetchPolicy: "no-cache",
    })
    const result = data?.evaluate_approval_matrix
    if (!tested || loading) {
        return {state: EExampleResult.PENDING}
    }
    if (error) {
        return {state: EExampleResult.ERROR}
    }
    if (!result) {
        return {state: EExampleResult.PENDING}
    }
    if (result.errors.length > 0) {
        return {state: EExampleResult.INVALID, result}
    }
    return isStatus(result.decision)
        ? {state: EExampleResult.DECIDED, result}
        : {state: EExampleResult.ERROR}
}

/** The rule that decides the applied example: a position from 1, empty for the last rule, or none. */
export const decidingRule = (evaluation: IExampleEvaluation): number | null | undefined =>
    evaluation.state === EExampleResult.DECIDED ? (evaluation.result?.rule ?? null) : undefined

/** The enrollment the example was taken from. */
export interface IExampleSource {
    name: string
    /** The application's status now. */
    status?: string | null
    recorded: IRecordedDecision | null
    onBack: () => void
}

const Section = styled("div")({
    "padding": "16px 0",
    "borderTop": "1px solid #E3E7EF",
    "overflowWrap": "anywhere",
    "&:first-of-type": {paddingTop: 0, borderTop: 0},
})

const Summary = styled("dl")({
    "display": "grid",
    "gridTemplateColumns": "minmax(0, 1fr) minmax(0, 1fr)",
    "gap": "6px 12px",
    "margin": "8px 0 0",
    "fontSize": "14px",
    "& dt": {color: "#5A6070"},
    "& dd": {margin: 0, fontWeight: 600, overflowWrap: "anywhere"},
})

const Draft = styled("div")(({theme}) => ({
    marginTop: "16px",
    padding: "16px",
    border: "2px dashed #9AA3B8",
    borderRadius: "10px",
    background: "#FAFBFD",
    [theme.breakpoints.down("sm")]: {padding: "12px"},
}))

// While the card scrolls on wide screens, the editor keeps saying that it isn't applied.
const DraftHeading = styled("div")(({theme}) => ({
    margin: "-16px -16px 12px",
    padding: "12px 16px",
    [theme.breakpoints.down("sm")]: {margin: "-12px -12px 12px", padding: "12px"},
    borderRadius: "8px 8px 0 0",
    background: "#EEF0F6",
    // Flush with the top of the card's scrolling area: the card's own padding is above it.
    [theme.breakpoints.up("lg")]: {position: "sticky", top: `-${CARD_PADDING}px`, zIndex: 1},
}))

const FieldRow = styled(Box)({
    "display": "flex",
    "flexWrap": "wrap",
    "alignItems": "center",
    "justifyContent": "space-between",
    "gap": "8px",
    "fontSize": "14px",
    "& [role='group'] button": {whiteSpace: "nowrap", padding: "6px 10px"},
})

export interface ApprovalMatrixTestProps {
    /** The example the rules on screen decide. */
    applied: ITestEnrollment
    onApply: (enrollment: ITestEnrollment) => void
    /** How details without a result are read: as the sample's, or as an enrollment's. */
    entry: EMatrixEntry
    /** Whether the example's ID gives first and middle name together. */
    joint: boolean
    /** The enrollment the screen was opened from. */
    source?: IExampleSource
    comparedFields: string[]
    validIds: string[]
    fieldLabel: FieldLabel
    evaluation: IExampleEvaluation
    /** Whether the rules on screen include unsaved changes. */
    unsavedRules: boolean
    onGoToRule?: (rule: number | null) => void
}

export const ApprovalMatrixTest: React.FC<ApprovalMatrixTestProps> = ({
    applied,
    onApply,
    entry,
    joint,
    source,
    comparedFields,
    validIds,
    fieldLabel,
    evaluation,
    unsavedRules,
    onGoToRule,
}) => {
    const {t} = useTranslation()
    const [draft, setDraft] = useState<ITestEnrollment | null>(null)
    const editButton = useRef<HTMLButtonElement>(null)
    const appliedSection = useRef<HTMLDivElement>(null)
    // The sticky heading's height, so a focused control is never scrolled in under it.
    const [headingHeight, setHeadingHeight] = useState(0)
    const heading = useCallback((element: HTMLDivElement | null) => {
        if (!element) {
            return
        }
        const measure = () => setHeadingHeight(element.offsetHeight)
        measure()
        const observer = new ResizeObserver(measure)
        observer.observe(element)
        return () => {
            observer.disconnect()
            setHeadingHeight(0)
        }
    }, [])
    const firstControl = useRef<HTMLDivElement>(null)

    const recorded = source?.recorded ?? null
    const result = evaluation.state === EExampleResult.DECIDED ? evaluation.result : undefined
    const decision = isStatus(result?.decision) ? result.decision : null
    const comparison =
        recorded && entry === EMatrixEntry.ENROLLMENT
            ? historyComparison(recorded, applied, comparedFields, decision)
            : null

    const yesNo = (value: boolean) => t(`approvalsScreen.matrix.dialog.${value ? "yes" : "no"}`)
    const identityText = (identity: EIdentityMethod | null) =>
        identity
            ? t(`approvalsScreen.matrix.identity.${identity}`)
            : t("approvalsScreen.matrix.dialog.notReported")
    const idText = (id: string | null) =>
        id === null
            ? t("approvalsScreen.matrix.dialog.notReported")
            : validIds.includes(id)
              ? t(id)
              : t(`${KEY}.noLongerOffered`, {id: t(id)})
    const stateText = (state: EFieldState) => {
        if (state === EFieldState.WITH_FIRST_NAME) {
            return comparedFields.includes(FIRST_NAME)
                ? t(`${KEY}.withFirstName`)
                : t(`${KEY}.notComparedForId`)
        }
        return state === EFieldState.NOT_RECORDED
            ? t(`${KEY}.notRecorded`)
            : t(`approvalsScreen.matrix.fieldMatch.${state}`)
    }
    const stateOf = (enrollment: ITestEnrollment, field: string) =>
        fieldState(enrollment, field, entry, joint)

    const startEditing = () => {
        setDraft(applied)
        requestAnimationFrame(() => firstControl.current?.querySelector("button")?.focus())
    }
    const stopEditing = (apply: boolean) => {
        if (apply && draft) {
            onApply(draft)
        }
        setDraft(null)
        requestAnimationFrame(() => {
            // The applied example and its result come back into view where the editor was.
            appliedSection.current?.scrollIntoView({block: "nearest"})
            editButton.current?.focus({preventScroll: true})
        })
    }

    const choice = <T,>(
        label: string,
        options: Array<{value: T; text: string}>,
        value: T | undefined,
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

    const yesNoOptions = [
        {value: true, text: yesNo(true)},
        {value: false, text: yesNo(false)},
    ]
    const identityLabel = String(t("approvalsScreen.matrix.dialog.identity"))
    const voterFoundLabel = String(t("approvalsScreen.matrix.dialog.voterFound"))
    const alreadyEnrolledLabel = String(t("approvalsScreen.matrix.dialog.alreadyEnrolled"))
    const validIdLabel = String(t("approvalsScreen.matrix.dialog.validId"))

    const recordedLine = (value: IRecordedDecision, status?: string | null) => {
        if (!value.enrollment || !value.decision) {
            return (
                <Notice data-tone="warning" sx={{marginTop: "8px", padding: "8px 12px"}}>
                    {t(`${KEY}.unreadable`)}
                </Notice>
            )
        }
        const outcome = t(`approvalsScreen.matrix.decisions.${value.decision}`)
        return (
            <Box sx={{marginTop: "4px", fontSize: "14px"}}>
                <Muted>{t(`${KEY}.recorded`)}</Muted>{" "}
                {value.rule === null
                    ? t(`${KEY}.recordedLast`, {version: value.matrixVersion, decision: outcome})
                    : t(`${KEY}.recordedRule`, {
                          number: value.rule,
                          version: value.matrixVersion,
                          decision: outcome,
                      })}
                {value.invariant && (
                    <> {t(`approvalsScreen.matrix.invariants.${value.invariant}`)}</>
                )}
                {(value.approvedVoters ?? 0) > 1 && <> {t(`${KEY}.severalApproved`)}</>}
                {isStatus(status) && status !== value.decision && (
                    <> {t(`${KEY}.statusNow`, {status: t(`approvalsScreen.status.${status}`)})}</>
                )}
            </Box>
        )
    }

    const origin = () => {
        if (source) {
            return (
                <>
                    <Button
                        variant="secondary"
                        size="small"
                        startIcon={<ArrowBackIcon />}
                        onClick={source.onBack}
                        sx={{
                            maxWidth: "100%",
                            minHeight: 0,
                            padding: "6px 12px",
                            whiteSpace: "normal",
                            overflowWrap: "anywhere",
                            textAlign: "left",
                            textTransform: "none",
                            letterSpacing: "normal",
                            fontSize: "14px",
                            lineHeight: "20px",
                            marginBottom: "12px",
                        }}
                    >
                        {source.name
                            ? t(`${KEY}.back`, {name: source.name})
                            : t(`${KEY}.backUnnamed`)}
                    </Button>
                    <Box sx={{fontWeight: 600}}>
                        {source.name
                            ? t(`${KEY}.fromEnrollment`, {name: source.name})
                            : t(`${KEY}.fromUnnamed`)}
                    </Box>
                    {recorded ? (
                        recordedLine(recorded, source.status)
                    ) : (
                        <Notice data-tone="warning" sx={{marginTop: "8px", padding: "8px 12px"}}>
                            {t(`${KEY}.unreadable`)}
                        </Notice>
                    )}
                </>
            )
        }
        const customized = customizedSample(applied, comparedFields)
        return (
            <>
                <Box sx={{fontWeight: 600}}>
                    {t(`${KEY}.${customized ? "customTitle" : "sampleTitle"}`)}
                </Box>
                <Muted>{t(`${KEY}.${customized ? "customText" : "sampleText"}`)}</Muted>
            </>
        )
    }

    const summary = (
        <Summary>
            <dt>{identityLabel}</dt>
            <dd>{identityText(applied.identity)}</dd>
            <dt>{voterFoundLabel}</dt>
            <dd>{yesNo(applied.voter_found)}</dd>
            <dt>{alreadyEnrolledLabel}</dt>
            <dd>{yesNo(applied.voter_found && applied.already_enrolled)}</dd>
            <dt>{validIdLabel}</dt>
            <dd>{idText(applied.valid_id)}</dd>
            {applied.voter_found &&
                comparedFields.map((field) => (
                    <React.Fragment key={field}>
                        <dt>{fieldLabel(field)}</dt>
                        <dd>{stateText(stateOf(applied, field))}</dd>
                    </React.Fragment>
                ))}
        </Summary>
    )

    const fieldRow = (value: ITestEnrollment, field: string) => {
        const state = stateOf(value, field)
        return (
            <FieldRow key={field}>
                <span>
                    {fieldLabel(field)}
                    {state === EFieldState.NOT_RECORDED && (
                        <Muted> · {t(`${KEY}.notRecorded`)}</Muted>
                    )}
                </span>
                {state === EFieldState.WITH_FIRST_NAME ? (
                    <Muted>{stateText(state)}</Muted>
                ) : (
                    choice(
                        fieldLabel(field),
                        Object.values(EFieldMatch).map((match) => ({
                            value: match,
                            text: String(t(`approvalsScreen.matrix.fieldMatch.${match}`)),
                        })),
                        MATCH[state],
                        (match) => setDraft({...value, fields: {...value.fields, [field]: match}})
                    )
                )}
            </FieldRow>
        )
    }

    const editor = (value: ITestEnrollment) => {
        const ids =
            value.valid_id && !validIds.includes(value.valid_id)
                ? [...validIds, value.valid_id]
                : validIds
        const restores = entry === EMatrixEntry.ENROLLMENT && recorded?.enrollment
        return (
            <Draft
                role="group"
                aria-labelledby="approval-matrix-draft"
                sx={{
                    // Moving through the editor keeps each control clear of the save bar and of
                    // the editor's sticky heading.
                    "& button, & [role='combobox']": {
                        scrollMarginBottom: `calc(16px + var(${SAVE_BAR_ROOM}, 0px))`,
                        scrollMarginTop: {lg: `${headingHeight + 16}px`},
                    },
                }}
            >
                <DraftHeading ref={heading}>
                    <Overline id="approval-matrix-draft" sx={{margin: 0}}>
                        {t(`${KEY}.editing`)}
                    </Overline>
                    {decidingRule(evaluation) !== undefined && (
                        <Box sx={{fontSize: "13px"}}>{t(`${KEY}.stillApplied`)}</Box>
                    )}
                </DraftHeading>
                <Help>{t(`${KEY}.editingHelp`)}</Help>
                <div ref={firstControl}>
                    <Overline>{identityLabel}</Overline>
                    {choice<EIdentityMethod | null>(
                        identityLabel,
                        [
                            ...Object.values(EIdentityMethod).map((method) => ({
                                value: method,
                                text: String(t(`approvalsScreen.matrix.identity.${method}`)),
                            })),
                            {
                                value: null,
                                text: String(t("approvalsScreen.matrix.dialog.notReported")),
                            },
                        ],
                        value.identity,
                        (identity) => setDraft({...value, identity})
                    )}
                </div>
                <Overline>{voterFoundLabel}</Overline>
                {choice(voterFoundLabel, yesNoOptions, value.voter_found, (voter_found) =>
                    setDraft({...value, voter_found})
                )}
                <Overline>{alreadyEnrolledLabel}</Overline>
                {choice(
                    alreadyEnrolledLabel,
                    yesNoOptions,
                    value.voter_found && value.already_enrolled,
                    (already_enrolled) => setDraft({...value, already_enrolled}),
                    !value.voter_found
                )}
                <Overline>{validIdLabel}</Overline>
                <TextField
                    select
                    fullWidth
                    size="small"
                    slotProps={{
                        select: {displayEmpty: true},
                        htmlInput: {"aria-label": validIdLabel},
                    }}
                    value={value.valid_id ?? NOT_REPORTED}
                    onChange={(event) => setDraft({...value, valid_id: event.target.value || null})}
                >
                    <MenuItem value={NOT_REPORTED}>
                        {t("approvalsScreen.matrix.dialog.notReported")}
                    </MenuItem>
                    {ids.map((id) => (
                        <MenuItem key={id} value={id}>
                            {idText(id)}
                        </MenuItem>
                    ))}
                </TextField>
                {value.voter_found && comparedFields.length > 0 && (
                    <>
                        <Overline>{t("approvalsScreen.matrix.testDetails")}</Overline>
                        <Box sx={{display: "flex", flexDirection: "column", gap: "8px"}}>
                            {comparedFields.map((field) => fieldRow(value, field))}
                        </Box>
                    </>
                )}
                <Box
                    sx={{
                        "display": "flex",
                        "flexWrap": "wrap",
                        "gap": "8px",
                        "justifyContent": "flex-end",
                        "marginTop": "20px",
                        "& button": {whiteSpace: "nowrap"},
                        "& button:first-of-type": {whiteSpace: "normal", textAlign: "left"},
                    }}
                >
                    <Button
                        variant="secondary"
                        size="small"
                        sx={{marginRight: "auto"}}
                        onClick={() =>
                            setDraft(restores ? recorded.enrollment : sampleEnrollment())
                        }
                    >
                        {t(`${KEY}.${restores ? "restore" : "resetSample"}`)}
                    </Button>
                    <Button variant="cancel" size="small" onClick={() => stopEditing(false)}>
                        {t("common.label.cancel")}
                    </Button>
                    <Button
                        size="small"
                        disabled={sameEnrollment(value, applied)}
                        onClick={() => stopEditing(true)}
                    >
                        {t(`${KEY}.apply`)}
                    </Button>
                </Box>
            </Draft>
        )
    }

    const outcome = () => {
        if (evaluation.state === EExampleResult.PENDING) {
            return <Muted>{t(`${KEY}.evaluating`)}</Muted>
        }
        if (evaluation.state === EExampleResult.ERROR) {
            return <Notice data-tone="error">{t("approvalsScreen.matrix.testError")}</Notice>
        }
        if (evaluation.state === EExampleResult.INVALID) {
            return (
                <Notice data-tone="warning">
                    <div>
                        {t("approvalsScreen.matrix.testInvalid")}
                        {evaluation.result?.errors.map((problem, index) => {
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
            )
        }
        if (!result || !decision) {
            return null
        }
        return (
            <Notice data-tone={TONE[decision] ?? "neutral"}>
                <div>
                    <Box sx={{fontWeight: 600, marginBottom: "8px"}}>
                        {result.rule
                            ? t(`${KEY}.decides`, {number: result.rule})
                            : t(`${KEY}.lastDecides`)}
                    </Box>
                    <ApprovalOutcomeChip decision={decision} />
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
        )
    }

    return (
        <>
            <Section>
                {origin()}
                <Help sx={{margin: "8px 0 0"}}>{t(`${KEY}.noChanges`)}</Help>
            </Section>
            <Section ref={appliedSection}>
                <Box
                    sx={{
                        display: "flex",
                        flexWrap: "wrap",
                        alignItems: "center",
                        justifyContent: "space-between",
                        gap: "8px",
                    }}
                >
                    <Overline sx={{margin: 0}}>
                        {t(`${KEY}.${draft ? "appliedInUse" : "applied"}`)}
                    </Overline>
                    {!draft && (
                        <Button
                            ref={editButton}
                            variant="secondary"
                            size="small"
                            startIcon={<EditIcon />}
                            onClick={startEditing}
                        >
                            {t(`${KEY}.edit`)}
                        </Button>
                    )}
                </Box>
                {summary}
            </Section>
            <Section>
                <Overline sx={{margin: 0}}>{t(`${KEY}.simulation`)}</Overline>
                <Muted>{t(`${KEY}.${unsavedRules ? "rulesUnsaved" : "rulesOnScreen"}`)}</Muted>
                <Box
                    role="status"
                    aria-label={String(t(`${KEY}.simulation`))}
                    sx={{marginTop: "12px"}}
                >
                    {outcome()}
                </Box>
                {result && onGoToRule && (
                    <Button
                        variant="secondary"
                        size="small"
                        sx={{marginTop: "12px"}}
                        onClick={() => onGoToRule(result.rule ?? null)}
                    >
                        {result.rule
                            ? t(`${KEY}.goToRule`, {number: result.rule})
                            : t(`${KEY}.goToLast`)}
                    </Button>
                )}
                {comparison && (
                    <Box sx={{marginTop: "12px", fontSize: "14px", color: "#5A6070"}}>
                        {t(`${KEY}.comparison.${comparison}`)}
                    </Box>
                )}
                {draft && editor(draft)}
            </Section>
        </>
    )
}
