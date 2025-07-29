// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {LegacyRef, useEffect, useMemo, useState} from "react"
import {Identifier, SimpleForm, useGetList, useInfiniteGetList} from "react-admin"
import {useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {
    Maybe,
    Sequent_Backend_Area,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
} from "@/gql/graphql"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {
    Autocomplete,
    AutocompleteChangeDetails,
    AutocompleteChangeReason,
    Box,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
    Typography,
} from "@mui/material"
import TextField from "@mui/material/TextField"
import {IAreaContestResults, ICandidateResults, IInvalidVotes} from "@/types/TallySheets"
import {sortFunction} from "./utils"
import {
    EEnableCheckableLists,
    ICandidatePresentation,
    IContestPresentation,
} from "@sequentech/ui-core"
import {filterCandidateByCheckableLists} from "@/services/CandidatesFilter"
import {uniq} from "lodash"
import {createTree, getContestMatches} from "@/services/AreaService"
import {styled} from "@mui/material/styles"

const StyledError = styled(Typography)`
    color: ${({theme}) => theme.palette.red.main};
    margin-top: 3px;
    font-size: 0.85rem;
`

const votingChannels = [
    {id: "PAPER", name: "PAPER"},
    {id: "POSTAL", name: "POSTAL"},
]

interface EditTallySheetProps {
    contest: Sequent_Backend_Contest
    tallySheet?: Sequent_Backend_Tally_Sheet | undefined
    doSelectArea?: (areaId: Identifier) => void
    doCreatedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input) => void
    doEditedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet) => void
    submitRef: LegacyRef<HTMLButtonElement> | undefined
}

interface ICandidateResultsExtended extends ICandidateResults {
    name: string
}

interface IArea {
    id: string
    label?: Maybe<string> | undefined
}

const numbersRegExp = /^[0-9]+$/

