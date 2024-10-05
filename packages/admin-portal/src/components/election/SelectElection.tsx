// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {SxProps} from "@mui/material"
import {AutocompleteInput, Identifier, ReferenceInput} from "react-admin"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

interface SelectElectionProps {
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
    disabled,
    value,
}: SelectElectionProps) => {
    const aliasRenderer = useAliasRenderer()
    const electionFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length === 0) {
            return {name: ""}
        }
        return {name: searchText.trim()}
    }

    return (
        <ReferenceInput
            required
            fullWidth={true}
            reference="sequent_backend_election"
            source={source}
            filter={{
                tenant_id: tenantId,
                election_event_id: electionEventId,
            }}
            perPage={100} // // Setting initial larger records size of areas
            enableGetChoices={({q}) => q && q.length >= 3}
            label={label}
            disabled={disabled}
            value={value}
            defaultValue={value}
        >
            <AutocompleteInput
                label={label}
                fullWidth={true}
                optionText={aliasRenderer}
                filterToQuery={electionFilterToQuery}
                onChange={onSelectElection}
                debounce={100}
                sx={customStyle}
                disabled={disabled}
            />
        </ReferenceInput>
    )
}

export default SelectElection
