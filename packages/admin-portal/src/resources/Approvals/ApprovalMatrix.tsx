// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useId, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useNotify} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {
    Accordion,
    AccordionSummary,
    Alert,
    Autocomplete,
    Button,
    Chip,
    CircularProgress,
    TextField,
    Typography,
} from "@mui/material"
import ArrowBackIosIcon from "@mui/icons-material/ArrowBackIos"
import {Dialog} from "@sequentech/ui-essentials"
import {
    GetApprovalMatrixQuery,
    GetUserProfileAttributesQuery,
    SaveApprovalMatrixMutation,
} from "@/gql/graphql"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {GET_APPROVAL_MATRIX} from "@/queries/GetApprovalMatrix"
import {SAVE_APPROVAL_MATRIX} from "@/queries/SaveApprovalMatrix"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {getAttributeLabel} from "@/services/UserService"
import {IApplicationsStatus} from "@/types/applications"
import {IPermissions} from "@/types/keycloak"
import {CancelButton} from "../Tally/styles"
import {ApprovalMatrixRuleDialog} from "./ApprovalMatrixRuleDialog"
import {ApprovalMatrixRules} from "./ApprovalMatrixRules"
import {ApprovalMatrixTest} from "./ApprovalMatrixTest"
import {convertToCamelCase} from "./UtilsApprovals"
import {
    EMatrixSource,
    IApprovalMatrix,
    IApprovalRule,
    addRule,
    cleanMatrix,
    deleteRule,
    moveRule,
    profileFieldLabel,
    readMatrix,
    replaceRule,
    sameMatrix,
    validateMatrix,
    withComparedFields,
} from "./approvalMatrix"

/** The rule being edited: a position, the last rule, or a new rule. */
type Editing = {index: number | null; isOtherwise: boolean; rule: IApprovalRule}

const NEW_RULE: IApprovalRule = {when: {}, then: {decision: IApplicationsStatus.PENDING}}

export interface ApprovalMatrixProps {
    electionEventId: string
    goBack: () => void
}

