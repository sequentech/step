// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useNotify} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {Box, Button, CircularProgress, Menu, MenuItem} from "@mui/material"
import {styled} from "@mui/material/styles"
import AddIcon from "@mui/icons-material/Add"
import ArrowBackIcon from "@mui/icons-material/ArrowBack"
import CheckIcon from "@mui/icons-material/Check"
import FactCheckIcon from "@mui/icons-material/FactCheck"
import LockOutlinedIcon from "@mui/icons-material/LockOutlined"
import RuleIcon from "@mui/icons-material/Rule"
import {Dialog} from "@sequentech/ui-essentials"
import {
    GetApprovalMatrixQuery,
    GetUserProfileAttributesQuery,
    SaveApprovalMatrixMutation,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {GET_APPROVAL_MATRIX} from "@/queries/GetApprovalMatrix"
import {SAVE_APPROVAL_MATRIX} from "@/queries/SaveApprovalMatrix"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {getAttributeLabel} from "@/services/UserService"
import {IApplicationsStatus} from "@/types/applications"
import {IPermissions} from "@/types/keycloak"
import {ApprovalMatrixRuleDialog} from "./ApprovalMatrixRuleDialog"
import {ApprovalMatrixRules, ruleCardId} from "./ApprovalMatrixRules"
import {
    ApprovalMatrixTest,
    SAVE_BAR_ROOM,
    decidingRule,
    useExampleEvaluation,
} from "./ApprovalMatrixTest"
import {EMatrixEntry, recordedDecision, sampleEnrollment} from "./approvalExample"
import {IApplication, applicantName} from "./approvalReview"
import {contentAfter, scrollFrame} from "./matrixLayout"
import {convertToCamelCase} from "./UtilsApprovals"
import {
    EMatrixError,
    EMatrixSource,
    IApprovalMatrix,
    IApprovalRule,
    ITestEnrollment,
    addRule,
    cleanMatrix,
    deleteRule,
    matrixChanges,
    moveRule,
    profileFieldLabel,
    readMatrix,
    replaceRule,
    sameMatrix,
    validateMatrix,
    withComparedFields,
} from "./approvalMatrix"
import {
    AccentTag,
    Card,
    CardIcon,
    CardTitle,
    Help,
    Muted,
    Notice,
    NoticeTitle,
    Overline,
    Page,
    Segment,
    Tag,
    TagRow,
} from "./approvalStyles"

/** The rule being edited: a position, the last rule, or a new rule. */
type Editing = {index: number | null; isOtherwise: boolean; rule: IApprovalRule}

const NEW_RULE: IApprovalRule = {when: {}, then: {decision: IApplicationsStatus.PENDING}}

const BackLink = styled("button")(({theme}) => ({
    "display": "inline-flex",
    "alignItems": "center",
    "gap": "8px",
    "padding": "4px 8px 4px 0",
    "border": 0,
    "background": "none",
    "color": theme.palette.brandColor,
    "fontFamily": "inherit",
    "fontSize": "15px",
    "fontWeight": 600,
    "cursor": "pointer",
    "&:focus-visible": {outline: `2px solid ${theme.palette.brandSuccess}`},
}))

// The example comes first in the page, so reading and keyboard order follow
// what narrow screens show; wide screens place it on the right.
const Columns = styled("div")(({theme}) => ({
    display: "grid",
    gridTemplateColumns: "minmax(0, 1fr)",
    gridTemplateAreas: '"example" "main"',
    gap: "24px",
    alignItems: "start",
    marginTop: "24px",
    [theme.breakpoints.up("lg")]: {
        gridTemplateColumns: "minmax(0, 1fr) 360px",
        gridTemplateAreas: '"main example"',
    },
}))

/** Where the save bar sticks above the window's bottom, and the space kept above it. */
const SAVE_BAR_BOTTOM = 16
const SAVE_BAR_GAP = 16

const SaveBar = styled("div")(({theme}) => ({
    position: "sticky",
    bottom: `${SAVE_BAR_BOTTOM}px`,
    zIndex: 2,
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    justifyContent: "space-between",
    gap: "12px",
    marginTop: "24px",
    padding: "16px 20px",
    borderRadius: "12px",
    border: "1px solid #E3E7EF",
    background: theme.palette.white,
    boxShadow: "0 8px 24px rgba(15, 5, 76, 0.16)",
}))

/** The enrollment the matrix was opened from, and the way back to it. */
export interface IMatrixSource {
    application: IApplication
    onBack: () => void
}

export interface ApprovalMatrixProps {
    electionEventId: string
    goBack: () => void
    /** The enrollment the matrix was opened from; the toolbar opens it without one. */
    source?: IMatrixSource
}

export const ApprovalMatrix: React.FC<ApprovalMatrixProps> = ({
    electionEventId,
    goBack,
    source,
}) => {
    const {t, i18n} = useTranslation()
    const notify = useNotify()
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
    const application = source?.application
    const recorded = useMemo(
        () => (application ? recordedDecision(application) : null),
        [application]
    )
    // An enrollment whose recorded inputs can't be read is shown with the sample.
    const entry = recorded?.enrollment ? EMatrixEntry.ENROLLMENT : EMatrixEntry.TOOLBAR
    const joint = entry === EMatrixEntry.ENROLLMENT && Boolean(recorded?.jointNames)
    const [applied, setApplied] = useState<ITestEnrollment>(
        () => recorded?.enrollment ?? sampleEnrollment()
    )
    const evaluation = useExampleEvaluation(electionEventId, draft, applied, entry, joint)
    const [offered, setOffered] = useState<string[]>([])
    const [detailMenu, setDetailMenu] = useState<HTMLElement | null>(null)

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
    const profileFields = useMemo(
        () =>
            (profileAttributes ?? [])
                .map((attribute) => convertToCamelCase(attribute.name ?? ""))
                .filter((name) => name),
        [profileAttributes]
    )
    // The save bar's height as drawn (it wraps on narrow windows and long texts), with the
    // gap below it, so the example card and its editor stay clear of it.
    const [saveBarRoom, setSaveBarRoom] = useState(0)
    const saveBar = useCallback((element: HTMLDivElement | null) => {
        if (!element) {
            return
        }
        const measure = () => setSaveBarRoom(element.offsetHeight + SAVE_BAR_BOTTOM + SAVE_BAR_GAP)
        measure()
        const observer = new ResizeObserver(measure)
        observer.observe(element)
        return () => {
            observer.disconnect()
            setSaveBarRoom(0)
        }
    }, [])
    // The height of the page's sticky header (the portal's app bar), which covers the top of
    // the window; none outside the portal's layout.
    const [headerRoom, setHeaderRoom] = useState(0)
    useEffect(() => {
        const header = Array.from(document.querySelectorAll("header")).find((element) =>
            ["sticky", "fixed"].includes(getComputedStyle(element).position)
        )
        if (!header) {
            return
        }
        const measure = () => setHeaderRoom(header.offsetHeight)
        measure()
        const observer = new ResizeObserver(measure)
        observer.observe(header)
        return () => observer.disconnect()
    }, [])
    // What the page shows below the columns (the save bar, the page's margins): at the end of
    // the page the sticky card is pushed up by it, so the card is never taller than what fits
    // between the header and it. Measured from the sizes of what follows the columns, not from
    // positions, so neither the card's height nor empty window space changes it.
    const [belowRoom, setBelowRoom] = useState(0)
    const [columnsElement, setColumnsElement] = useState<HTMLDivElement | null>(null)
    const columns = useCallback((element: HTMLDivElement | null) => setColumnsElement(element), [])
    useEffect(() => {
        if (!columnsElement) {
            return
        }
        const scroller = scrollFrame(columnsElement)
        const measure = () => setBelowRoom(contentAfter(columnsElement, scroller))
        measure()
        const observer = new ResizeObserver(measure)
        observer.observe(columnsElement)
        observer.observe(scroller)
        return () => observer.disconnect()
    }, [columnsElement, saveBarRoom])
    const goToRule = useCallback((rule: number | null) => {
        const card = document.getElementById(ruleCardId(rule))
        const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
        card?.scrollIntoView({block: "center", behavior: still ? "auto" : "smooth"})
        card?.focus({preventScroll: true})
    }, [])

    if (loading) {
        return <CircularProgress aria-label={String(t("approvalsScreen.matrix.title"))} />
    }

    const back = (
        <BackLink type="button" onClick={goBack}>
            <ArrowBackIcon fontSize="small" />
            {t("approvalsScreen.matrix.back")}
        </BackLink>
    )

    if (error || !current || !draft || !saved) {
        return (
            <Page>
                {back}
                <Notice data-tone="error" role="alert" sx={{marginTop: "16px"}}>
                    {t("approvalsScreen.matrix.loadError")}
                </Notice>
            </Page>
        )
    }

    const changed = !sameMatrix(draft, saved)
    const problems = validateMatrix(draft)
    const changes = changed ? matrixChanges(saved, draft, t, fieldLabel) : []
    const brokenRules = new Set(problems.map(({rule}) => rule)).size
    // The details offered to compare: those of the saved version and any added since.
    const details = [...saved.compared_fields, ...draft.compared_fields, ...offered].filter(
        (field, index, fields) => fields.indexOf(field) === index
    )
    const otherDetails = profileFields.filter((field) => !details.includes(field))

    const toggleDetail = (field: string) =>
        setDraft(
            withComparedFields(
                draft,
                draft.compared_fields.includes(field)
                    ? draft.compared_fields.filter((compared) => compared !== field)
                    : details.filter(
                          (detail) => detail === field || draft.compared_fields.includes(detail)
                      )
            )
        )

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
            setOffered([])
            notify(t("approvalsScreen.matrix.save.success", {version: savedVersion.version}), {
                type: "success",
            })
        } catch {
            notify(t("approvalsScreen.matrix.save.error"), {type: "error"})
        }
    }

    const savedOn = current.created_at
        ? new Date(current.created_at).toLocaleString(i18n.language, {
              dateStyle: "medium",
              timeStyle: "short",
          })
        : "-"

    return (
        <Page style={{[SAVE_BAR_ROOM]: `${saveBarRoom}px`} as React.CSSProperties}>
            {back}
            <Box
                sx={{
                    display: "flex",
                    flexWrap: "wrap",
                    justifyContent: "space-between",
                    gap: "16px",
                    marginTop: "16px",
                }}
            >
                <Box sx={{maxWidth: "460px"}}>
                    <Box component="h2" sx={{margin: "0 0 8px", fontSize: "26px", fontWeight: 600}}>
                        {t("approvalsScreen.matrix.title")}
                    </Box>
                    <Muted>{t("approvalsScreen.matrix.subtitle")}</Muted>
                </Box>
                <Box sx={{textAlign: "right"}}>
                    <TagRow sx={{justifyContent: "flex-end", marginBottom: "8px"}}>
                        <AccentTag>
                            {t("approvalsScreen.matrix.versionChip", {version: current.version})}
                        </AccentTag>
                        {changed && (
                            <Tag
                                sx={{
                                    background: "#FFF3CD",
                                    borderColor: "#F0D58C",
                                    color: "#7A5200",
                                }}
                            >
                                {t("approvalsScreen.matrix.unsaved")}
                            </Tag>
                        )}
                        {!canEdit && (
                            <Tag>
                                <LockOutlinedIcon sx={{fontSize: "16px"}} aria-hidden />
                                {t("approvalsScreen.matrix.viewOnly")}
                            </Tag>
                        )}
                    </TagRow>
                    <Muted>
                        {current.source === EMatrixSource.SAVED
                            ? t("approvalsScreen.matrix.savedBy", {
                                  date: savedOn,
                                  user: current.created_by ?? "-",
                              })
                            : t("approvalsScreen.matrix.builtIn")}
                    </Muted>
                </Box>
            </Box>

            {!canEdit && (
                <Notice data-tone="info" sx={{marginTop: "24px"}}>
                    <LockOutlinedIcon aria-hidden />
                    <div>
                        <NoticeTitle>{t("approvalsScreen.matrix.readOnlyTitle")}</NoticeTitle>
                        {t("approvalsScreen.matrix.readOnlyText")}
                    </div>
                </Notice>
            )}

            <Columns ref={columns}>
                <Card
                    aria-labelledby="approval-matrix-example"
                    sx={{
                        gridArea: "example",
                        boxSizing: "border-box",
                        position: {lg: "sticky"},
                        top: {lg: `calc(${headerRoom}px + 16px)`},
                        // The card scrolls on its own when it is taller than the window, and
                        // stays between the page's header and the save bar, so its buttons can
                        // always be reached.
                        maxHeight: {
                            lg: `calc(100vh - ${headerRoom}px - 16px - ${Math.max(
                                16,
                                saveBarRoom,
                                belowRoom
                            )}px)`,
                        },
                        overflowY: {lg: "auto"},
                    }}
                >
                    <ApprovalMatrixTest
                        applied={applied}
                        onApply={setApplied}
                        entry={entry}
                        joint={joint}
                        source={
                            source && {
                                name: applicantName(source.application),
                                status: source.application.status,
                                recorded,
                                onBack: source.onBack,
                            }
                        }
                        comparedFields={cleanMatrix(draft).compared_fields}
                        validIds={current.valid_ids}
                        fieldLabel={fieldLabel}
                        evaluation={evaluation}
                        unsavedRules={changed}
                        onGoToRule={goToRule}
                    />
                </Card>
                <Box sx={{gridArea: "main", display: "flex", flexDirection: "column", gap: "24px"}}>
                    <Card aria-labelledby="approval-matrix-compared">
                        <CardTitle id="approval-matrix-compared">
                            <CardIcon>
                                <FactCheckIcon fontSize="small" />
                            </CardIcon>
                            {t("approvalsScreen.matrix.compared")}
                        </CardTitle>
                        <Help>{t("approvalsScreen.matrix.comparedHelp")}</Help>
                        <TagRow
                            role="group"
                            aria-label={String(t("approvalsScreen.matrix.compared"))}
                        >
                            {details.map((field) => {
                                const compared = draft.compared_fields.includes(field)
                                return (
                                    <Segment
                                        key={field}
                                        type="button"
                                        aria-pressed={compared}
                                        disabled={!canEdit}
                                        onClick={() => toggleDetail(field)}
                                        sx={{
                                            "display": "inline-flex",
                                            "alignItems": "center",
                                            "gap": "6px",
                                            "padding": "8px 14px",
                                            "borderRadius": "8px",
                                            "&[aria-pressed='true']": {
                                                background: "#ECEEFB",
                                                borderColor: "#C9CEF2",
                                                color: "#0F054C",
                                            },
                                            "&:disabled": {opacity: 1},
                                        }}
                                    >
                                        {compared && (
                                            <CheckIcon sx={{fontSize: "18px"}} aria-hidden />
                                        )}
                                        {fieldLabel(field)}
                                    </Segment>
                                )
                            })}
                            {canEdit && otherDetails.length > 0 && (
                                <>
                                    <Button
                                        variant="secondary"
                                        size="small"
                                        startIcon={<AddIcon />}
                                        aria-haspopup="menu"
                                        onClick={(event) => setDetailMenu(event.currentTarget)}
                                        sx={{minHeight: "38px"}}
                                    >
                                        {t("approvalsScreen.matrix.addCompared")}
                                    </Button>
                                    <Menu
                                        anchorEl={detailMenu}
                                        open={detailMenu !== null}
                                        onClose={() => setDetailMenu(null)}
                                    >
                                        {otherDetails.map((field) => (
                                            <MenuItem
                                                key={field}
                                                onClick={() => {
                                                    setDetailMenu(null)
                                                    setOffered([...offered, field])
                                                    setDraft(
                                                        withComparedFields(draft, [
                                                            ...draft.compared_fields,
                                                            field,
                                                        ])
                                                    )
                                                }}
                                            >
                                                {fieldLabel(field)}
                                            </MenuItem>
                                        ))}
                                    </Menu>
                                </>
                            )}
                        </TagRow>
                        {problems.some(({code}) => code === EMatrixError.NO_COMPARED_FIELDS) && (
                            <Notice data-tone="error" role="alert" sx={{marginTop: "16px"}}>
                                {t(
                                    `approvalsScreen.matrix.errors.${EMatrixError.NO_COMPARED_FIELDS}`
                                )}
                            </Notice>
                        )}
                    </Card>

                    <Card aria-labelledby="approval-matrix-rules">
                        <CardTitle id="approval-matrix-rules">
                            <CardIcon sx={{background: "#E3F4EC", color: "#0B6B43"}}>
                                <RuleIcon fontSize="small" />
                            </CardIcon>
                            {t("approvalsScreen.matrix.rules")}
                        </CardTitle>
                        <Help>{t("approvalsScreen.matrix.rulesHelp")}</Help>
                        <ApprovalMatrixRules
                            matrix={draft}
                            canEdit={canEdit}
                            fieldLabel={fieldLabel}
                            appliesTo={decidingRule(evaluation)}
                            problems={problems}
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
                    </Card>
                </Box>
            </Columns>

            {canEdit && changed && (
                <SaveBar
                    ref={saveBar}
                    role="region"
                    aria-label={String(t("approvalsScreen.matrix.unsaved"))}
                >
                    <div>
                        <NoticeTitle>{t("approvalsScreen.matrix.saveBar.title")}</NoticeTitle>
                        <Muted>
                            {problems.length > 0
                                ? t("approvalsScreen.matrix.saveBar.fix", {count: brokenRules})
                                : changes.length > 1
                                  ? `${changes[0]} · ${t("approvalsScreen.matrix.saveBar.more", {
                                        count: changes.length - 1,
                                    })}`
                                  : changes[0]}
                        </Muted>
                    </div>
                    <Box sx={{display: "flex", gap: "12px"}}>
                        <Button
                            variant="cancel"
                            onClick={() => {
                                setDraft(saved)
                                setOffered([])
                            }}
                        >
                            {t("approvalsScreen.matrix.discard")}
                        </Button>
                        <Button
                            disabled={problems.length > 0 || saving}
                            onClick={() => setConfirmSave(true)}
                        >
                            {t("approvalsScreen.matrix.save.button", {
                                version: current.next_version,
                            })}
                        </Button>
                    </Box>
                </SaveBar>
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
                title={String(
                    t("approvalsScreen.matrix.save.title", {version: current.next_version})
                )}
                ok={String(
                    t("approvalsScreen.matrix.save.confirm", {version: current.next_version})
                )}
                cancel={String(t("common.label.cancel"))}
                handleClose={async (result: boolean) => {
                    if (result) {
                        await save()
                    } else {
                        setConfirmSave(false)
                    }
                }}
            >
                <p>{t("approvalsScreen.matrix.save.body")}</p>
                <Notice data-tone="neutral">
                    <div>
                        <Overline sx={{margin: "0 0 4px"}}>
                            {t("approvalsScreen.matrix.save.changes")}
                        </Overline>
                        <Box component="ul" sx={{margin: 0, paddingLeft: "20px"}}>
                            {changes.map((change) => (
                                <li key={change}>{change}</li>
                            ))}
                        </Box>
                    </div>
                </Notice>
                <Box component="p" sx={{marginBottom: 0}}>
                    <Muted>{t("approvalsScreen.matrix.save.log")}</Muted>
                </Box>
            </Dialog>
        </Page>
    )
}
