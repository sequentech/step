// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {SxProps, Theme} from "@mui/material"
import {AutocompleteInput, Identifier, ReferenceInput} from "react-admin"

interface SelectContestProps {
    tenantId: string | null
    electionEventId: string | Identifier | undefined
    electionId: string | Identifier | undefined
    source: string
    label?: string
    onSelectContest?: (...event: any[]) => void
    customStyle?: SxProps<Theme>
    isRequired?: boolean
    disabled?: boolean
}

const SelectContest = ({
    tenantId,
    electionEventId,
    electionId,
    source,
    label,
    onSelectContest,
    customStyle,
    isRequired,
    disabled,
}: SelectContestProps) => {
    const contestFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {name: ""}
        }
        return {name: searchText.trim()}
    }

    return (
        <ReferenceInput
            fullWidth={true}
            reference="sequent_backend_contest"
            source={source}
            isRequired={isRequired}
            filter={{
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
            }}
            perPage={100} // // Setting initial larger records size of contests
            enableGetChoices={({q}) => q && q.length >= 3}
            label={label}
        >
            <AutocompleteInput
                TextFieldProps={{required: isRequired}}
                label={label}
                fullWidth={true}
                optionText={(contest) => contest.name}
                filterToQuery={contestFilterToQuery}
                onChange={onSelectContest}
                debounce={100}
                sx={customStyle as any}
                isRequired={isRequired}
                disabled={disabled}
            />
        </ReferenceInput>
    )
}

export default SelectContest
