// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {LegacyRef, useEffect, useMemo, useState} from "react"
import {SimpleForm, useGetList} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {
    Maybe,
    Sequent_Backend_Area,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
} from "@/gql/graphql"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {
    Autocomplete,
    AutocompleteChangeDetails,
    AutocompleteChangeReason,
    FormControl,
    InputLabel,
    MenuItem,
    Select,
    SelectChangeEvent,
} from "@mui/material"
import TextField from "@mui/material/TextField"
import {ITallySheetConfig} from "@/types/TallySheets"
import {createTree, getContestMatches} from "@/services/AreaService"

export const votingChannels = [
    {id: "PAPER", name: "PAPER"},
    {id: "POSTAL", name: "POSTAL"},
]

interface IArea {
    id: string
    label?: Maybe<string> | undefined
}

interface IContest {
    id: string
    label?: Maybe<string> | undefined
}

interface TallySheetConfigStepProps {
    election: Sequent_Backend_Election
    submitRef: LegacyRef<HTMLButtonElement> | undefined
    currentConfig?: ITallySheetConfig
    setConfig: React.Dispatch<React.SetStateAction<ITallySheetConfig | undefined>>
    setChoosenContest: (contest: Sequent_Backend_Contest | undefined) => void
    setIsButtonDisabled: (disabled: boolean) => void
    version?: number
}

export const TallySheetConfigStep: React.FC<TallySheetConfigStepProps> = (
    props: TallySheetConfigStepProps
) => {
    const {
        election: election,
        submitRef,
        setConfig,
        currentConfig,
        setChoosenContest,
        setIsButtonDisabled,
        version,
    } = props

    const {t} = useTranslation()

    const [areasList, setAreasList] = useState<IArea[]>([])
    const [contestList, setContestList] = useState<IArea[]>([])

    const [channel, setChannel] = React.useState<string | undefined>(currentConfig?.channel)
    const [contestId, setContestId] = React.useState<string | undefined>(currentConfig?.contest_id)
    const [areaId, setAreaId] = React.useState<string | undefined>(currentConfig?.area_id)

    const [areaNameFilter, setAreaNameFilter] = useState<string | null>(null)
    const [contestNameFilter, setContestNameFilter] = useState<string | null>(null)
    const [areaIds, setAreaIds] = useState<Array<string>>([])
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

    const {data: contests, refetch: refetchContests} = useGetList<Sequent_Backend_Contest>(
        "sequent_backend_contest",
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

        const matchedAreaContests = getContestMatches(tree, election.id)
        const matchedAreas = matchedAreaContests.map((area) => area.area_id)
        const uniqueAreas: Array<string> = uniqueElements(matchedAreas)

        setAreaIds(uniqueAreas)
    }, [areaContests, allAreas])

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
                    label: item.name,
                }
            })
            setContestList(contestsListTemp)
        }
    }, [contests])

    useEffect(() => {
        let isEnableNextButton: boolean = !!(areaId && contestId && channel)
        setIsButtonDisabled(!isEnableNextButton)
    }, [areaId, contestId, channel])

    const handleAreaChange = (
        event: React.SyntheticEvent,
        value: IArea,
        reason: AutocompleteChangeReason,
        details?: AutocompleteChangeDetails
    ) => {
        setAreaId(value.id)
    }

    const handleContestChange = (
        event: React.SyntheticEvent,
        value: IContest,
        reason: AutocompleteChangeReason,
        details?: AutocompleteChangeDetails
    ) => {
        setContestId(value.id)
    }

    let timeoutId: ReturnType<typeof setTimeout>
    const debouncedSearchArea = (event: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = event.target
        clearTimeout(timeoutId)
        timeoutId = setTimeout(() => {
            setAreaNameFilter(value ? value.trim() : null)
            refetchAreas()
        }, 350)
    }

    const debouncedSearchContest = (event: React.ChangeEvent<HTMLInputElement>) => {
        const {value} = event.target
        clearTimeout(timeoutId)
        timeoutId = setTimeout(() => {
            setContestNameFilter(value ? value.trim() : null)
            refetchContests()
        }, 350)
    }

    const onSubmit: SubmitHandler<FieldValues> = async (result) => {
        if (contestId && areaId && channel) {
            setConfig({
                contest_id: contestId,
                area_id: areaId,
                channel: channel,
            })
            setChoosenContest(contests?.find((contest) => contest.id === contestId))
        }
    }

    let currentArea = useMemo(
        () => areasList.find((area) => area.id === areaId) || null,
        [areaId, areasList]
    )

    let currentContest = useMemo(
        () => contestList.find((contest) => contest.id === contestId) || null,
        [contestId, contestList]
    )

    return (
        <SimpleForm toolbar={false} onSubmit={onSubmit}>
            <>
                <PageHeaderStyles.Title>{t("tallysheet.common.title")}</PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t("tallysheet.common.subtitle")}
                </PageHeaderStyles.SubTitle>
                {version && (
                    <PageHeaderStyles.SubTitle>Vesrion: {version}</PageHeaderStyles.SubTitle>
                )}

                <FormControl fullWidth size="small" required>
                    <Autocomplete
                        sx={{width: 300}}
                        onChange={handleAreaChange as any}
                        options={areasList ?? []}
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

                <FormControl fullWidth size="small" required>
                    <Autocomplete
                        sx={{width: 300}}
                        onChange={handleContestChange as any}
                        options={contestList ?? []}
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

                <FormControl fullWidth size="small" required>
                    <InputLabel>{t("tallysheet.label.channel")}</InputLabel>
                    <Select
                        name="channel"
                        value={channel || ""}
                        label={String(t("tallysheet.label.channel"))}
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

                <button ref={submitRef} type="submit" style={{display: "none"}} />
            </>
        </SimpleForm>
    )
}
