// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {LegacyRef, useEffect, useMemo, useRef, useState} from "react"
import {Identifier, SimpleForm, useGetList, useInfiniteGetList} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {
    Maybe,
    Sequent_Backend_Area,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
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
import {sortFunction, translateSharedValidationError} from "./utils"
import {
    EEnableCheckableLists,
    ICandidatePresentation,
    IContestPresentation,
    TallySheetVotingChannel,
    isTallySheetVotingChannel,
} from "@sequentech/ui-core"
import {validate_area_contest_results_js} from "sequent-core"
import {filterCandidateByCheckableLists} from "@/services/CandidatesFilter"
import {uniq} from "lodash"
import {createTree, getContestMatches} from "@/services/AreaService"
import {styled} from "@mui/material/styles"
import {IPermissions} from "@/types/keycloak"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {normalizeAreaContestResults} from "./validation"

const StyledError = styled(Typography)`
    color: ${({theme}) => theme.palette.red.main};
    margin-top: 3px;
    font-size: 0.85rem;
`

const votingChannels = [TallySheetVotingChannel.Paper, TallySheetVotingChannel.Postal] as const

interface EditTallySheetProps {
    election: Sequent_Backend_Election
    choosenContest?: Sequent_Backend_Contest
    setChoosenContest: (contest: Sequent_Backend_Contest | undefined) => void
    tallySheet?: Sequent_Backend_Tally_Sheet | undefined
    doSelectArea?: (areaId: Identifier) => void
    doCreatedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input) => void
    doEditedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet) => void
    submitRef: LegacyRef<HTMLButtonElement> | undefined
    setIsButtonDisabled: (disabled: boolean) => void
}

interface ICandidateResultsExtended extends ICandidateResults {
    name: string
}

interface IArea {
    id: string
    label?: Maybe<string> | undefined
}

interface IContest {
    id: string
    label?: Maybe<string> | undefined
}

interface SharedValidationError {
    code: string
    message: string
    field: string
    params: Record<string, string>
}

interface IContestMarkBounds {
    max_votes?: Maybe<number>
    counting_algorithm?: Maybe<string>
    cumulative_number_of_checkboxes?: number
}

const validateAreaContestResults = (
    content: IAreaContestResults,
    contestBounds: IContestMarkBounds
): SharedValidationError[] =>
    validate_area_contest_results_js(normalizeAreaContestResults(content), contestBounds)

const numbersRegExp = /^[0-9]+$/

