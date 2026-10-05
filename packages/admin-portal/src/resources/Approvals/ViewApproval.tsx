// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {Identifier, useDataProvider, useGetOne, useNotify, useRefresh} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {useQuery as useLookup} from "@tanstack/react-query"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Box,
    Button,
    Checkbox,
    CircularProgress,
    FormControlLabel,
    IconButton,
    Radio,
    TextField,
    Tooltip,
} from "@mui/material"
import {styled} from "@mui/material/styles"
import ArrowBackIosNewIcon from "@mui/icons-material/ArrowBackIosNew"
import ArrowForwardIosIcon from "@mui/icons-material/ArrowForwardIos"
import CheckIcon from "@mui/icons-material/Check"
import CloseIcon from "@mui/icons-material/Close"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined"
import WarningAmberIcon from "@mui/icons-material/WarningAmber"
import {Dialog} from "@sequentech/ui-essentials"
import {
    ChangeApplicationStatusMutation,
    GetUserProfileAttributesQuery,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {useSignedAction} from "@/hooks/useSignedAction"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {CHANGE_APPLICATION_STATUS} from "@/queries/ChangeApplicationStatus"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {getAttributeLabel} from "@/services/UserService"
import {ApplicationsError, IApplicationsStatus, RejectReason} from "@/types/applications"
import {ApprovalStatusChip} from "./ApprovalChips"
import {ApprovalVoterStep, NO_VOTER} from "./ApprovalVoterStep"
import {convertToCamelCase} from "./UtilsApprovals"
import {
    EIdentityMethod,
    conditionLabels,
    profileFieldLabel,
    rejectionReasonKey,
} from "./approvalMatrix"
import {
    IRegistryVoter,
    VoterFilter,
    applicantData,
    applicantName,
    candidateFilters,
    decidedBy,
    decisionDetails,
    differenceText,
    differingFields,
    displayValue,
    initials,
    joinList,
    listAnnotation,
    rankCandidates,
    searchFilters,
    voterName,
    waitingTime,
} from "./approvalReview"
import {
    Avatar,
    Muted,
    Notice,
    NoticeTitle,
    NumberBadge,
    OptionCard,
    OptionTitle,
    Overline,
    Tag,
    TagRow,
} from "./approvalStyles"

const JOINT_NAME_DOCUMENTS = ["seamanBook", "driversLicense"]
const ID_CARD_TYPE_FIELD = "sequent.read-only.id-card-type"
/** How many registry voters each lookup asks for. */
const LOOKUP_SIZE = 10
/** How many registry voters the step offers. */
const CANDIDATES_SHOWN = 5

const STEPS = ["identity", "voter", "decide"] as const
type Step = (typeof STEPS)[number]

const Stepper = styled("ol")({
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "8px",
    margin: "24px 0 16px",
    padding: 0,
    listStyle: "none",
})

const StepItem = styled("li")({
    "display": "flex",
    "alignItems": "center",
    "gap": "8px",
    "fontSize": "15px",
    "color": "#5B6272",
    "&[aria-current='step'], &[data-done='true']": {color: "#191D23"},
    "&:not(:last-of-type)::after": {
        content: '""',
        width: "min(160px, 10vw)",
        borderTop: "1px dashed #8A90A0",
        margin: "0 4px",
    },
})

const Section = styled(Accordion)({
    "width": "100%",
    "margin": "0 0 8px !important",
    "boxShadow": "0 1px 2px rgba(15, 5, 76, 0.12)",
    "&::before": {display: "none"},
})

const DetailsTable = styled("table")({
    "width": "100%",
    "borderCollapse": "collapse",
    "border": "1px solid #E3E7EF",
    "fontSize": "14px",
    "& th, & td": {
        padding: "8px 16px",
        borderBottom: "1px solid #E3E7EF",
        textAlign: "left",
    },
    "& th": {width: "40%", fontWeight: 600},
})

const Footer = styled("div")(({theme}) => ({
    position: "sticky",
    bottom: 0,
    zIndex: 2,
    display: "flex",
    flexWrap: "wrap",
    justifyContent: "space-between",
    gap: "12px",
    marginTop: "32px",
    padding: "16px",
    background: theme.palette.white,
    boxShadow: "0 -2px 6px rgba(15, 5, 76, 0.08)",
}))

export interface ViewApprovalProps {
    electionEventId: string
    electionId?: string
    currApprovalId: Identifier | String | null
    goBack: () => void
    /** Opens the approval matrix on the rule that decided this enrollment. */
    onViewRule?: (decided: {version: number; rule: number | null}) => void
    electionEventRecord?: Sequent_Backend_Election_Event
}

export const ViewApproval: React.FC<ViewApprovalProps> = ({
    electionEventId,
    electionId,
    currApprovalId,
    goBack,
    onViewRule,
}) => {
    const {t, i18n} = useTranslation()
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const refresh = useRefresh()
    const dataProvider = useDataProvider()
    const openSigning = useSignedAction()

    const [step, setStep] = useState<Step | null>("identity")
    const [faceToFace, setFaceToFace] = useState(false)
    const [chosen, setChosen] = useState<string | null>(null)
    const [search, setSearch] = useState("")
    const [lookedUp, setLookedUp] = useState("")
    const [verdict, setVerdict] = useState<"approve" | "reject" | null>(null)
    const [reason, setReason] = useState<RejectReason>(RejectReason.NO_VOTER)
    const [message, setMessage] = useState("")
    const [confirmApprove, setConfirmApprove] = useState(false)
    const [sending, setSending] = useState(false)
    const [copied, setCopied] = useState(false)
    const now = useMemo(() => new Date(), [])

    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {variables: {tenantId, electionEventId}}
    )
    const {
        data: task,
        isLoading,
        error,
    } = useGetOne("sequent_backend_applications", {id: currApprovalId})
    const [changeStatus] = useMutation<ChangeApplicationStatusMutation>(CHANGE_APPLICATION_STATUS)

    // The registry is searched a moment after the administrator stops typing.
    useEffect(() => {
        const timer = setTimeout(() => setLookedUp(search.trim()), 400)
        return () => clearTimeout(timer)
    }, [search])

    const profileAttributes = userAttributes?.get_user_profile_attributes
    const fieldLabel = useMemo(
        () => profileFieldLabel(profileAttributes ?? [], t, getAttributeLabel),
        [profileAttributes, t]
    )
    const decision = useMemo(() => (task ? decisionDetails(task) : null), [task])
    const comparedFields = useMemo(() => {
        const decided = Object.keys(decision?.fields ?? {})
        const asked = task ? listAnnotation(task, "search-attributes") : []
        // The decision lists the fields in alphabetical order; keep the order asked for.
        return decided.length > 0
            ? [
                  ...asked.filter((field) => decided.includes(field)),
                  ...decided.filter((field) => !asked.includes(field)),
              ]
            : asked
    }, [decision, task])
    const unsetAttributes = useMemo(
        () => (task ? listAnnotation(task, "unset-attributes") : []),
        [task]
    )
    const status = String(task?.status ?? "").toUpperCase()
    const isAccepted = status === IApplicationsStatus.ACCEPTED

    const filters: VoterFilter[] = useMemo(
        () =>
            !task
                ? []
                : lookedUp
                  ? searchFilters(lookedUp)
                  : candidateFilters(task, comparedFields),
        [task, lookedUp, comparedFields]
    )
    const lookup = useLookup({
        queryKey: ["approval-candidates", electionEventId, electionId, currApprovalId, filters],
        enabled: !!task && !isAccepted && filters.length > 0,
        queryFn: async (): Promise<IRegistryVoter[]> => {
            const pages = await Promise.all(
                filters.map((filter) =>
                    dataProvider.getList<IRegistryVoter & {id: string}>("user", {
                        filter: {
                            tenant_id: tenantId,
                            election_event_id: electionEventId,
                            election_id: electionId,
                            ...filter,
                        },
                        pagination: {page: 1, perPage: LOOKUP_SIZE},
                        sort: {field: "id", order: "ASC"},
                    })
                )
            )
            return pages.flatMap((page) => page.data)
        },
    })
    const candidates = useMemo(
        () =>
            task
                ? rankCandidates(task, lookup.data ?? [], comparedFields, unsetAttributes).slice(
                      0,
                      CANDIDATES_SHOWN
                  )
                : [],
        [task, lookup.data, comparedFields, unsetAttributes]
    )

    if (isLoading) {
        return <CircularProgress aria-label={String(t("approvalsScreen.list.title"))} />
    }

    const backButton = (
        <Button variant="secondary" onClick={goBack} startIcon={<ArrowBackIosNewIcon />}>
            {t("approvalsScreen.flow.backToList")}
        </Button>
    )

    if (error || !task) {
        return (
            <Box sx={{padding: "24px 16px"}}>
                <Notice data-tone="error" role="alert" sx={{marginBottom: "16px"}}>
                    {t("approvalsScreen.review.loadError")}
                </Notice>
                {backButton}
            </Box>
        )
    }

    const data = applicantData(task)
    const name = applicantName(task) || String(t("approvalsScreen.list.unnamed"))
    const isPending = status === IApplicationsStatus.PENDING
    const canDecide = !isAccepted
    const identity = decision?.identity ?? null
    const typedByHand = identity === EIdentityMethod.MANUAL_ENTRY
    const jointNames = JOINT_NAME_DOCUMENTS.includes(data[ID_CARD_TYPE_FIELD] ?? "")
    const chosenCandidate = candidates.find((candidate) => candidate.voter.id === chosen)
    const best = candidates.find((candidate) => !candidate.alreadyEnrolled)
    const officer = decidedBy(task)
    const formatDate = (value: string | null | undefined, withTime = true) =>
        value
            ? new Date(value).toLocaleString(i18n.language, {
                  dateStyle: "medium",
                  ...(withTime ? {timeStyle: "short"} : {}),
              })
            : "-"

    const whyApproveIsOff = !chosenCandidate
        ? chosen === NO_VOTER
            ? t("approvalsScreen.flow.decide.noVoter")
            : t("approvalsScreen.flow.decide.chooseVoter")
        : chosenCandidate.alreadyEnrolled
          ? t("approvalsScreen.flow.decide.enrolled")
          : typedByHand && !faceToFace
            ? t("approvalsScreen.flow.decide.faceToFace")
            : null
    const canApprove = whyApproveIsOff === null
    // Approving is offered first; rejecting only once no voter was found.
    const effectiveVerdict =
        verdict ?? (canApprove ? "approve" : isPending && chosen === NO_VOTER ? "reject" : null)
    const messageMissing = reason === RejectReason.OTHER && !message.trim()

    const why = (): string => {
        const rejectionReason = String(
            t(rejectionReasonKey((task.annotations ?? {}).rejection_reason))
        ).toLowerCase()
        if (isAccepted) {
            return officer
                ? t("approvalsScreen.review.why.approvedBy", {
                      name: officer,
                      date: formatDate(task.updated_at, false),
                  })
                : t("approvalsScreen.review.why.approvedAuto")
        }
        if (status === IApplicationsStatus.REJECTED) {
            return officer
                ? t("approvalsScreen.review.why.rejectedBy", {
                      name: officer,
                      date: formatDate(task.updated_at, false),
                      reason: rejectionReason,
                  })
                : t("approvalsScreen.review.why.rejectedAuto", {reason: rejectionReason})
        }
        if (!decision) {
            return t("approvalsScreen.review.why.unknown")
        }
        if (typedByHand) {
            return t("approvalsScreen.review.why.typedByHand")
        }
        if (decision.approvedVoters > 1) {
            return t("approvalsScreen.review.why.severalVoters")
        }
        if (!decision.voterFound) {
            return t("approvalsScreen.review.why.noVoter")
        }
        const differing = differingFields(decision)
        if (differing.length === 0) {
            return t("approvalsScreen.review.why.pending")
        }
        // The registry values are those of the closest voter, when it differs in the same details.
        const differences = (best?.details ?? []).filter((detail) => !detail.same)
        const sameDetails =
            differences.length > 0 &&
            differences
                .flatMap((detail) => detail.fields)
                .every((field) => differing.includes(field))
        return sameDetails
            ? t("approvalsScreen.review.why.differs", {
                  count: differences.length,
                  details: differences
                      .map((detail) => differenceText(detail, t, fieldLabel))
                      .join("; "),
              })
            : t("approvalsScreen.review.why.differsFields", {
                  count: differing.length,
                  fields: joinList(
                      differing.map((field) => fieldLabel(field).toLowerCase()),
                      t
                  ),
              })
    }

    const approve = async () => {
        setConfirmApprove(false)
        if (!chosenCandidate) {
            return
        }
        setSending(true)
        try {
            const {data: reply, errors} = await changeStatus({
                variables: {
                    tenant_id: tenantId,
                    id: task.id,
                    user_id: chosenCandidate.voter.id,
                    area_id: task.area_id,
                    election_event_id: electionEventId,
                },
            })
            if (reply?.ApplicationChangeStatus?.error || errors) {
                notify(
                    reply?.ApplicationChangeStatus?.error === ApplicationsError.APPROVED_VOTER
                        ? t("approvalsScreen.notifications.VoterApprovedAlready")
                        : t("approvalsScreen.notifications.approveError"),
                    {type: "error"}
                )
                return
            }
            // Approving a voter may wait for signatures: its panel opens instead.
            if (!openSigning(reply?.ApplicationChangeStatus, {onChange: () => refresh()})) {
                notify(t("approvalsScreen.notifications.approveSuccess", {name}), {
                    type: "success",
                })
            }
            goBack()
        } catch {
            notify(t("approvalsScreen.notifications.approveError"), {type: "error"})
        } finally {
            setSending(false)
        }
    }

    const reject = async () => {
        setSending(true)
        try {
            await changeStatus({
                variables: {
                    tenant_id: tenantId,
                    id: task.id,
                    user_id: "",
                    area_id: task.area_id,
                    election_event_id: electionEventId,
                    rejection_reason: reason,
                    rejection_message: reason === RejectReason.OTHER ? message.trim() : undefined,
                },
            })
            notify(t("approvalsScreen.notifications.rejectSuccess", {name}), {type: "success"})
            goBack()
        } catch {
            notify(t("approvalsScreen.notifications.rejectError"), {type: "error"})
        } finally {
            setSending(false)
        }
    }

    const copyId = async () => {
        try {
            await navigator.clipboard.writeText(String(task.id))
            setCopied(true)
        } catch {
            setCopied(false)
        }
    }

    const done = (candidate: Step): boolean =>
        step !== null && STEPS.indexOf(candidate) < STEPS.indexOf(step)

    const section = (key: Step, summary: string, children: React.ReactNode) => {
        const number = STEPS.indexOf(key) + 1
        return (
            <Section
                key={key}
                expanded={step === key}
                onChange={(_, expanded) => setStep(expanded ? key : null)}
                disableGutters
            >
                <AccordionSummary
                    expandIcon={<ExpandMoreIcon />}
                    id={`approval-step-${key}`}
                    aria-controls={`approval-step-${key}-content`}
                >
                    <Box sx={{display: "flex", alignItems: "center", gap: "12px", flexGrow: 1}}>
                        <NumberBadge data-pending={step !== key && !done(key)}>
                            {done(key) ? <CheckIcon fontSize="small" /> : number}
                        </NumberBadge>
                        <Box component="h3" sx={{margin: 0, fontSize: "20px", fontWeight: 600}}>
                            {t(`approvalsScreen.flow.steps.${key}`)}
                        </Box>
                    </Box>
                    {step !== key && summary && (
                        <Box sx={{alignSelf: "center", marginRight: "12px"}}>
                            <Muted>{summary}</Muted>
                        </Box>
                    )}
                </AccordionSummary>
                <AccordionDetails sx={{padding: "0 20px 24px"}}>{children}</AccordionDetails>
            </Section>
        )
    }

    const identityMethod = identity ?? "UNKNOWN"
    const identitySummary = typedByHand
        ? faceToFace
            ? t("approvalsScreen.flow.identity.checked")
            : t("approvalsScreen.flow.identity.notChecked")
        : t(`approvalsScreen.idCheck.method.${identityMethod}`)
    const voterSummary =
        chosen === NO_VOTER
            ? t("approvalsScreen.flow.voter.noneChosen")
            : chosenCandidate
              ? `${voterName(chosenCandidate.voter)} · ${t("approvalsScreen.review.detailsMatch", {
                    count: chosenCandidate.matching,
                    total: chosenCandidate.details.length,
                })}`
              : t("approvalsScreen.flow.voter.notChosen")

    const detailRows = (profileAttributes ?? []).flatMap((attribute) => {
        const key = convertToCamelCase(attribute.name ?? "")
        const value = data[key]
        return attribute.name && value
            ? [
                  <tr key={attribute.name}>
                      <th scope="row">
                          {t(getAttributeLabel(attribute.display_name ?? attribute.name))}
                      </th>
                      <td>{t(value, {defaultValue: displayValue(value, i18n.language)})}</td>
                  </tr>,
              ]
            : []
    })

    const identityStep = (
        <>
            <Box sx={{display: "flex", alignItems: "center", gap: "12px", marginBottom: "12px"}}>
                <Box component="h4" sx={{margin: 0, fontSize: "15px"}}>
                    {t("approvalsScreen.idCheck.title")}
                </Box>
                <Tag
                    sx={
                        identity === EIdentityMethod.VERIFIED
                            ? {background: "#E7F8EF", borderColor: "#A9E0C3", color: "#0B6B43"}
                            : typedByHand
                              ? {background: "#FFF3CD", borderColor: "#F0D58C", color: "#7A5200"}
                              : undefined
                    }
                >
                    {t(`approvalsScreen.idCheck.method.${identityMethod}`)}
                </Tag>
            </Box>
            <Box sx={{marginBottom: "16px"}}>
                {identity === EIdentityMethod.VERIFIED
                    ? t("approvalsScreen.idCheck.verified")
                    : typedByHand
                      ? t("approvalsScreen.idCheck.typedByHand")
                      : t("approvalsScreen.idCheck.unknown")}
            </Box>
            {typedByHand && canDecide && (
                <Notice data-tone="warning" sx={{marginBottom: "16px"}}>
                    <WarningAmberIcon aria-hidden />
                    <div>
                        <NoticeTitle>{t("approvalsScreen.idCheck.faceToFaceTitle")}</NoticeTitle>
                        {t("approvalsScreen.idCheck.faceToFaceText")}
                        <FormControlLabel
                            sx={{display: "flex", marginTop: "8px"}}
                            control={
                                <Checkbox
                                    checked={faceToFace}
                                    onChange={(event) => setFaceToFace(event.target.checked)}
                                />
                            }
                            label={String(t("approvalsScreen.flow.identity.confirm"))}
                        />
                    </div>
                </Notice>
            )}
            <Box component="h4" sx={{margin: "24px 0 12px", fontSize: "15px"}}>
                {t("approvalsScreen.flow.identity.details")}
            </Box>
            <DetailsTable>
                <tbody>
                    {detailRows}
                    <tr>
                        <th scope="row">{t("approvalsScreen.review.applicationId")}</th>
                        <td>
                            <Box component="code" sx={{fontSize: "13px"}}>
                                {String(task.id)}
                            </Box>
                            <Tooltip
                                title={String(
                                    t(
                                        copied
                                            ? "approvalsScreen.review.copied"
                                            : "approvalsScreen.review.copy"
                                    )
                                )}
                            >
                                <IconButton
                                    size="small"
                                    aria-label={String(t("approvalsScreen.review.copy"))}
                                    onClick={copyId}
                                >
                                    <ContentCopyIcon sx={{fontSize: "16px"}} />
                                </IconButton>
                            </Tooltip>
                        </td>
                    </tr>
                </tbody>
            </DetailsTable>
        </>
    )

    const decideStep = (
        <Box sx={{maxWidth: "720px"}}>
            <Box
                role="radiogroup"
                aria-label={String(t("approvalsScreen.flow.steps.decide"))}
                sx={{display: "flex", flexDirection: "column", gap: "12px"}}
            >
                <OptionCard
                    data-selected={effectiveVerdict === "approve"}
                    data-disabled={!canApprove}
                >
                    <Radio
                        sx={{padding: "2px"}}
                        name="approval-verdict"
                        checked={effectiveVerdict === "approve"}
                        disabled={!canApprove}
                        onChange={() => setVerdict("approve")}
                    />
                    <CheckIcon sx={{color: "#0B6B43"}} aria-hidden />
                    <div>
                        <OptionTitle>{t("approvalsScreen.flow.decide.approve")}</OptionTitle>
                        <Muted>
                            {whyApproveIsOff ??
                                t("approvalsScreen.flow.decide.approveText", {
                                    voter: chosenCandidate ? voterName(chosenCandidate.voter) : "",
                                })}
                        </Muted>
                    </div>
                </OptionCard>
                {isPending && (
                    <OptionCard data-selected={effectiveVerdict === "reject"}>
                        <Radio
                            sx={{padding: "2px"}}
                            name="approval-verdict"
                            checked={effectiveVerdict === "reject"}
                            onChange={() => setVerdict("reject")}
                        />
                        <CloseIcon sx={{color: "#DC2626"}} aria-hidden />
                        <div>
                            <OptionTitle>{t("approvalsScreen.flow.decide.reject")}</OptionTitle>
                            <Muted>{t("approvalsScreen.flow.decide.rejectText")}</Muted>
                        </div>
                    </OptionCard>
                )}
            </Box>
            {isPending && effectiveVerdict === "reject" && (
                <Box sx={{margin: "24px 0 0 48px"}}>
                    <Box component="h4" sx={{margin: "0 0 8px", fontSize: "15px"}}>
                        {t("approvalsScreen.reject.rejectReason")}
                    </Box>
                    <Box
                        role="radiogroup"
                        aria-label={String(t("approvalsScreen.reject.rejectReason"))}
                    >
                        {Object.values(RejectReason).map((value) => (
                            <Box
                                component="label"
                                key={value}
                                sx={{display: "flex", gap: "8px", marginBottom: "8px"}}
                            >
                                <Radio
                                    sx={{padding: "2px", alignSelf: "flex-start"}}
                                    name="approval-reject-reason"
                                    checked={reason === value}
                                    onChange={() => setReason(value)}
                                />
                                <div>
                                    <OptionTitle sx={{fontSize: "14px"}}>
                                        {t(`approvalsScreen.reject.reasons.${value}`)}
                                    </OptionTitle>
                                    <Muted>{t(`approvalsScreen.reject.hint.${value}`)}</Muted>
                                </div>
                            </Box>
                        ))}
                    </Box>
                    {reason === RejectReason.OTHER ? (
                        <TextField
                            fullWidth
                            multiline
                            minRows={2}
                            sx={{marginTop: "8px"}}
                            label={String(t("approvalsScreen.reject.message"))}
                            value={message}
                            onChange={(event) => setMessage(event.target.value)}
                            error={messageMissing}
                            helperText={
                                messageMissing
                                    ? String(t("approvalsScreen.reject.messageRequired"))
                                    : undefined
                            }
                        />
                    ) : (
                        <Notice data-tone="neutral" sx={{marginTop: "8px"}}>
                            <div>
                                <Overline sx={{margin: "0 0 4px"}}>
                                    {t("approvalsScreen.reject.previewTitle")}
                                </Overline>
                                {t(`approvalsScreen.reject.preview.${reason}`)}
                            </div>
                        </Notice>
                    )}
                </Box>
            )}
        </Box>
    )

    const next = () => {
        const index = step ? STEPS.indexOf(step) : -1
        setStep(STEPS[Math.min(index + 1, STEPS.length - 1)])
    }

    return (
        <Box sx={{padding: "24px 16px 0"}}>
            <Box sx={{display: "flex", flexWrap: "wrap", alignItems: "center", gap: "16px"}}>
                <Avatar data-size="large" aria-hidden>
                    {initials(name)}
                </Avatar>
                <Box sx={{flexGrow: 1, minWidth: 0}}>
                    <Box component="h2" sx={{margin: 0, fontSize: "24px", fontWeight: 600}}>
                        {name}
                    </Box>
                    <Muted>
                        {[
                            data.email,
                            t("approvalsScreen.review.applied", {
                                date: formatDate(task.created_at),
                            }),
                            isPending
                                ? t("approvalsScreen.review.waiting", {
                                      time: waitingTime(task.created_at, now, t),
                                  })
                                : "",
                        ]
                            .filter((part) => part)
                            .join(" · ")}
                    </Muted>
                </Box>
                <ApprovalStatusChip status={status} />
            </Box>

            <Notice
                data-tone={isPending ? "warning" : isAccepted ? "success" : "neutral"}
                sx={{marginTop: "16px"}}
            >
                {isPending ? <WarningAmberIcon aria-hidden /> : <InfoOutlinedIcon aria-hidden />}
                <Box sx={{flexGrow: 1}}>
                    <NoticeTitle>
                        {isPending
                            ? t("approvalsScreen.review.whyTitle")
                            : t("approvalsScreen.review.decisionTitle")}
                    </NoticeTitle>
                    <Box sx={{maxWidth: "720px"}}>{why()}</Box>
                    {decision && (
                        <TagRow sx={{marginTop: "12px"}}>
                            <Overline sx={{margin: "0 4px 0 0"}}>
                                {decision.rule === null
                                    ? t("approvalsScreen.review.ruleLast", {
                                          version: decision.matrixVersion,
                                      })
                                    : t("approvalsScreen.review.rule", {
                                          rule: decision.rule,
                                          version: decision.matrixVersion,
                                      })}
                            </Overline>
                            {decision.rule !== null &&
                                conditionLabels(decision.conditions, t, fieldLabel).map((label) => (
                                    <Tag key={label}>{label}</Tag>
                                ))}
                            {onViewRule && (
                                <Button
                                    variant="secondary"
                                    size="small"
                                    sx={{marginLeft: "auto", minHeight: "36px"}}
                                    onClick={() =>
                                        onViewRule({
                                            version: decision.matrixVersion,
                                            rule: decision.rule,
                                        })
                                    }
                                >
                                    {t("approvalsScreen.review.seeRule")}
                                </Button>
                            )}
                        </TagRow>
                    )}
                </Box>
            </Notice>

            {canDecide ? (
                <>
                    <Stepper aria-label={String(t("approvalsScreen.flow.stepsLabel"))}>
                        {STEPS.map((key, index) => (
                            <StepItem
                                key={key}
                                aria-current={step === key ? "step" : undefined}
                                data-done={done(key)}
                            >
                                <NumberBadge
                                    data-pending={step !== key && !done(key)}
                                    sx={{width: "24px", height: "24px", fontSize: "12px"}}
                                >
                                    {index + 1}
                                </NumberBadge>
                                {t(`approvalsScreen.flow.steps.${key}`)}
                            </StepItem>
                        ))}
                    </Stepper>
                    {section("identity", identitySummary, identityStep)}
                    {section(
                        "voter",
                        voterSummary,
                        <ApprovalVoterStep
                            candidates={candidates}
                            comparedFields={comparedFields}
                            fieldLabel={fieldLabel}
                            chosen={chosen}
                            onChoose={(value) => {
                                setChosen(value)
                                setVerdict(null)
                            }}
                            search={search}
                            onSearch={setSearch}
                            loading={lookup.isLoading}
                            failed={lookup.isError}
                            jointNames={jointNames}
                        />
                    )}
                    {section("decide", "", decideStep)}
                </>
            ) : (
                <Box sx={{marginTop: "24px"}}>{identityStep}</Box>
            )}

            <Footer>
                {backButton}
                {canDecide &&
                    (step !== "decide" ? (
                        <Button
                            onClick={next}
                            endIcon={<ArrowForwardIosIcon />}
                            sx={{minWidth: "220px"}}
                        >
                            {t("approvalsScreen.flow.continue")}
                        </Button>
                    ) : effectiveVerdict === "reject" ? (
                        <Button
                            disabled={sending || messageMissing}
                            onClick={reject}
                            sx={{
                                "minWidth": "220px",
                                "backgroundColor": "#DC2626",
                                "borderColor": "#DC2626",
                                "&:hover, &:focus": {backgroundColor: "#B91C1C"},
                            }}
                        >
                            {t("approvalsScreen.review.reject")}
                        </Button>
                    ) : (
                        <Button
                            disabled={sending || !canApprove || effectiveVerdict !== "approve"}
                            onClick={() => setConfirmApprove(true)}
                            sx={{minWidth: "220px"}}
                        >
                            {t("approvalsScreen.review.approve")}
                        </Button>
                    ))}
            </Footer>

            <Dialog
                variant="info"
                open={confirmApprove}
                title={String(t("approvalsScreen.review.approveDialog.title", {name}))}
                ok={String(t("approvalsScreen.review.approveDialog.confirm"))}
                cancel={String(t("common.label.cancel"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        approve()
                    } else {
                        setConfirmApprove(false)
                    }
                }}
            >
                <p>{t("approvalsScreen.review.approveDialog.body")}</p>
                {chosenCandidate && (
                    <Notice data-tone="neutral" sx={{alignItems: "center"}}>
                        <Avatar aria-hidden>{initials(voterName(chosenCandidate.voter))}</Avatar>
                        <div>
                            <OptionTitle>{voterName(chosenCandidate.voter)}</OptionTitle>
                            <Muted>
                                {t("approvalsScreen.review.detailsMatch", {
                                    count: chosenCandidate.matching,
                                    total: chosenCandidate.details.length,
                                })}
                            </Muted>
                        </div>
                    </Notice>
                )}
                {typedByHand && faceToFace && (
                    <p>{t("approvalsScreen.review.approveDialog.checked")}</p>
                )}
                <Box component="p" sx={{marginBottom: 0}}>
                    <Muted>{t("approvalsScreen.review.approveDialog.irreversible")}</Muted>
                </Box>
            </Dialog>
        </Box>
    )
}
