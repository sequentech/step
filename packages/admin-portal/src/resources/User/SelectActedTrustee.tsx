import { GET_TRUSTEES_NAMES } from '@/queries/GetTrusteesNames';
import { useQuery } from '@apollo/client';
import { SxProps } from '@mui/material';
import React from 'react';

import {AutocompleteInput, Identifier, ReferenceInput, SelectInput} from "react-admin";
interface SelectActedTrusteeProps {
    tenantId: string | null
    onSelectTrustee: (trustee: {id: string, name: string}) => void
    defaultValue?: string | string[];
    source: string;
    customStyle?: SxProps;
}
const SelectActedTrustee: React.FC<SelectActedTrusteeProps> = ({tenantId
    , onSelectTrustee, defaultValue, source, customStyle
}) => {
    const {data: trustees} = useQuery(GET_TRUSTEES_NAMES, {
        variables: {
            tenantId: tenantId,
        },
    })
    const handleSelectTrustee = (event: any) => {
        const selectedTrusteeName = trustees?.sequent_backend_trustee.find((trustee: any) => trustee.id === event.target.value);
        onSelectTrustee(selectedTrusteeName);
    }

    const getDefaultValue = () => {
        const value = defaultValue instanceof Array ? defaultValue[0] : defaultValue;
        if (value) { 
            const trustee = trustees?.sequent_backend_trustee.find((trustee: {id: string, name: string}) => trustee.name == defaultValue);
            return trustee?.id;
        }
    }

    return (
        <SelectInput
        source={source} 
        onChange={handleSelectTrustee}
        optionValue='id'
        optionText={(trustee) => trustee.name}
        defaultValue={getDefaultValue()}
        choices={trustees && trustees.sequent_backend_trustee ? trustees.sequent_backend_trustee as any[] : []}
        label={"Acted Trustee"}
        sx={{minHeight: 50}}
        />
    )
}

export default SelectActedTrustee