export const EditTallySheet: React.FC<EditTallySheetProps> = (props) => {
    const {
        tallySheet,
        election: election,
        doCreatedTalySheet,
        submitRef,
        setIsButtonDisabled,
        choosenContest,
        setChoosenContest,
    } = props

    const {t, i18n} = useTranslation()
    const aliasRenderer = useAliasRenderer()

    const translateValidationError = (error: SharedValidationError): string =>
        translateSharedValidationError(t, error)

    const [areasList, setAreasList] = useState<IArea[]>([])
    const [contestList, setContestList] = useState<IContest[]>([])
    const [channel, setChannel] = React.useState<TallySheetVotingChannel | null>(null)
    const [results, setResults] = useState<IAreaContestResults>(() =>
        normalizeAreaContestResults({
            area_id: tallySheet?.area_id || "",
            contest_id: choosenContest?.id ?? tallySheet?.contest_id ?? "",
            total_votes: tallySheet?.content?.total_votes || 0,
            total_valid_votes: tallySheet?.content?.total_valid_votes || 0,
            invalid_votes: tallySheet?.content?.invalid_votes || {},
            total_blank_votes: tallySheet?.content?.total_blank_votes || 0,
            census: tallySheet?.content?.census,
            candidate_results: tallySheet?.content?.candidate_results || {},
        })
    )
    const [invalids, setInvalids] = useState<IInvalidVotes>({})
    const [candidatesResults, setCandidatesResults] = useState<ICandidateResultsExtended[]>([])
    const [areaNameFilter, setAreaNameFilter] = useState<string | null>(null)
    const [contestNameFilter, setContestNameFilter] = useState<string | null>(null)
    const [areaIds, setAreaIds] = useState<Array<string>>([])
    const [sharedValidationErrors, setSharedValidationErrors] = useState<SharedValidationError[]>(
        []
    )
    const {data: areaContests} = useGetList<Sequent_Backend_Area_Contest>(
        "sequent_backend_area_contest",
        {
            filter: {
                tenant_id: election.tenant_id,
                election_event_id: election.election_event_id,
                election_id: election.id,
            },
            pagination: {
                perPage: 10000, // Setting initial larger records size of areas
                page: 1,
            },
        }
    )

    const {data: contests} = useGetList<Sequent_Backend_Contest>("sequent_backend_contest", {
        filter: {
            tenant_id: election.tenant_id,
            election_event_id: election.election_event_id,
            election_id: election.id,
        },
        pagination: {
            perPage: 10000, // Setting initial larger records size of areas
            page: 1,
        },
    })

    const {data: allAreas} = useGetList<Sequent_Backend_Area>("sequent_backend_area", {
        filter: {
            tenant_id: election.tenant_id,
            election_event_id: election.election_event_id,
            election_id: election.id,
        },
        pagination: {
            perPage: 10000, // Setting initial larger records size of areas
            page: 1,
        },
    })

    const {data: areas, refetch: refetchAreas} = useGetList<Sequent_Backend_Area>(
        "sequent_backend_area",
        {
            filter: {
                tenant_id: election.tenant_id,
                election_event_id: election.election_event_id,
                election_id: election.id,
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
        }
    )

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
        let presentation = choosenContest?.presentation as IContestPresentation | undefined
        return presentation?.enable_checkable_lists ?? EEnableCheckableLists.CANDIDATES_AND_LISTS
    }, [choosenContest])

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

        const selectedContestId = choosenContest?.id ?? tallySheet?.contest_id
        if (!selectedContestId) {
            setAreaIds([])
            return
        }

        const matchedAreaContests = getContestMatches(tree, selectedContestId)
        const matchedAreas = matchedAreaContests.map((area) => area.area_id)
        const uniqueAreas: Array<string> = uniqueElements(matchedAreas)

        setAreaIds(uniqueAreas)
    }, [areaContests, allAreas, choosenContest, tallySheet])

    useEffect(() => {
        const tallySaved: string | null = localStorage.getItem("tallySheetData")

        if ((tallySheet || tallySaved) && candidates) {
            const tallySheetTemp = tallySaved ? JSON.parse(tallySaved || "") : tallySheet
            if (tallySheetTemp.content) {
                const contentTemp = normalizeAreaContestResults({...tallySheetTemp.content})
                setInvalids(contentTemp.invalid_votes ?? {})
                if (contentTemp.candidate_results) {
                    let candidatesResultsTemp: ICandidateResultsExtended[] = []
                    for (const candidate of candidates) {
                        let isValid = filterCandidateByCheckableLists(candidate, checkableLists)
                        if (!isValid) {
                            continue
                        }
                        const candidateTemp: ICandidateResultsExtended = {
                            candidate_id: candidate.id,
                            name: aliasRenderer(candidate.presentation),
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
            setChannel(
                tallySheetTemp.channel && isTallySheetVotingChannel(tallySheetTemp.channel)
                    ? tallySheetTemp.channel
                    : null
            )
        }
    }, [tallySheet, candidates, i18n.language])

    useEffect(() => {
        if (election) {
            setResults((prev: IAreaContestResults) => ({
                ...prev,
                contest_id: choosenContest?.id ?? tallySheet?.contest_id ?? "",
            }))
        }
    }, [election])

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
        if (contests) {
            const contestsListTemp: IContest[] = contests?.map((item) => {
                return {
                    id: item.id,
                    label: aliasRenderer(item.presentation),
                }
            })
            setContestList(contestsListTemp)
        }
    }, [contests, i18n.language])

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
                    name: aliasRenderer(candidate.presentation),
                }
                candidatesTemp.push(candidateTemp)
            }
            candidatesTemp.sort(sortFunction)
            setCandidatesResults(candidatesTemp)
        }
    }, [candidates, tallySheet, i18n.language])

    const recalculateTotals = () => {
        const normalizedResults = normalizeAreaContestResults(results)
        const normalizedInvalids =
            normalizeAreaContestResults({...normalizedResults, invalid_votes: invalids})
                .invalid_votes ?? {}
        const totalValidVotes = normalizedResults.total_valid_votes ?? 0
        const totalVotes = totalValidVotes + (normalizedInvalids.total_invalid ?? 0)

        const newResults = {...normalizedResults, total_votes: totalVotes}

        const candidateResultsForValidation: {[id: string]: ICandidateResults} = {}
        for (const candidateResult of candidatesResults) {
            candidateResultsForValidation[candidateResult.candidate_id] = {
                candidate_id: candidateResult.candidate_id,
                total_votes: candidateResult.total_votes,
            }
        }

        const contestPresentation = choosenContest?.presentation as IContestPresentation | undefined
        const sharedValidationErrors = validateAreaContestResults(
            {
                ...newResults,
                invalid_votes: normalizedInvalids,
                candidate_results: candidateResultsForValidation,
            },
            {
                max_votes: choosenContest?.max_votes,
                counting_algorithm: choosenContest?.counting_algorithm,
                cumulative_number_of_checkboxes:
                    contestPresentation?.cumulative_number_of_checkboxes,
            }
        )

        const codes = new Set(sharedValidationErrors.map((error) => error.code))
        setTotalValidError(codes.has("invalid_total_valid_votes"))
        setCensusError(codes.has("total_votes_exceeds_census"))
        setSharedValidationMessages(
            sharedValidationErrors
                .filter(
                    (error) =>
                        error.code !== "invalid_total_valid_votes" &&
                        error.code !== "total_votes_exceeds_census"
                )
                .map((error) => error.message)
        )

        setSharedValidationErrors(sharedValidationErrors)
        setIsButtonDisabled(sharedValidationErrors.length > 0)

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
        choosenContest,
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

    const handleContestChange = (
        event: React.SyntheticEvent,
        value: IContest | null,
        reason: AutocompleteChangeReason,
        details?: AutocompleteChangeDetails
    ) => {
        setResults((prev: IAreaContestResults) => ({
            ...prev,
            contest_id: value?.id as any,
        }))

        setChoosenContest(contests?.find((contest) => contest.id === value?.id))
    }

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
                [event.target.name as string]: undefined,
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

    const areaSearchTimeoutId = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const debouncedSearchArea = (event: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = event.target
        clearTimeout(areaSearchTimeoutId.current)
        areaSearchTimeoutId.current = setTimeout(() => {
            setAreaNameFilter(value ? value.trim() : null)
            refetchAreas()
        }, 350)
    }

    const contestSearchTimeoutId = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const debouncedSearchContest = (event: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = event.target
        clearTimeout(contestSearchTimeoutId.current)
        contestSearchTimeoutId.current = setTimeout(() => {
            setContestNameFilter(value ? value.trim() : null)
        }, 350)
    }

    const onSubmit: SubmitHandler<FieldValues> = async (result) => {
        const candidatesResultsTemp: {[id: string]: ICandidateResults} = {}
        for (const candidate of candidatesResults) {
            const candidateTemp: ICandidateResults = {
                candidate_id: candidate.candidate_id,
                total_votes: candidate.total_votes,
            }
            candidatesResultsTemp[candidate.candidate_id] = candidateTemp
        }
        const resultsTemp = normalizeAreaContestResults({
            ...results,
            invalid_votes: invalids,
            candidate_results: candidatesResultsTemp,
        })

        const tallySheetData:
            | Sequent_Backend_Tally_Sheet
            | Sequent_Backend_Tally_Sheet_Insert_Input = {
            tenant_id: election.tenant_id,
            election_event_id: election.election_event_id,
            election_id: election.id,
            contest_id: choosenContest?.id ?? tallySheet?.contest_id,
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

    let currentContest = useMemo(
        () => contestList.find((contest) => contest.id === results?.contest_id) || null,
        [results?.contest_id, contestList]
    )

    const filteredContestList = useMemo(() => {
        if (!contestNameFilter) {
            return contestList
        }
        const needle = contestNameFilter.toLowerCase()
        return contestList.filter((contest) => contest.label?.toLowerCase().includes(needle))
    }, [contestList, contestNameFilter])

    useEffect(() => {
        if (choosenContest) {
            refetchCandidates()
        }
    }, [choosenContest])

    const totalValidVotesError = sharedValidationErrors.find(
        (error) => error.code === "invalid_total_valid_votes"
    )
    const censusExceededError = sharedValidationErrors.find(
        (error) => error.code === "total_votes_exceeds_census"
    )
    const otherValidationErrors = sharedValidationErrors.filter(
        (error) =>
            error.code !== "invalid_total_valid_votes" &&
            error.code !== "total_votes_exceeds_census"
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
                        onChange={handleContestChange as any}
                        options={filteredContestList ?? []}
                        getOptionLabel={(option) =>
                            typeof option.label === "string" ? option.label : option.id
                        }
                        renderInput={(params) => (
                            <TextField
                                {...params}
                                label="Search Contest"
                                onChange={debouncedSearchContest}
                                value={contestNameFilter}
                                required
                            />
                        )}
                        value={currentContest}
                        isOptionEqualToValue={(a, b) => a.id === b.id}
                    />
                </FormControl>

                <FormControl fullWidth size="small">
                    <Autocomplete
                        sx={{width: 300}}
                        onChange={handleChange as any}
                        options={areasList ?? []}
                        getOptionLabel={(option) =>
                            typeof option.label === "string" ? option.label : option.id
                        }
                        renderInput={(params) => (
                            <TextField
                                {...params}
                                label="Search Area"
                                onChange={debouncedSearchArea}
                                value={areaNameFilter}
                                required
                            />
                        )}
                        value={currentArea}
                        isOptionEqualToValue={(a, b) => a.id === b.id}
                    />
                </FormControl>

                <FormControl size="small" sx={{width: 300}}>
                    <InputLabel>{t("tallysheet.label.channel")}</InputLabel>
                    <Select
                        name="channel"
                        value={channel || ""}
                        label={String(t("tallysheet.label.channel"))}
                        onChange={(e: SelectChangeEvent) => {
                            const value = e.target.value
                            setChannel(isTallySheetVotingChannel(value) ? value : null)
                        }}
                        required
                    >
                        {votingChannels.map((votingChannel) => (
                            <MenuItem key={votingChannel} value={votingChannel}>
                                {votingChannel}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>

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
                    {totalValidVotesError && (
                        <StyledError>{translateValidationError(totalValidVotesError)}</StyledError>
                    )}
                    {otherValidationErrors.map((error) => (
                        <StyledError key={error.code + error.field}>
                            {translateValidationError(error)}
                        </StyledError>
                    ))}
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
                    {censusExceededError && (
                        <StyledError>{translateValidationError(censusExceededError)}</StyledError>
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
