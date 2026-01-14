import React, {LegacyRef} from "react"
import {SimpleForm} from "react-admin"
import {FieldValues, SubmitHandler} from "react-hook-form"
import {
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
} from "@/gql/graphql"
import {IAreaContestResults, ICandidateResults} from "@/types/TallySheets"
import {TallySheetsDataFields} from "./TallySheetsDataFields"
import {useTallySheetsDataState} from "./hooks/useTallySheetDataState"

interface TallySheetsDataStepProps {
    election: Sequent_Backend_Election
    choosenContest?: Sequent_Backend_Contest
    submitRef: LegacyRef<HTMLButtonElement> | undefined
    setIsButtonDisabled: (disabled: boolean) => void
    submitDataStep: (results: IAreaContestResults) => void
    tallySheet?: Sequent_Backend_Tally_Sheet
}

export const TallySheetsDataStep = (props: TallySheetsDataStepProps) => {
    const {election, submitRef, setIsButtonDisabled, choosenContest, submitDataStep, tallySheet} =
        props

    const {results, invalids, candidatesResults, totalValidError, censusError, handlers} =
        useTallySheetsDataState({
            election,
            choosenContest,
            setIsButtonDisabled,
            editable: true,
            tallySheet,
        })

    const onSubmit: SubmitHandler<FieldValues> = async () => {
        const resultsTemp = {...results}
        const invalidsTemp = {...invalids}

        const candidatesResultsTemp: Record<string, ICandidateResults> = {}
        for (const c of candidatesResults) {
            candidatesResultsTemp[c.candidate_id] = {
                candidate_id: c.candidate_id,
                total_votes: c.total_votes,
            }
        }

        resultsTemp.invalid_votes = invalidsTemp
        resultsTemp.candidate_results = candidatesResultsTemp

        submitDataStep(resultsTemp)
    }

    return (
        <SimpleForm toolbar={false} onSubmit={onSubmit}>
            <>
                <TallySheetsDataFields
                    results={results}
                    invalids={invalids}
                    candidatesResults={candidatesResults}
                    totalValidError={totalValidError}
                    censusError={censusError}
                    isEditable
                    onNumberChange={handlers.handleNumberChange}
                    onCensusChange={handlers.handleCensusChange}
                    onInvalidChange={handlers.handleInvalidChange}
                    onCandidateChange={handlers.handleCandidateChange}
                />

                <button ref={submitRef} type="submit" style={{display: "none"}} />
            </>
        </SimpleForm>
    )
}
