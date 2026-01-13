// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {LegacyRef, useEffect, useMemo, useState} from "react"
import {SimpleForm, useInfiniteGetList} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
} from "@/gql/graphql"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {Box, Typography} from "@mui/material"
import TextField from "@mui/material/TextField"
import {IAreaContestResults, ICandidateResults, IInvalidVotes} from "@/types/TallySheets"
import {sortFunction} from "./utils"
import {EEnableCheckableLists, IContestPresentation} from "@sequentech/ui-core"
import {filterCandidateByCheckableLists} from "@/services/CandidatesFilter"
import {styled} from "@mui/material/styles"

const StyledError = styled(Typography)`
    color: ${({theme}) => theme.palette.red.main};
    margin-top: 3px;
    font-size: 0.85rem;
`

interface ICandidateResultsExtended extends ICandidateResults {
    name: string
}

const numbersRegExp = /^[0-9]+$/

interface TallySheetsDataStepProps {
    election: Sequent_Backend_Election
    choosenContest?: Sequent_Backend_Contest
    tallySheet?: Sequent_Backend_Tally_Sheet
    submitRef: LegacyRef<HTMLButtonElement> | undefined
    setIsButtonDisabled: (disabled: boolean) => void
    submitDataStep: (results: IAreaContestResults) => void
}

