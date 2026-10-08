// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {Box, CircularProgress, InputAdornment, Radio, TextField} from "@mui/material"
import {styled} from "@mui/material/styles"
import CheckIcon from "@mui/icons-material/Check"
import CloseIcon from "@mui/icons-material/Close"
import SearchIcon from "@mui/icons-material/Search"
import {FieldLabel} from "./approvalMatrix"
import {ICandidate, displayValue, initials, joinList, voterName, voterValue} from "./approvalReview"
import {Avatar, Help, Muted, Notice, OptionCard, OptionTitle, Tag} from "./approvalStyles"

/** The choice of step 2 when no registry voter is the applicant. */
export const NO_VOTER = "none"

const NAME_FIELDS = ["firstName", "middleName", "lastName"]

const CompareTable = styled("table")({
    "width": "100%",
    "borderCollapse": "collapse",
    "border": "1px solid #E3E7EF",
    "fontSize": "14px",
    "& th, & td": {
        padding: "8px 16px",
        borderBottom: "1px solid #E3E7EF",
        textAlign: "left",
    },
    "& thead th": {color: "#3D4353", fontSize: "12px", fontWeight: 600},
    "& tbody th": {fontWeight: 600},
    "& tr[data-same='false']": {background: "#FFF6E0"},
    "& tr[data-same='false'] td": {fontWeight: 600},
})

const Result = styled("span")({
    "display": "inline-flex",
    "alignItems": "center",
    "gap": "4px",
    "fontWeight": 600,
    "&[data-same='true']": {color: "#0B6B43"},
    "& svg": {fontSize: "18px"},
})

export interface ApprovalVoterStepProps {
    /** The registry voters found for the enrollment or the search, the closest first. */
    candidates: ICandidate[]
    comparedFields: string[]
    fieldLabel: FieldLabel
    /** The chosen voter's id, `NO_VOTER`, or empty before choosing. */
    chosen: string | null
    onChoose: (chosen: string) => void
    search: string
    onSearch: (search: string) => void
    loading: boolean
    failed: boolean
    /** Whether first and middle name are compared together for this enrollment. */
    jointNames: boolean
    /** A decided enrollment is shown without the choice. */
    readOnly?: boolean
}

