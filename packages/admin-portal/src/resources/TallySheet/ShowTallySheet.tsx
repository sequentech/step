// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {Identifier, SimpleForm, useCreate, useGetList, useNotify, useUpdate} from "react-admin"
import {useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {
    Maybe,
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
} from "@/gql/graphql"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {GET_CONTESTS_EXTENDED} from "@/queries/GetContestsExtended"
import {
    Box,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
    TextField,
    Typography,
} from "@mui/material"
import {IAreaContestResults, ICandidateResults, IInvalidVotes} from "@/types/TallySheets"
import {sortFunction} from "./utils"
import {EEnableCheckableLists, IContestPresentation} from "@sequentech/ui-core"
import {filterCandidateByCheckableLists} from "@/services/CandidatesFilter"

const votingChannels = [
    {id: "PAPER", name: "PAPER"},
    {id: "POSTAL", name: "POSTAL"},
]

interface ShowTallySheetProps {
    tallySheet?: Sequent_Backend_Tally_Sheet | Sequent_Backend_Tally_Sheet_Insert_Input
    contest: Sequent_Backend_Contest
    id?: Identifier | undefined
    doSelectArea?: (areaId: Identifier) => void
    doCreatedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input) => void
    doEditedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet) => void
    submitRef: any
}

interface ICandidateResultsExtended extends ICandidateResults {
    name?: Maybe<string> | undefined
}

interface IArea {
    id: string
    name?: Maybe<string> | undefined
}

interface ICandidateResultsExtended extends ICandidateResults {
    name?: Maybe<string> | undefined
}

