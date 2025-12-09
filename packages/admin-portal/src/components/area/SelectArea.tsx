// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {SxProps, Theme} from "@mui/material"
import {AutocompleteInput, Identifier, ReferenceInput} from "react-admin"

interface SelectAreaProps {
    tenantId: string | null
    electionEventId: string | Identifier | undefined
    source: string
    label?: string
    onSelectArea?: (...event: any[]) => void
    customStyle?: SxProps<Theme>
    isRequired?: boolean
    disabled?: boolean
}

const SelectArea = ({
    tenantId,
    electionEventId,
    source,
    label,
    onSelectArea,
    customStyle,
    isRequired,
    disabled,
}: SelectAreaProps) => {
    const areaFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length == 0) {
            return {name: ""}
        }
        return {name: searchText.trim()}
    }

    return (
        <ReferenceInput
            fullWidth={true}
            reference="sequent_backend_area"
            source={source}
            isRequired={isRequired}
            filter={{
                tenant_id: tenantId,
                election_event_id: electionEventId,
            }}
            perPage={100} // // Setting initial larger records size of areas
            enableGetChoices={({q}) => q && q.length >= 3}
            label={label}
        >
            <AutocompleteInput
                TextFieldProps={{required: isRequired}}
                label={label}
                fullWidth={true}
                optionText={(area) => area.name}
                filterToQuery={areaFilterToQuery}
                onChange={onSelectArea}
                debounce={100}
                sx={customStyle as any}
                isRequired={isRequired}
                disabled={disabled}
            />
        </ReferenceInput>
    )
}

export default SelectArea
