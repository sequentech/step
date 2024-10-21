// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {SxProps} from "@mui/material"
import {
    AutocompleteInput,
    Identifier,
    isRequired,
    ReferenceInput,
    useDataProvider,
} from "react-admin"
import {EReportType} from "@/types/reports"
import {ITemplateType} from "@/types/templates"

interface SelectTemplateProps {
    tenantId: string | null
    templateType: ITemplateType | undefined
    source: string
    label?: string
    onSelectTemplate?: (template: {alias: string}) => void
    customStyle?: SxProps
    disabled?: boolean
    value?: string | null
    isRequired?: boolean
}

const SelectTemplate = ({
    tenantId,
    templateType,
    source,
    label,
    onSelectTemplate,
    customStyle,
    disabled,
    value,
    isRequired,
}: SelectTemplateProps) => {
    const dataProvider = useDataProvider()

    const templateFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length === 0) {
            return {"template.name": ""}
        }
        return {"template.name": searchText.trim()}
    }
    const handleTemplateChange = async (id: string) => {
        const {data} = await dataProvider.getOne("sequent_backend_template", {id})
        if (onSelectTemplate && data?.template?.alias) {
            onSelectTemplate({alias: data.template.alias})
        }
    }
    return (
        <ReferenceInput
            required
            fullWidth={true}
            reference="sequent_backend_template"
            source={source}
            filter={{
                tenant_id: tenantId,
                type: templateType,
            }}
            perPage={100}
            label={label}
            disabled={disabled}
            value={value}
            defaultValue={value}
            isRequired={isRequired}
        >
            <AutocompleteInput
                label={label}
                fullWidth={true}
                optionText={(record) => record.template.name}
                filterToQuery={templateFilterToQuery}
                onChange={handleTemplateChange}
                debounce={100}
                sx={customStyle}
                disabled={disabled}
            />
        </ReferenceInput>
    )
}

export default SelectTemplate