export const ApprovalMatrix: React.FC<ApprovalMatrixProps> = ({electionEventId, goBack}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const id = useId()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const canEdit = authContext.isAuthorized(true, tenantId, IPermissions.APPROVAL_MATRIX_WRITE)

    const {data, loading, error} = useQuery<GetApprovalMatrixQuery>(GET_APPROVAL_MATRIX, {
        variables: {electionEventId},
        fetchPolicy: "network-only",
    })
    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {variables: {tenantId, electionEventId}}
    )
    const [saveMatrix, {loading: saving}] =
        useMutation<SaveApprovalMatrixMutation>(SAVE_APPROVAL_MATRIX)

    const [current, setCurrent] = useState<GetApprovalMatrixQuery["get_approval_matrix"] | null>(
        null
    )
    const [draft, setDraft] = useState<IApprovalMatrix | null>(null)
    const [editing, setEditing] = useState<Editing | null>(null)
    const [confirmSave, setConfirmSave] = useState(false)

    useEffect(() => {
        if (data?.get_approval_matrix) {
            setCurrent(data.get_approval_matrix)
            setDraft(readMatrix(data.get_approval_matrix.matrix))
        }
    }, [data])

    const saved = useMemo(() => (current ? readMatrix(current.matrix) : null), [current])
    const profileAttributes = userAttributes?.get_user_profile_attributes
    const fieldLabel = useMemo(
        () => profileFieldLabel(profileAttributes ?? [], t, getAttributeLabel),
        [profileAttributes, t]
    )
    const fieldOptions = useMemo(
        () =>
            (profileAttributes ?? [])
                .map((attribute) => convertToCamelCase(attribute.name ?? ""))
                .filter((name) => name),
        [profileAttributes]
    )

    if (loading) {
        return <CircularProgress aria-label={String(t("approvalsScreen.matrix.title"))} />
    }

    const footer = (save?: React.ReactNode) => (
        <WizardStyles.FooterContainer>
            <WizardStyles.StyledFooter>
                <CancelButton className="list-actions" onClick={goBack}>
                    <ArrowBackIosIcon />
                    {t("common.label.back")}
                </CancelButton>
                {save}
            </WizardStyles.StyledFooter>
        </WizardStyles.FooterContainer>
    )

    if (error || !current || !draft || !saved) {
        return (
            <WizardStyles.WizardContainer>
                <WizardStyles.ContentWrapper>
                    <Alert severity="error">{t("approvalsScreen.matrix.loadError")}</Alert>
                </WizardStyles.ContentWrapper>
                {footer()}
            </WizardStyles.WizardContainer>
        )
    }

    const changed = !sameMatrix(draft, saved)
    const problems = validateMatrix(draft)

    const applyRule = (rule: IApprovalRule | null) => {
        if (rule && editing) {
            setDraft(
                editing.isOtherwise
                    ? {...draft, otherwise: rule.then}
                    : editing.index === null
                      ? addRule(draft, rule)
                      : replaceRule(draft, editing.index, rule)
            )
        }
        setEditing(null)
    }

    const save = async () => {
        setConfirmSave(false)
        try {
            const {data: result} = await saveMatrix({
                variables: {electionEventId, matrix: cleanMatrix(draft)},
            })
            const savedVersion = result?.save_approval_matrix
            const matrix = readMatrix(savedVersion?.matrix)
            if (!savedVersion || !matrix) {
                notify(t("approvalsScreen.matrix.save.error"), {type: "error"})
                return
            }
            setCurrent(savedVersion)
            setDraft(matrix)
            notify(t("approvalsScreen.matrix.save.success", {version: savedVersion.version}), {
                type: "success",
            })
        } catch {
            notify(t("approvalsScreen.matrix.save.error"), {type: "error"})
        }
    }

    return (
        <WizardStyles.WizardContainer>
            <WizardStyles.ContentWrapper>
                <WizardStyles.ContentBox>
                    <Accordion sx={{width: "100%"}} expanded={true}>
                        <AccordionSummary
                            expandIcon={false}
                            id={`${id}-title`}
                            aria-controls={`${id}-title-content`}
                        >
                            <WizardStyles.AccordionTitle>
                                {t("approvalsScreen.matrix.title")}
                            </WizardStyles.AccordionTitle>
                        </AccordionSummary>
                        <WizardStyles.AccordionDetails>
                            <Typography variant="body2">
                                {t("approvalsScreen.matrix.subtitle")}
                            </Typography>
                            <Typography variant="body2" sx={{marginTop: "1rem"}}>
                                {current.source === EMatrixSource.SAVED
                                    ? t("approvalsScreen.matrix.version", {
                                          version: current.version,
                                          date: current.created_at
                                              ? new Date(current.created_at).toLocaleString()
                                              : "-",
                                          user: current.created_by ?? "-",
                                      })
                                    : t("approvalsScreen.matrix.builtInVersion", {
                                          version: current.version,
                                      })}
                                {changed && (
                                    <Chip
                                        size="small"
                                        sx={{marginLeft: "8px"}}
                                        label={t("approvalsScreen.matrix.unsaved")}
                                    />
                                )}
                            </Typography>
                            {!canEdit && (
                                <Alert severity="info" sx={{marginTop: "1rem"}}>
                                    {t("approvalsScreen.matrix.readOnly")}
                                </Alert>
                            )}
                        </WizardStyles.AccordionDetails>
                    </Accordion>

                    <Accordion sx={{width: "100%"}} expanded={true}>
                        <AccordionSummary
                            expandIcon={false}
                            id={`${id}-compared`}
                            aria-controls={`${id}-compared-content`}
                        >
                            <WizardStyles.AccordionTitle>
                                {t("approvalsScreen.matrix.compared")}
                            </WizardStyles.AccordionTitle>
                        </AccordionSummary>
                        <WizardStyles.AccordionDetails>
                            {canEdit ? (
                                <Autocomplete
                                    multiple
                                    freeSolo
                                    options={fieldOptions}
                                    value={draft.compared_fields}
                                    getOptionLabel={fieldLabel}
                                    onChange={(_, fields) =>
                                        setDraft(withComparedFields(draft, fields))
                                    }
                                    renderInput={(params) => (
                                        <TextField
                                            {...params}
                                            label={t("approvalsScreen.matrix.comparedFields")}
                                            helperText={t(
                                                "approvalsScreen.matrix.comparedFieldsHelp"
                                            )}
                                        />
                                    )}
                                />
                            ) : (
                                <Typography>
                                    {draft.compared_fields.map(fieldLabel).join(", ")}
                                </Typography>
                            )}
                            <Typography variant="body2" sx={{marginTop: "1rem"}}>
                                {t("approvalsScreen.matrix.comparedHelp")}
                            </Typography>
                        </WizardStyles.AccordionDetails>
                    </Accordion>

                    <Accordion sx={{width: "100%"}} expanded={true}>
                        <AccordionSummary
                            expandIcon={false}
                            id={`${id}-rules`}
                            aria-controls={`${id}-rules-content`}
                        >
                            <WizardStyles.AccordionTitle>
                                {t("approvalsScreen.matrix.rules")}
                            </WizardStyles.AccordionTitle>
                        </AccordionSummary>
                        <WizardStyles.AccordionDetails>
                            <ApprovalMatrixRules
                                matrix={draft}
                                canEdit={canEdit}
                                fieldLabel={fieldLabel}
                                onEdit={(index) =>
                                    setEditing(
                                        index === null
                                            ? {
                                                  index,
                                                  isOtherwise: true,
                                                  rule: {when: {}, then: draft.otherwise},
                                              }
                                            : {index, isOtherwise: false, rule: draft.rules[index]}
                                    )
                                }
                                onMove={(index, offset) => setDraft(moveRule(draft, index, offset))}
                                onDelete={(index) => setDraft(deleteRule(draft, index))}
                                onAdd={() =>
                                    setEditing({index: null, isOtherwise: false, rule: NEW_RULE})
                                }
                            />
                            {problems.length > 0 && (
                                <Alert severity="warning" sx={{marginTop: "1rem"}}>
                                    {problems.map(({code, rule}) => {
                                        const text = t(`approvalsScreen.matrix.errors.${code}`)
                                        return (
                                            <div key={`${rule}-${code}`}>
                                                {rule
                                                    ? t("approvalsScreen.matrix.ruleError", {
                                                          number: rule,
                                                          error: text,
                                                      })
                                                    : text}
                                            </div>
                                        )
                                    })}
                                </Alert>
                            )}
                        </WizardStyles.AccordionDetails>
                    </Accordion>

                    <Accordion sx={{width: "100%"}} expanded={true}>
                        <AccordionSummary
                            expandIcon={false}
                            id={`${id}-test`}
                            aria-controls={`${id}-test-content`}
                        >
                            <WizardStyles.AccordionTitle>
                                {t("approvalsScreen.matrix.test")}
                            </WizardStyles.AccordionTitle>
                        </AccordionSummary>
                        <WizardStyles.AccordionDetails>
                            <ApprovalMatrixTest
                                electionEventId={electionEventId}
                                matrix={draft}
                                validIds={current.valid_ids}
                                fieldLabel={fieldLabel}
                            />
                        </WizardStyles.AccordionDetails>
                    </Accordion>
                </WizardStyles.ContentBox>
            </WizardStyles.ContentWrapper>

            {footer(
                canEdit && (
                    <Button
                        disabled={!changed || problems.length > 0 || saving}
                        onClick={() => setConfirmSave(true)}
                    >
                        {t("approvalsScreen.matrix.save.button")}
                    </Button>
                )
            )}

            {editing && (
                <ApprovalMatrixRuleDialog
                    open={true}
                    rule={editing.rule}
                    number={editing.index === null ? null : editing.index + 1}
                    isOtherwise={editing.isOtherwise}
                    comparedFields={draft.compared_fields}
                    validIds={current.valid_ids}
                    fieldLabel={fieldLabel}
                    onClose={applyRule}
                />
            )}

            <Dialog
                variant="info"
                open={confirmSave}
                title={String(t("approvalsScreen.matrix.save.title"))}
                ok={String(t("approvalsScreen.matrix.save.button"))}
                cancel={String(t("common.label.cancel"))}
                handleClose={async (result: boolean) => {
                    if (result) {
                        await save()
                    } else {
                        setConfirmSave(false)
                    }
                }}
            >
                {t("approvalsScreen.matrix.save.body", {version: current.next_version})}
            </Dialog>
        </WizardStyles.WizardContainer>
    )
}
