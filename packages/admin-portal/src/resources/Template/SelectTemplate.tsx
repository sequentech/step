import React, { useEffect } from "react";
import { SxProps } from "@mui/material";
import { AutocompleteInput, useDataProvider, useGetList, required } from "react-admin";
import { ITemplateType } from "@/types/templates";

interface SelectTemplateProps {
    tenantId: string | null;
    templateType: ITemplateType | undefined;
    source: string;
    label?: string;
    onSelectTemplate?: (template: { alias: string }) => void;
    customStyle?: SxProps;
    disabled?: boolean;
    value?: string | null;
    isRequired?: boolean;
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
    const dataProvider = useDataProvider();

    const { data: templates, isLoading } = useGetList(
        "sequent_backend_template",
        {
            filter: {
                tenant_id: tenantId,
                type: templateType,
            },
            sort: { field: "template.name", order: "ASC" },
            pagination: { page: 1, perPage: 100 },
        }
    );

    const choices = templates
        ? templates.map((template) => ({
            id: template.alias, // Use alias as id
            name: template.template.name,
        }))
        : [];

    const handleTemplateChange = (alias: string) => {
        if (onSelectTemplate) {
            onSelectTemplate({ alias });
        }
    };

    return (
        <AutocompleteInput
            source={source}
            label={label}
            fullWidth={true}
            choices={choices}
            onChange={handleTemplateChange}
            debounce={100}
            sx={customStyle}
            disabled={disabled}
            validate={isRequired ? [required()] : undefined}
            isLoading={isLoading}
            optionValue="id" // alias is used as id
            optionText="name"
            defaultValue={value || ""}
        />
    );
};

export default SelectTemplate;
