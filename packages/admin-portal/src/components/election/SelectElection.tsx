// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {SxProps} from "@mui/material"
import {AutocompleteInput, Identifier, ReferenceInput, InputProps} from "react-admin"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

interface SelectElectionProps extends InputProps {
    tenantId: string | null
    electionEventId: string | Identifier | undefined
    source: string
    label?: string
    onSelectElection?: (electionId: string) => void
    customStyle?: SxProps
    disabled?: boolean
    value?: string | null
}

const SelectElection = ({
    tenantId,
    electionEventId,
    source,
    label,
    onSelectElection,
    customStyle,
    isRequired,
    disabled,
    value,
}: SelectElectionProps) => {
    isRequired = isRequired === undefined ? true : isRequired
    const aliasRenderer = useAliasRenderer()
    const electionFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length === 0) {
            return {name: ""}
        }
        return {"name@_ilike,alias@_ilike": searchText.trim()}
    }

    return (
        <ReferenceInput
            fullWidth={true}
            reference="sequent_backend_election"
            source={source}
            filter={{
                tenant_id: tenantId,
                election_event_id: electionEventId,
            }}
            perPage={200} // Setting initial larger records size of elements
            enableGetChoices={({q}) => q && q.length >= 3}
            label={label}
            disabled={disabled}
            required={isRequired}
            value={value}
            defaultValue={value}
            sort={{field: "alias", order: "ASC"}}
        >
            <AutocompleteInput
                TextFieldProps={{required: isRequired}}
                label={label}
                fullWidth={true}
                optionText={aliasRenderer}
                filterToQuery={electionFilterToQuery}
                onChange={onSelectElection}
                debounce={300}
                sx={customStyle as any}
                disabled={disabled}
            />
        </ReferenceInput>
    )
}

export default SelectElection