export const EditTallySheet: React.FC<EditTallySheetProps> = (props) => {
    const {tallySheet, contest, doCreatedTalySheet, submitRef} = props

    const {t} = useTranslation()

    const [areasList, setAreasList] = useState<IArea[]>([])
    const [channel, setChannel] = React.useState<string | null>(null)
    const [results, setResults] = useState<IAreaContestResults>({
        area_id: "",
        contest_id: "-",
        invalid_votes: {},
        candidate_results: {},
    })
    const [invalids, setInvalids] = useState<IInvalidVotes>({})
    const [candidatesResults, setCandidatesResults] = useState<ICandidateResultsExtended[]>([])
    const [areaNameFilter, setAreaNameFilter] = useState<string | null>(null)
    const [areaIds, setAreaIds] = useState<Array<string>>([])
    const [totalValidError, setTotalValidError] = useState<boolean>(false)
    const [censusError, setCensusError] = useState<boolean>(false)

    const {data: areaContests} = useGetList<Sequent_Backend_Area_Contest>(
        "sequent_backend_area_contest",
        {
            filter: {
                tenant_id: contest.tenant_id,
                election_event_id: contest.election_event_id,
                contest_id: contest.id,
            },
            pagination: {
                perPage: 10000, // Setting initial larger records size of areas
                page: 1,
            },
        }
    )

    const {data: allAreas} = useGetList<Sequent_Backend_Area>("sequent_backend_area", {
        filter: {
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
        },
        pagination: {
            perPage: 10000, // Setting initial larger records size of areas
            page: 1,
        },
    })

    const {data: areas, refetch} = useGetList<Sequent_Backend_Area>("sequent_backend_area", {
        filter: {
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
            name: areaNameFilter ?? "",
            id: {
                format: "hasura-raw-query",
                value: {_in: areaIds},
            },
            /*parent_id: {
                format: "hasura-raw-query",
                value: {_is_null: true},
            },*/
        },
        pagination: {
            perPage: 10000, // Setting initial larger records size of areas
            page: 1,
        },
    })

    const {
        data: fetchedCandidates,
        hasNextPage,
        fetchNextPage,
    } = useInfiniteGetList<Sequent_Backend_Candidate>("sequent_backend_candidate", {
        filter: {
            contest_id: contest.id,
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
        },
        pagination: {page: 1, perPage: 50},
    })

    const checkableLists = useMemo(() => {
        let presentation = contest.presentation as IContestPresentation | undefined
        return presentation?.enable_checkable_lists ?? EEnableCheckableLists.CANDIDATES_AND_LISTS
    }, [contest.presentation])

    const candidates = useMemo(() => {
        //force fetch all records
        hasNextPage && fetchNextPage()
        return fetchedCandidates?.pages.flatMap((item) => item.data)
    }, [fetchedCandidates])

    const uniqueElements = (arr: string[]): string[] => {
        const uniqueObj: {[key: string]: boolean} = {}
        const uniqueArr: string[] = []

        for (const item of arr) {
            if (!uniqueObj[item]) {
                uniqueObj[item] = true
                uniqueArr.push(item)
            }
        }

        return uniqueArr
    }

    useEffect(() => {
        const treeNodeAreas = (allAreas ?? []).map((area) => ({
            id: area.id,
            tenant_id: area.tenant_id,
            election_event_id: area.election_event_id,
            parent_id: area.parent_id,
        }))

        const treeAreaContests = (areaContests ?? []).map((areaContest) => ({
            id: areaContest.id,
            area_id: areaContest.area_id,
            contest_id: areaContest.contest_id,
        }))

        const tree = createTree(treeNodeAreas, treeAreaContests)

        const matchedAreaContests = getContestMatches(tree, contest.id)
        const matchedAreas = matchedAreaContests.map((area) => area.area_id)
        const uniqueAreas: Array<string> = uniqueElements(matchedAreas)

        setAreaIds(uniqueAreas)
    }, [areaContests, allAreas])

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
            setChannel(tallySheetTemp.channel)
        }
    }, [tallySheet, candidates])

    useEffect(() => {
        if (contest) {
            setResults((prev: IAreaContestResults) => ({
                ...prev,
                contest_id: contest.id,
            }))
        }
    }, [contest])

    useEffect(() => {
        if (areas) {
            const areatListTemp: IArea[] = areas?.map((item) => {
                return {
                    id: item.id,
                    label: item.name,
                }
            })
            setAreasList(areatListTemp)
        }
    }, [areas])

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
        let totalVotes = totalValidVotes + (invalids?.total_invalid ?? 0) + (newResults.total_blank_votes ?? 0)

        newResults.total_valid_votes = totalValidVotes
        newResults.total_votes = totalVotes

        // Census must be entered manually, we do not recalculate it.
        // Notify error if census is too small.
        if ( newResults.census && newResults.census < newResults.total_votes) {
            setCensusError(true)
        } else {
            setCensusError(false)
        }

        let allCanditateResultsEntered = true
        let canditatesVotesSum = 0
        for (const candidateResult of candidatesResults) {
            if ( !candidateResult.total_votes ) {
                allCanditateResultsEntered = false
                break
            }
            canditatesVotesSum += candidateResult.total_votes
        }
        
        if ( allCanditateResultsEntered && canditatesVotesSum !== totalValidVotes ) {
            setTotalValidError(true)
        } else {
            setTotalValidError(false)
        }


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

    const handleChange = (
        event: React.SyntheticEvent,
        value: IArea | null,
        reason: AutocompleteChangeReason,
        details?: AutocompleteChangeDetails
    ) => {
        // setArea(event.target.value as string)
        setResults((prev: IAreaContestResults) => ({
            ...prev,
            area_id: value?.id as any,
        }))
    }

    const handleTextChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setResults((prev: IAreaContestResults) => ({
            ...prev,
            [event.target.name as string]: event.target.value as string,
        }))
    }

    const handleNumberChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        console.log(results)
        console.log(event.target.name)
        console.log(event.target.value)
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

        console.log(results)
    }
    const handleCensusChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        console.log(event.target.value)
        console.log(results.total_votes ?? 0)
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

    let timeoutId: NodeJS.Timeout
    const debouncedSearchArea = (event: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = event.target
        clearTimeout(timeoutId)
        timeoutId = setTimeout(() => {
            setAreaNameFilter(value ? value.trim() : null)
            refetch()
        }, 350)
    }

    const onSubmit: SubmitHandler<FieldValues> = async (result) => {
        const resultsTemp = {...results}
        const invalidsTemp = {...invalids}
        const candidatesResultsTemp: {[id: string]: ICandidateResults} = {}
        for (const candiate of candidatesResults) {
            const candiateTemp: ICandidateResults = {
                candidate_id: candiate.candidate_id,
                total_votes: candiate.total_votes,
            }
            candidatesResultsTemp[candiate.candidate_id] = candiateTemp
        }
        resultsTemp.invalid_votes = invalidsTemp
        resultsTemp.candidate_results = candidatesResultsTemp

        const tallySheetData:
            | Sequent_Backend_Tally_Sheet
            | Sequent_Backend_Tally_Sheet_Insert_Input = {
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
            election_id: contest.election_id,
            contest_id: contest.id,
            area_id: resultsTemp.area_id,
            channel: channel || "",
            content: resultsTemp,
        }

        if (tallySheet) {
            tallySheetData.id = tallySheet.id
        }

        localStorage.setItem("tallySheetData", JSON.stringify(tallySheetData))

        if (doCreatedTalySheet) {
            doCreatedTalySheet(tallySheetData)
        }
    }

    let currentArea = useMemo(
        () => areasList.find((area) => area.id === results?.area_id) || null,
        [results?.area_id, areasList]
    )

    return (
        <SimpleForm toolbar={false} onSubmit={onSubmit}>
            <>
                <PageHeaderStyles.Title>{t("tallysheet.common.title")}</PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t("tallysheet.common.subtitle")}
                </PageHeaderStyles.SubTitle>

                <FormControl fullWidth size="small">
                    <Autocomplete
                        sx={{width: 300}}
                        onChange={handleChange as any}
                        options={areasList ?? []}
                        renderInput={(params) => (
                            <TextField
                                {...params}
                                label="Search Area"
                                onChange={debouncedSearchArea}
                                value={areaNameFilter}
                            />
                        )}
                        value={currentArea}
                        isOptionEqualToValue={(a, b) => a.id === b.id}
                    />
                </FormControl>

                <FormControl fullWidth size="small">
                    <InputLabel>{t("tallysheet.label.channel")}</InputLabel>
                    <Select
                        name="channel"
                        value={channel || ""}
                        label={t("tallysheet.label.channel")}
                        onChange={(e: SelectChangeEvent) => setChannel(e.target.value)}
                        required
                    >
                        {votingChannels.map((item) => (
                            <MenuItem key={item.id} value={item.id}>
                                {item.name}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>

                <PageHeaderStyles.Wrapper>
                    <PageHeaderStyles.Title>{t("tallysheet.common.data")}</PageHeaderStyles.Title>
                </PageHeaderStyles.Wrapper>

                <TextField
                    label={t("tallysheet.label.contest_id")}
                    name="constest_id"
                    value={results.contest_id}
                    onChange={handleTextChange}
                    size="small"
                    style={{display: "none"}}
                    required
                />

                <TextField
                    label={t("tallysheet.label.total_votes")}
                    name="total_votes"
                    value={typeof results.total_votes === "number" ? results.total_votes : ""}
                    onChange={handleNumberChange}
                    size="small"
                    required
                    disabled
                />
                <>
                    <TextField
                        label={t("tallysheet.label.total_valid_votes")}
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
                        label={t("tallysheet.label.total_invalid")}
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
                        label={t("tallysheet.label.implicit_invalid")}
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
                        label={t("tallysheet.label.explicit_invalid")}
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
                    label={t("tallysheet.label.total_blank_votes")}
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
                        label={t("tallysheet.label.census")}
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
                            label={t("tallysheet.label.total_votes")}
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