export const ShowTallySheet: React.FC<ShowTallySheetProps> = (props) => {
    const {contest, submitRef, tallySheet} = props

    const notify = useNotify()
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

    const {data: areas} = useQuery(GET_CONTESTS_EXTENDED, {
        variables: {
            electionEventId: contest.election_event_id,
            contestId: contest.id,
            tenantId: contest.tenant_id,
        },
    })

    const {data: candidates} = useGetList<Sequent_Backend_Candidate>("sequent_backend_candidate", {
        filter: {
            contest_id: contest.id,
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
        },
    })
    const checkableLists = useMemo(() => {
        let presentation = contest.presentation as IContestPresentation | undefined
        return presentation?.enable_checkable_lists ?? EEnableCheckableLists.CANDIDATES_AND_LISTS
    }, [contest.presentation])

    useEffect(() => {
        const tallySaved: string | null = localStorage.getItem("tallySheet")

        if ((tallySheet || tallySaved) && candidates) {
            const tallySheetTemp:
                | Sequent_Backend_Tally_Sheet
                | Sequent_Backend_Tally_Sheet_Insert_Input = tallySheet
                ? {...tallySheet}
                : JSON.parse(tallySaved || "")
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
                            name: candidate.name,
                            total_votes:
                                contentTemp.candidate_results?.[candidate.id]?.total_votes ?? 0,
                        }
                        candidatesResultsTemp.push(candidateTemp)
                    }
                    candidatesResultsTemp.sort(sortFunction)
                    setCandidatesResults(candidatesResultsTemp)
                }
                setResults(contentTemp)
            }

            if (tallySheetTemp.channel) {
                setChannel(tallySheetTemp.channel)
            }
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
        if (candidates) {
            const candidatesTemp: ICandidateResultsExtended[] = []
            for (const candidate of candidates) {
                const candidateTemp: ICandidateResultsExtended = {
                    candidate_id: candidate.id,
                    name: candidate.name,
                }
                candidatesTemp.push(candidateTemp)
            }
        }
    }, [candidates])

    useEffect(() => {
        if (areas) {
            const areatListTemp: IArea[] = areas?.sequent_backend_area_contest?.map(
                (item: {area: IArea}) => {
                    return {
                        id: item.area.id,
                        name: item.area.name,
                    }
                }
            )
            setAreasList(areatListTemp)
        }
    }, [areas])

    useEffect(() => {
        window.scrollTo(0, 0)
    }, [])

    const handleChange = (event: SelectChangeEvent) => {
        // setArea(event.target.value as string)
        setResults((prev: IAreaContestResults) => ({
            ...prev,
            area_id: event.target.value as string,
        }))
    }

    const handleTextChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setResults((prev: IAreaContestResults) => ({
            ...prev,
            [event.target.name as string]: event.target.value as string,
        }))
    }

    const handleInvalidChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        setInvalids((prev: IInvalidVotes) => ({
            ...prev,
            [event.target.name as string]: event.target.value as string,
        }))
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
                candidateTemp.total_votes =
                    event.target.value !== "" ? parseInt(event.target.value) : 0
            }

            const finalCandidates = [...candidateRest, candidateTemp]
            finalCandidates.sort(sortFunction)
            setCandidatesResults(finalCandidates)
        }
    }

    const [create] = useCreate("sequent_backend_tally_sheet")
    const [update] = useUpdate("sequent_backend_tally_sheet")

    const onSubmit: SubmitHandler<FieldValues> = async (result) => {
        const resultsTemp = {...results}
        const invalidsTemp = {...invalids}
        const candidatesResultsTemp: {[id: string]: ICandidateResults} = {}
        for (const candidate of candidatesResults) {
            const candiateTemp: ICandidateResults = {
                candidate_id: candidate.candidate_id,
                total_votes: candidate.total_votes,
            }
            candidatesResultsTemp[candidate.candidate_id] = candiateTemp
        }
        resultsTemp.invalid_votes = invalidsTemp
        resultsTemp.candidate_results = candidatesResultsTemp

        const tallySheetData: Sequent_Backend_Tally_Sheet_Insert_Input = {
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
            election_id: contest.election_id,
            contest_id: contest.id,
            area_id: resultsTemp.area_id,
            channel: channel || "",
            content: resultsTemp,
        }

        if (tallySheet && tallySheet.id) {
            update(
                "sequent_backend_tally_sheet",
                {
                    id: tallySheet.id,
                    data: tallySheetData,
                },
                {
                    onSuccess: () => {
                        notify(t("tallysheet.createTallySuccess"), {type: "success"})
                    },
                    onError: (error) => {
                        notify(t("tallysheet.createTallyError"), {type: "error"})
                    },
                }
            )
        } else {
            create(
                "sequent_backend_tally_sheet",
                {
                    data: tallySheetData,
                },
                {
                    onSuccess: () => {
                        notify(t("tallysheet.createTallySuccess"), {type: "success"})
                    },
                    onError: (error) => {
                        notify(t("tallysheet.createTallyError"), {type: "error"})
                    },
                }
            )
        }
    }

    return (
        <SimpleForm toolbar={false} onSubmit={onSubmit}>
            <>
                <PageHeaderStyles.Title>{t("tallysheet.common.title")}</PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t("tallysheet.common.subtitle")}
                </PageHeaderStyles.SubTitle>

                <FormControl fullWidth size="small">
                    <InputLabel>{t("tallysheet.label.area")}</InputLabel>
                    <Select
                        name="area_id"
                        value={results.area_id || ""}
                        label={t("tallysheet.label.area")}
                        onChange={handleChange}
                        disabled
                    >
                        {areasList.map((item) => (
                            <MenuItem key={item.id} value={item.id}>
                                {item.name}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>

                <FormControl fullWidth size="small">
                    <InputLabel>{t("tallysheet.label.channel")}</InputLabel>
                    <Select
                        name="channel"
                        value={channel || ""}
                        label={t("tallysheet.label.channel")}
                        onChange={(e: SelectChangeEvent) => setChannel(e.target.value)}
                        disabled
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
                    disabled
                    style={{display: "none"}}
                />

                <TextField
                    label={t("tallysheet.label.total_votes")}
                    name="total_votes"
                    value={results.total_votes}
                    onChange={handleTextChange}
                    size="small"
                    disabled
                    type="number"
                />
                <TextField
                    label={t("tallysheet.label.total_valid_votes")}
                    name="total_valid_votes"
                    value={results.total_valid_votes}
                    onChange={handleTextChange}
                    size="small"
                    disabled
                    type="number"
                />

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
                        value={invalids.total_invalid}
                        onChange={handleInvalidChange}
                        size="small"
                        disabled
                        type="number"
                    />
                    <TextField
                        label={t("tallysheet.label.implicit_invalid")}
                        name="implicit_invalid"
                        value={invalids.implicit_invalid}
                        onChange={handleInvalidChange}
                        size="small"
                        disabled
                        type="number"
                    />
                    <TextField
                        label={t("tallysheet.label.explicit_invalid")}
                        name="explicit_invalid"
                        value={invalids.explicit_invalid}
                        onChange={handleInvalidChange}
                        size="small"
                        disabled
                        type="number"
                    />
                </Box>

                <TextField
                    label={t("tallysheet.label.total_blank_votes")}
                    name="total_blank_votes"
                    value={results.total_blank_votes}
                    onChange={handleTextChange}
                    size="small"
                    disabled
                    type="number"
                />
                <TextField
                    label={t("tallysheet.label.census")}
                    name="census"
                    value={results.census}
                    onChange={handleTextChange}
                    size="small"
                    disabled
                    type="number"
                />

                <PageHeaderStyles.Wrapper>
                    <PageHeaderStyles.Title>
                        {t("tallysheet.common.candidates")}
                    </PageHeaderStyles.Title>
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
                            label={t("tallysheet.label.total_votes")}
                            name="total_votes"
                            value={candidate.total_votes}
                            onChange={handleCandidateChange}
                            size="small"
                            disabled
                            type="number"
                        />
                    </Box>
                ))}

                <button ref={submitRef} type="submit" style={{display: "none"}} />
            </>
        </SimpleForm>
    )
}
