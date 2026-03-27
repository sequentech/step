// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import TextField from "@mui/material/TextField"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {IAreaContestResults, ICandidateResults, IInvalidVotes} from "@/types/TallySheets"

const StyledError = styled(Typography)`
    color: ${({theme}) => theme.palette.red.main};
    margin-top: 3px;
    font-size: 0.85rem;
`

export interface ICandidateResultsExtended extends ICandidateResults {
    name: string
}

export type TallySheetsDataFieldsProps = {
    results: IAreaContestResults
    invalids: IInvalidVotes
    candidatesResults: ICandidateResultsExtended[]
    totalValidError?: boolean
    censusError?: boolean
    // handlers (optional in readOnly mode)
    onNumberChange?: (e: React.ChangeEvent<HTMLInputElement>) => void
    onCensusChange?: (e: React.ChangeEvent<HTMLInputElement>) => void
    onInvalidChange?: (e: React.ChangeEvent<HTMLInputElement>) => void
    onCandidateChange?: (e: React.ChangeEvent<HTMLInputElement>) => void

    isEditable: boolean
}

export function TallySheetsDataFields(props: TallySheetsDataFieldsProps) {
    const {t} = useTranslation()
    const {
        results,
        invalids,
        candidatesResults,
        totalValidError,
        censusError,
        onNumberChange,
        onCensusChange,
        onInvalidChange,
        onCandidateChange,
        isEditable,
    } = props

    return (
        <>
            <PageHeaderStyles.Wrapper>
                <PageHeaderStyles.Title>{t("tallysheet.common.data")}</PageHeaderStyles.Title>
            </PageHeaderStyles.Wrapper>
            <TextField
                label={String(t("tallysheet.label.total_votes"))}
                name="total_votes"
                value={typeof results.total_votes === "number" ? results.total_votes : ""}
                onChange={onNumberChange}
                size="small"
                required
                disabled
            />

            <>
                <TextField
                    label={String(t("tallysheet.label.total_valid_votes"))}
                    name="total_valid_votes"
                    value={
                        typeof results.total_valid_votes === "number"
                            ? results.total_valid_votes
                            : ""
                    }
                    onChange={onNumberChange}
                    size="small"
                    required
                    disabled={!isEditable}
                />
                {totalValidError && (
                    <StyledError>{t("tallysheet.inputError.totalValidDoesNotMatch")}</StyledError>
                )}
            </>

            <Box
                sx={{
                    width: "100%",
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    gap: "1rem",
                }}
            >
                <TextField
                    label={String(t("tallysheet.label.total_invalid"))}
                    name="total_invalid"
                    value={typeof invalids.total_invalid === "number" ? invalids.total_invalid : ""}
                    onChange={onInvalidChange}
                    size="small"
                    required
                    disabled
                />

                <TextField
                    label={String(t("tallysheet.label.implicit_invalid"))}
                    name="implicit_invalid"
                    value={
                        typeof invalids.implicit_invalid === "number"
                            ? invalids.implicit_invalid
                            : ""
                    }
                    onChange={onInvalidChange}
                    size="small"
                    required
                    disabled={!isEditable}
                />

                <TextField
                    label={String(t("tallysheet.label.explicit_invalid"))}
                    name="explicit_invalid"
                    value={
                        typeof invalids.explicit_invalid === "number"
                            ? invalids.explicit_invalid
                            : ""
                    }
                    onChange={onInvalidChange}
                    size="small"
                    required
                    disabled={!isEditable}
                />
            </Box>

            <TextField
                label={String(t("tallysheet.label.total_blank_votes"))}
                name="total_blank_votes"
                value={
                    typeof results.total_blank_votes === "number" ? results.total_blank_votes : ""
                }
                onChange={onNumberChange}
                size="small"
                required
                disabled={!isEditable}
            />

            <>
                <TextField
                    label={String(t("tallysheet.label.census"))}
                    name="census"
                    value={typeof results.census === "number" ? results.census : ""}
                    onChange={onCensusChange}
                    size="small"
                    required
                    disabled={!isEditable}
                />
                {censusError && (
                    <StyledError>{t("tallysheet.inputError.censusTooSmall")}</StyledError>
                )}
            </>

            <PageHeaderStyles.Wrapper>
                <PageHeaderStyles.Title>{t("tallysheet.common.candidates")}</PageHeaderStyles.Title>
            </PageHeaderStyles.Wrapper>

            {candidatesResults.map((candidate) => (
                <Box
                    sx={{
                        width: "100%",
                        display: "flex",
                        flexDirection: "row",
                        justifyContent: "space-between",
                        alignItems: "center",
                        gap: "1rem",
                    }}
                    key={candidate.candidate_id}
                >
                    <Typography variant="body1" sx={{width: "50%"}}>
                        {candidate.name}
                    </Typography>

                    <TextField
                        id={candidate.candidate_id}
                        label={String(t("tallysheet.label.total_votes"))}
                        name="total_votes"
                        value={candidate.total_votes ?? ""}
                        onChange={onCandidateChange}
                        size="small"
                        required
                        disabled={!isEditable}
                    />
                </Box>
            ))}
        </>
    )
}
