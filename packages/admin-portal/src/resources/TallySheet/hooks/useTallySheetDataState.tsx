// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {useInfiniteGetList} from "react-admin"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
    Sequent_Backend_Tally_Sheet_Insert_Input,
} from "@/gql/graphql"
import {IAreaContestResults, IInvalidVotes} from "@/types/TallySheets"
import {EEnableCheckableLists, IContestPresentation} from "@sequentech/ui-core"
import {filterCandidateByCheckableLists} from "@/services/CandidatesFilter"
import {sortFunction} from "../utils"
import {ICandidateResultsExtended} from "../TallySheetsDataFields"

const numbersRegExp = /^[0-9]+$/

type Params = {
    election: Sequent_Backend_Election
    choosenContest?: Sequent_Backend_Contest
    tallySheet?: Sequent_Backend_Tally_Sheet | Sequent_Backend_Tally_Sheet_Insert_Input
    setIsButtonDisabled?: (disabled: boolean) => void
    editable: boolean
}

export function useTallySheetsDataState(params: Params) {
    const {election, choosenContest, tallySheet, setIsButtonDisabled, editable} = params

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
    const [totalValidError, setTotalValidError] = useState(false)
    const [censusError, setCensusError] = useState(false)

    const checkableLists = useMemo(() => {
        const presentation = election.presentation as IContestPresentation | undefined
        return presentation?.enable_checkable_lists ?? EEnableCheckableLists.CANDIDATES_AND_LISTS
    }, [election.presentation])

    const {
        data: fetchedCandidates,
        hasNextPage,
        fetchNextPage,
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

    const candidates = useMemo(() => {
        // force fetch all pages
        hasNextPage && fetchNextPage()
        return fetchedCandidates?.pages.flatMap((p) => p.data) ?? []
    }, [fetchedCandidates, hasNextPage, fetchNextPage])

    // init from tallySheet/localStorage (only once candidates exist)
    useEffect(() => {
        const tallySaved = localStorage.getItem("tallySheetData")

        if ((tallySheet || tallySaved) && candidates.length) {
            const tallySheetTemp = tallySaved ? JSON.parse(tallySaved) : tallySheet
            if (tallySheetTemp?.content) {
                const contentTemp: IAreaContestResults = {...tallySheetTemp.content}

                if (contentTemp.invalid_votes) setInvalids({...contentTemp.invalid_votes})

                if (contentTemp.candidate_results) {
                    const list: ICandidateResultsExtended[] = []
                    for (const c of candidates) {
                        if (!filterCandidateByCheckableLists(c, checkableLists)) continue
                        list.push({
                            candidate_id: c.id,
                            name: c.name as string,
                            total_votes: contentTemp.candidate_results[c.id]?.total_votes,
                        })
                    }
                    list.sort(sortFunction)
                    setCandidatesResults(list)
                }

                setResults(contentTemp)
            }
        }
    }, [tallySheet, candidates, checkableLists])

    // init candidates for NEW form (no tally)
    useEffect(() => {
        const tallySaved = localStorage.getItem("tallySheetData")
        if (!(tallySheet || tallySaved) && candidates.length) {
            const list: ICandidateResultsExtended[] = []
            for (const c of candidates) {
                if (!filterCandidateByCheckableLists(c, checkableLists)) continue
                list.push({candidate_id: c.id, name: c.name as string})
            }
            list.sort(sortFunction)
            setCandidatesResults(list)
        }
    }, [candidates, tallySheet, checkableLists])

    const setButtonDisabled = (value: boolean) => {
        if (setIsButtonDisabled) {
            setIsButtonDisabled(value)
        }
    }

    useEffect(() => {
        if (!editable) {
            setButtonDisabled(false)
            setTotalValidError(false)
            setCensusError(false)
            return
        }

        const newResults = {...results}
        const totalValidVotes = newResults.total_valid_votes ?? 0
        const totalBlankVotes = newResults.total_blank_votes ?? 0
        const totalVotes = totalValidVotes + (invalids?.total_invalid ?? 0)

        newResults.total_valid_votes = totalValidVotes
        newResults.total_votes = totalVotes

        let disableNextButton = false
        if (newResults.census !== undefined && newResults.census < newResults.total_votes) {
            disableNextButton = true
            setCensusError(true)
        } else {
            setCensusError(false)
        }

        let candidatesVotesSum = 0
        for (const cr of candidatesResults) candidatesVotesSum += cr.total_votes ?? 0

        if (candidatesVotesSum + totalBlankVotes !== totalValidVotes) {
            disableNextButton = true
            setTotalValidError(true)
        } else {
            setTotalValidError(false)
        }

        setButtonDisabled(disableNextButton)

        if (JSON.stringify(newResults) !== JSON.stringify(results)) {
            setResults(newResults)
        }
    }, [
        editable,
        results,
        candidatesResults,
        results.total_blank_votes,
        results.total_valid_votes,
        invalids?.total_invalid,
        invalids,
        setIsButtonDisabled,
    ])

    const handleNumberChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        if (!editable) return

        if (event.target.value === "") {
            setResults((prev) => ({...prev, [event.target.name]: "" as any}))
            return
        }
        if (event.target.value === "0") {
            setResults((prev) => ({...prev, [event.target.name]: 0 as any}))
            return
        }
        if (event.target.value.match(numbersRegExp)) {
            setResults((prev) => ({...prev, [event.target.name]: +event.target.value}))
        }
    }

    const handleCensusChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        if (!editable) return
        const census = event.target.value.match(numbersRegExp) ? Number(event.target.value) : 0
        setResults((prev) => ({...prev, census}))
    }

    const handleInvalidChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        if (!editable) return

        const key = event.target.name as "explicit_invalid" | "implicit_invalid"
        const next = {...invalids}

        if (event.target.value === "") next[key] = 0
        else if (event.target.value.match(numbersRegExp)) next[key] = Number(event.target.value)

        next.total_invalid = (next.explicit_invalid ?? 0) + (next.implicit_invalid ?? 0)
        setInvalids(next)
    }

    const handleCandidateChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        if (!editable) return

        const id = event.target.id
        const current = candidatesResults.find((c) => c.candidate_id === id)
        if (!current) return

        const rest = candidatesResults.filter((c) => c.candidate_id !== id)

        if (!event.target.value) {
            delete current.total_votes
        } else if (event.target.value.match(numbersRegExp)) {
            current.total_votes = +event.target.value
        } else {
            current.total_votes = +(current.total_votes ?? 0)
        }

        const next = [...rest, current].sort((a, b) => a.name.localeCompare(b.name))
        setCandidatesResults(next)
    }

    return {
        results,
        invalids,
        candidatesResults,
        totalValidError,
        censusError,
        handlers: {
            handleNumberChange,
            handleCensusChange,
            handleInvalidChange,
            handleCandidateChange,
        },
    }
}