/** Step 2 of the review: choosing the registry voter the enrollment belongs to. */
export const ApprovalVoterStep: React.FC<ApprovalVoterStepProps> = ({
    candidates,
    comparedFields,
    fieldLabel,
    chosen,
    onChoose,
    search,
    onSearch,
    loading,
    failed,
    jointNames,
    readOnly = false,
}) => {
    const {t, i18n} = useTranslation()
    const best = candidates.find((candidate) => !candidate.alreadyEnrolled) ?? candidates[0]
    const compared =
        candidates.find((candidate) => candidate.voter.id === chosen) ??
        (chosen === NO_VOTER ? undefined : best)
    const fields = joinList(
        comparedFields.map((field) => fieldLabel(field).toLowerCase()),
        t
    )

    const meta = (candidate: ICandidate) =>
        comparedFields
            .filter((field) => !NAME_FIELDS.includes(field))
            .map((field) => voterValue(candidate.voter, field))
            .filter((value) => value)
            .map((value) => displayValue(value, i18n.language))
            .join(" · ")

    return (
        <>
            <Help>
                {search.trim()
                    ? t("approvalsScreen.review.registrySearching")
                    : t("approvalsScreen.review.registryHelp", {fields})}
            </Help>
            {failed && (
                <Notice data-tone="error" role="alert" sx={{marginBottom: "12px"}}>
                    {t("approvalsScreen.review.registryError")}
                </Notice>
            )}
            <Box
                role="radiogroup"
                aria-label={String(t("approvalsScreen.review.candidates"))}
                sx={{display: "flex", flexDirection: "column", gap: "10px"}}
            >
                {loading && (
                    <CircularProgress
                        size={24}
                        aria-label={String(t("approvalsScreen.review.registryLoading"))}
                    />
                )}
                {!loading && !failed && candidates.length === 0 && (
                    <Muted>{t("approvalsScreen.review.noCandidates")}</Muted>
                )}
                {candidates.map((candidate) => {
                    const id = candidate.voter.id ?? ""
                    const name = voterName(candidate.voter)
                    const disabled = readOnly || candidate.alreadyEnrolled
                    return (
                        <OptionCard
                            key={id}
                            data-selected={chosen === id}
                            data-disabled={disabled}
                            sx={{alignItems: "center"}}
                        >
                            <Radio
                                sx={{padding: "2px"}}
                                name="approval-voter"
                                checked={chosen === id}
                                disabled={disabled}
                                onChange={() => onChoose(id)}
                            />
                            <Avatar aria-hidden>{initials(name)}</Avatar>
                            <Box sx={{flexGrow: 1, minWidth: 0}}>
                                <OptionTitle>{name}</OptionTitle>
                                <Muted>{meta(candidate)}</Muted>
                            </Box>
                            {candidate.alreadyEnrolled ? (
                                <Tag>{t("approvalsScreen.review.alreadyEnrolled")}</Tag>
                            ) : (
                                <Tag
                                    sx={
                                        candidate === best && !search.trim()
                                            ? {
                                                  background: "#FFF3CD",
                                                  borderColor: "#F0D58C",
                                                  color: "#7A5200",
                                              }
                                            : undefined
                                    }
                                >
                                    {candidate === best && !search.trim()
                                        ? `${t("approvalsScreen.review.bestMatch")} · `
                                        : ""}
                                    {t("approvalsScreen.review.detailsMatch", {
                                        count: candidate.matching,
                                        total: candidate.details.length,
                                    })}
                                </Tag>
                            )}
                        </OptionCard>
                    )
                })}
                {!readOnly && (
                    <OptionCard data-selected={chosen === NO_VOTER}>
                        <Radio
                            sx={{padding: "2px"}}
                            name="approval-voter"
                            checked={chosen === NO_VOTER}
                            onChange={() => onChoose(NO_VOTER)}
                        />
                        <div>
                            <OptionTitle>{t("approvalsScreen.flow.voter.none")}</OptionTitle>
                            <Muted>{t("approvalsScreen.flow.voter.noneHint")}</Muted>
                        </div>
                    </OptionCard>
                )}
            </Box>

            {!readOnly && (
                <TextField
                    size="small"
                    sx={{marginTop: "24px", width: "min(640px, 100%)"}}
                    placeholder={String(t("approvalsScreen.review.registrySearch"))}
                    value={search}
                    onChange={(event) => onSearch(event.target.value)}
                    slotProps={{
                        htmlInput: {
                            "aria-label": String(t("approvalsScreen.review.registrySearch")),
                        },
                        input: {
                            startAdornment: (
                                <InputAdornment position="start">
                                    <SearchIcon />
                                </InputAdornment>
                            ),
                        },
                    }}
                />
            )}

            {compared && (
                <>
                    <Box component="h4" sx={{margin: "32px 0 12px", fontSize: "15px"}}>
                        {t("approvalsScreen.review.compareTitle", {
                            name: voterName(compared.voter),
                        })}
                    </Box>
                    <CompareTable>
                        <thead>
                            <tr>
                                <th scope="col">{t("approvalsScreen.review.col.detail")}</th>
                                <th scope="col">{t("approvalsScreen.review.col.enrollment")}</th>
                                <th scope="col">{t("approvalsScreen.review.col.registry")}</th>
                                <th scope="col">{t("approvalsScreen.review.col.result")}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {compared.details.map((detail) => (
                                <tr key={detail.fields.join("+")} data-same={detail.same}>
                                    <th scope="row">
                                        {joinList(detail.fields.map(fieldLabel), t)}
                                    </th>
                                    <td>{displayValue(detail.enrollment, i18n.language)}</td>
                                    <td>{displayValue(detail.registry, i18n.language)}</td>
                                    <td>
                                        <Result data-same={detail.same}>
                                            {detail.same ? (
                                                <CheckIcon aria-hidden />
                                            ) : (
                                                <CloseIcon aria-hidden />
                                            )}
                                            {detail.same
                                                ? t("approvalsScreen.review.same")
                                                : t("approvalsScreen.review.differs")}
                                        </Result>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </CompareTable>
                    <Box sx={{marginTop: "12px"}}>
                        <Muted>
                            {t("approvalsScreen.review.compareNote")}
                            {jointNames ? ` ${t("approvalsScreen.review.compareJoint")}` : ""}
                        </Muted>
                    </Box>
                </>
            )}
        </>
    )
}
