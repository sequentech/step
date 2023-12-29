// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    Identifier,
    RecordContext,
    SaveButton,
    SimpleForm,
    TextInput,
    useGetList,
    useNotify,
    useRefresh,
} from "react-admin"
import {useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    Sequent_Backend_Area,
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

const votingChannels = [
    {id: "PAPER", name: "PAPER"},
    {id: "POSTAL", name: "POSTAL"},
]

interface EditTallySheetProps {
    contest: Sequent_Backend_Contest
    id?: Identifier | undefined
    doSelectArea?: (areaId: Identifier) => void
    doCreatedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet_Insert_Input) => void
    doEditedTalySheet?: (tallySheet: Sequent_Backend_Tally_Sheet) => void
    submitRef: any
}

interface ICandidateResultsExtended extends ICandidateResults {
    name: string
}

export const EditTallySheet: React.FC<EditTallySheetProps> = (props) => {
    const {id, contest, doCreatedTalySheet, submitRef} = props

    const refresh = useRefresh()
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [renderUI, setRenderUI] = useState(false)
    const [areasList, setAreasList] = useState<Sequent_Backend_Area[]>([])
    const [area, setArea] = React.useState<string | null>(null)
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

    const {data: candidates} = useGetList("sequent_backend_candidate", {
        filter: {
            contest_id: contest.id,
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
        },
    })

    console.log("candidates", candidates)

    useEffect(() => {
        console.log("results", results)
    }, [results])

    useEffect(() => {
        console.log("invalids", invalids)
    }, [invalids])

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
            const candidatesTemp = []
            for (const candidate of candidates) {
                const candidateTemp: ICandidateResultsExtended = {
                    candidate_id: candidate.id,
                    name: candidate.name,
                }
                candidatesTemp.push(candidateTemp)
            }
            candidatesTemp.sort((a, b) => a.name.localeCompare(b.name))
            setCandidatesResults(candidatesTemp)
        }
    }, [candidates])

    useEffect(() => {
        if (areas) {
            const areatListTemp = areas.sequent_backend_area_contest.map(
                (item: {area: {id: string; name: string}}) => {
                    return {
                        id: item.area.id,
                        name: item.area.name,
                    }
                }
            )
            setAreasList(areatListTemp)
        }
    }, [areas])

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
            finalCandidates.sort((a, b) => a.name.localeCompare(b.name))
            setCandidatesResults(finalCandidates)
        }
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

        const tallySheetData: Sequent_Backend_Tally_Sheet_Insert_Input = {
            tenant_id: contest.tenant_id,
            election_event_id: contest.election_event_id,
            election_id: contest.election_id,
            contest_id: contest.id,
            area_id: resultsTemp.area_id,
            channel: channel || "",
            content: resultsTemp,
        }

        if (doCreatedTalySheet) {
            localStorage.setItem("tallySheetData", JSON.stringify(tallySheetData))
            doCreatedTalySheet(tallySheetData)
        }
    }

    const parseValues = (incoming: any) => {
        const temp = {...incoming}

        return temp
    }

    if (id) {
        return (
            <PageHeaderStyles.Wrapper>
                <RecordContext.Consumer>
                    {(incoming) => {
                        const parsedValue = parseValues(incoming)
                        console.log("parsedValue :>> ", parsedValue)
                        return (
                            <SimpleForm
                                record={parsedValue}
                                toolbar={<SaveButton />}
                                onSubmit={onSubmit}
                            >
                                <>
                                    <PageHeaderStyles.Title>
                                        {t("areas.common.title")}
                                    </PageHeaderStyles.Title>
                                    <PageHeaderStyles.SubTitle>
                                        {t("areas.common.subTitle")}
                                    </PageHeaderStyles.SubTitle>
                                    <FormControl fullWidth size="small">
                                        <InputLabel>{t("tallysheet.label.area")}</InputLabel>
                                        <Select
                                            value={area || ""}
                                            label={t("tallysheet.label.area")}
                                            onChange={handleChange}
                                        >
                                            {areasList.map((item) => (
                                                <MenuItem key={item.id} value={item.id}>
                                                    {item.name}
                                                </MenuItem>
                                            ))}
                                        </Select>
                                    </FormControl>{" "}
                                    <TextInput source="name" />
                                    <TextInput source="description" />
                                </>
                            </SimpleForm>
                        )
                    }}
                </RecordContext.Consumer>
            </PageHeaderStyles.Wrapper>
        )
    } else {
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
                        >
                            {votingChannels.map((item) => (
                                <MenuItem key={item.id} value={item.id}>
                                    {item.name}
                                </MenuItem>
                            ))}
                        </Select>
                    </FormControl>

                    <PageHeaderStyles.Wrapper>
                        <PageHeaderStyles.Title>
                            {t("tallysheet.common.data")}
                        </PageHeaderStyles.Title>
                    </PageHeaderStyles.Wrapper>

                    <TextField
                        label={t("tallysheet.label.contest_id")}
                        name="constest_id"
                        value={results.contest_id}
                        onChange={handleTextChange}
                        size="small"
                        style={{display: "none"}}
                    />

                    <TextField
                        label={t("tallysheet.label.total_votes")}
                        name="total_votes"
                        value={results.total_votes}
                        onChange={handleTextChange}
                        size="small"
                        type="number"
                    />
                    <TextField
                        label={t("tallysheet.label.total_valid_votes")}
                        name="total_valid_votes"
                        value={results.total_valid_votes}
                        onChange={handleTextChange}
                        size="small"
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
                            type="number"
                        />
                        <TextField
                            label={t("tallysheet.label.implicit_invalid")}
                            name="implicit_invalid"
                            value={invalids.implicit_invalid}
                            onChange={handleInvalidChange}
                            size="small"
                            type="number"
                        />
                        <TextField
                            label={t("tallysheet.label.explicit_invalid")}
                            name="explicit_invalid"
                            value={invalids.explicit_invalid}
                            onChange={handleInvalidChange}
                            size="small"
                            type="number"
                        />
                    </Box>

                    <TextField
                        label={t("tallysheet.label.total_blank_votes")}
                        name="total_blank_votes"
                        value={results.total_blank_votes}
                        onChange={handleTextChange}
                        size="small"
                        type="number"
                    />
                    <TextField
                        label={t("tallysheet.label.census")}
                        name="census"
                        value={results.census}
                        onChange={handleTextChange}
                        size="small"
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
                                type="number"
                            />
                        </Box>
                    ))}

                    <button ref={submitRef} type="submit" style={{display: "none"}} />
                </>
            </SimpleForm>
        )
    }
}
