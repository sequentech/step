import React from "react"
import {SxProps} from "@mui/material"
import {AutocompleteInput, Identifier, ReferenceInput} from "react-admin"
import {EReportTypes} from "@/types/reports"

interface SelectTemplateProps {
    tenantId: string | null
    templateType: EReportTypes
    source: string
    label?: string
    onSelectTemplate?: (templateId: string) => void
    customStyle?: SxProps
    disabled?: boolean
    value?: string | null
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
}: SelectTemplateProps) => {
    const templateFilterToQuery = (searchText: string) => {
        if (!searchText || searchText.length === 0) {
            return {"template.name": ""}
        }
        return {"template.name": searchText.trim()}
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
        >
            <AutocompleteInput
                label={label}
                fullWidth={true}
                optionText={(record) => record.template.name}
                filterToQuery={templateFilterToQuery}
                onChange={onSelectTemplate}
                debounce={100}
                sx={customStyle}
                disabled={disabled}
            />
        </ReferenceInput>
    )
}

export default SelectTemplate