export const TallySheetsDataStep = (props: TallySheetsDataStepProps) => {
    const {
        tallySheet,
        election: election,
        submitRef,
        setIsButtonDisabled,
        choosenContest,
        submitDataStep,
    } = props

    const {t} = useTranslation()

    const [results, setResults] = useState<IAreaContestResults>({
        area_id: tallySheet?.area_id || "",
        contest_id: choosenContest?.id ?? tallySheet?.contest_id,
        total_votes: tallySheet?.content?.total_votes || 0,
        total_valid_votes: tallySheet?.content?.total_valid_votes || 0,
        invalid_votes: tallySheet?.content?.invalid_votes || {},
        total_blank_votes: tallySheet?.content?.total_blank_votes || 0,
        census: tallySheet?.content?.census,
        candidate_results: tallySheet?.content?.candidate_results || {},
    })
    const [invalids, setInvalids] = useState<IInvalidVotes>({})
    const [candidatesResults, setCandidatesResults] = useState<ICandidateResultsExtended[]>([])
    const [totalValidError, setTotalValidError] = useState<boolean>(false)
    const [censusError, setCensusError] = useState<boolean>(false)

    const {
        data: fetchedCandidates,
        hasNextPage,
        fetchNextPage,
        refetch: refetchCandidates,
    } = useInfiniteGetList<Sequent_Backend_Candidate>(
        "sequent_backend_candidate",
        {
            filter: {
                contest_id: choosenContest?.id ?? tallySheet?.contest_id,
                tenant_id: election.tenant_id,
                election_event_id: election.election_event_id,
            },
            pagination: {page: 1, perPage: 50},
        },
        {enabled: !!choosenContest || !!tallySheet?.contest_id}
    )

    const checkableLists = useMemo(() => {
        let presentation = election.presentation as IContestPresentation | undefined
        return presentation?.enable_checkable_lists ?? EEnableCheckableLists.CANDIDATES_AND_LISTS
    }, [election.presentation])

    const candidates = useMemo(() => {
        //force fetch all records
        hasNextPage && fetchNextPage()
        return fetchedCandidates?.pages.flatMap((item) => item.data)
    }, [fetchedCandidates])

    useEffect(() => {
        const tallySaved: string | null = localStorage.getItem("tallySheetData")

        if ((tallySheet || tallySaved) && candidates) {
            const tallySheetTemp = tallySaved ? JSON.parse(tallySaved || "") : tallySheet
            if (tallySheetTemp.content) {
                const contentTemp: IAreaContestResults = {...tallySheetTemp.content}
                if (contentTemp.invalid_votes) {
                    const invalidsTemp = {...contentTemp.invalid_votes}
                    setInvalids(invalidsTemp)
                }
                if (contentTemp.candidate_results) {
                    let candidatesResultsTemp: ICandidateResultsExtended[] = []
                    for (const candidate of candidates) {
                        let isValid = filterCandidateByCheckableLists(candidate, checkableLists)
                        if (!isValid) {
                            continue
                        }
                        const candidateTemp: ICandidateResultsExtended = {
                            candidate_id: candidate.id,
                            name: candidate.name as string,
                        }
                        if (contentTemp.candidate_results[candidate.id]) {
                            candidateTemp.total_votes =
                                contentTemp.candidate_results[candidate.id].total_votes
                        }

                        candidatesResultsTemp.push(candidateTemp)
                    }
                    candidatesResultsTemp.sort(sortFunction)
                    setCandidatesResults(candidatesResultsTemp)
                }
                setResults(contentTemp)
            }
        }
    }, [tallySheet, candidates])

    useEffect(() => {
        if (election) {
            setResults((prev: IAreaContestResults) => ({
                ...prev,
                contest_id: choosenContest?.id ?? tallySheet?.contest_id,
            }))
        }
    }, [election])

    useEffect(() => {
        window.scrollTo(0, 0)
    }, [])

    useEffect(() => {
        const tallySaved: string | null = localStorage.getItem("tallySheetData")

        if (!(tallySheet || tallySaved) && candidates) {
            const candidatesTemp = []
            for (const candidate of candidates) {
                let isValid = filterCandidateByCheckableLists(candidate, checkableLists)
                if (!isValid) {
                    continue
                }
                const candidateTemp: ICandidateResultsExtended = {
                    candidate_id: candidate.id,
                    name: candidate.name as string,
                }
                candidatesTemp.push(candidateTemp)
            }
            candidatesTemp.sort(sortFunction)
            setCandidatesResults(candidatesTemp)
        }
    }, [candidates, tallySheet])

    const recalculateTotals = () => {
        let newResults = {...results}
        let totalValidVotes = newResults.total_valid_votes ?? 0
        let totalBlankVotes = newResults.total_blank_votes ?? 0
        let totalVotes = totalValidVotes + (invalids?.total_invalid ?? 0)

        newResults.total_valid_votes = totalValidVotes
        newResults.total_votes = totalVotes

        // Census must be entered manually, we do not recalculate it.
        // Notify error if census is too small.
        let disableNextButton = false
        if (newResults.census !== undefined && newResults.census < newResults.total_votes) {
            disableNextButton = true
            setCensusError(true)
        } else {
            setCensusError(false)
        }

        let canditatesVotesSum = 0
        for (const candidateResult of candidatesResults) {
            canditatesVotesSum += candidateResult.total_votes ?? 0
        }

        if (canditatesVotesSum + totalBlankVotes !== totalValidVotes) {
            disableNextButton = true
            setTotalValidError(true)
        } else {
            setTotalValidError(false)
        }

        setIsButtonDisabled(disableNextButton)

        if (JSON.stringify(newResults) !== JSON.stringify(results)) {
            setResults(newResults)
        }
    }

    useEffect(recalculateTotals, [
        results,
        candidatesResults,
        results.total_blank_votes,
        results.total_valid_votes,
        invalids?.total_invalid,
        invalids,
    ])

    const handleTextChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setResults((prev: IAreaContestResults) => ({
            ...prev,
            [event.target.name as string]: event.target.value as string,
        }))
    }

    const handleNumberChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        if (event.target.value === "") {
            setResults((prev: IAreaContestResults) => ({
                ...prev,
                [event.target.name as string]: "",
            }))
        } else if (event.target.value === "0") {
            setResults((prev: IAreaContestResults) => ({...prev, [event.target.name as string]: 0}))
        } else {
            if (event.target.value.match(numbersRegExp)) {
                setResults((prev: IAreaContestResults) => ({
                    ...prev,
                    [event.target.name as string]: +event.target.value,
                }))
            }
        }
    }

    const handleCensusChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        let census = 0
        if (event.target.value.match(numbersRegExp)) {
            census = Number(event.target.value)
        }
        setResults({
            ...results,
            census,
        })
    }

    const handleInvalidChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        let newInvalid = {...invalids}
        let key: "explicit_invalid" | "implicit_invalid" = event.target.name as any
        if (event.target.value === "") {
            newInvalid[key] = 0
        } else if (event.target.value.match(numbersRegExp)) {
            newInvalid[key] = Number(event.target.value)
        }
        newInvalid.total_invalid =
            (newInvalid.explicit_invalid ?? 0) + (newInvalid.implicit_invalid ?? 0)
        setInvalids(newInvalid)
    }

    const handleCandidateChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        const candidateTemp = candidatesResults.find(
            (item) => item.candidate_id === event.target.id
        )
        const candidateRest = candidatesResults.filter(
            (item) => item.candidate_id !== event.target.id
        )
        if (candidateTemp) {
            if (!event.target.value) {
                delete candidateTemp.total_votes
            } else {
                if (event.target.value.match(numbersRegExp)) {
                    candidateTemp.total_votes = +event.target.value
                } else {
                    candidateTemp.total_votes = +(candidateTemp?.total_votes || 0)
                }
            }

            const finalCandidates = [...candidateRest, candidateTemp]
            finalCandidates.sort((a, b) => a.name.localeCompare(b.name))
            setCandidatesResults(finalCandidates)
        }
    }

    const onSubmit: SubmitHandler<FieldValues> = async (result) => {
        const resultsTemp = {...results}
        const invalidsTemp = {...invalids}
        const candidatesResultsTemp: {[id: string]: ICandidateResults} = {}
        for (const candidate of candidatesResults) {
            const candidateTemp: ICandidateResults = {
                candidate_id: candidate.candidate_id,
                total_votes: candidate.total_votes,
            }
            candidatesResultsTemp[candidate.candidate_id] = candidateTemp
        }
        resultsTemp.invalid_votes = invalidsTemp
        resultsTemp.candidate_results = candidatesResultsTemp

        submitDataStep(resultsTemp)
    }

    useEffect(() => {
        if (choosenContest) {
            refetchCandidates()
        }
    }, [choosenContest])

    return (
        <SimpleForm toolbar={false} onSubmit={onSubmit}>
            <>
                <PageHeaderStyles.Wrapper>
                    <PageHeaderStyles.Title>{t("tallysheet.common.data")}</PageHeaderStyles.Title>
                </PageHeaderStyles.Wrapper>

                <TextField
                    label={String(t("tallysheet.label.contest_id"))}
                    name="constest_id"
                    value={results.contest_id}
                    onChange={handleTextChange}
                    size="small"
                    style={{display: "none"}}
                    required
                />

                <TextField
                    label={String(t("tallysheet.label.total_votes"))}
                    name="total_votes"
                    value={typeof results.total_votes === "number" ? results.total_votes : ""}
                    onChange={handleNumberChange}
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
                        onChange={handleNumberChange}
                        size="small"
                        required
                    />
                    {totalValidError && (
                        <StyledError>
                            {t("tallysheet.inputError.totalValidDoesNotMatch")}
                        </StyledError>
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
                        value={
                            typeof invalids.total_invalid === "number" ? invalids.total_invalid : ""
                        }
                        onChange={handleInvalidChange}
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
                        onChange={handleInvalidChange}
                        size="small"
                        required
                    />
                    <TextField
                        label={String(t("tallysheet.label.explicit_invalid"))}
                        name="explicit_invalid"
                        value={
                            typeof invalids.explicit_invalid === "number"
                                ? invalids.explicit_invalid
                                : ""
                        }
                        onChange={handleInvalidChange}
                        size="small"
                        required
                    />
                </Box>

                <TextField
                    label={String(t("tallysheet.label.total_blank_votes"))}
                    name="total_blank_votes"
                    value={
                        typeof results.total_blank_votes === "number"
                            ? results.total_blank_votes
                            : ""
                    }
                    onChange={handleNumberChange}
                    size="small"
                    required
                />
                <>
                    <TextField
                        label={String(t("tallysheet.label.census"))}
                        name="census"
                        value={typeof results.census === "number" ? results.census : ""}
                        onChange={handleCensusChange}
                        size="small"
                        required
                    />
                    {censusError && (
                        <StyledError>{t("tallysheet.inputError.censusTooSmall")}</StyledError>
                    )}
                </>
                <PageHeaderStyles.Wrapper>
                    <PageHeaderStyles.Title>
                        {t("tallysheet.common.candidates")}
                    </PageHeaderStyles.Title>
                </PageHeaderStyles.Wrapper>

                {candidatesResults.map((candidate: ICandidateResultsExtended) => (
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
                            value={candidate.total_votes}
                            onChange={handleCandidateChange}
                            size="small"
                            required
                        />
                    </Box>
                ))}
                <button ref={submitRef} type="submit" style={{display: "none"}} />
            </>
        </SimpleForm>
    )
}
