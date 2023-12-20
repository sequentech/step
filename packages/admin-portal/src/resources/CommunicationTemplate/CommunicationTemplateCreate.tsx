import React from "react"

import {FormStyles} from "@/components/styles/FormStyles"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {FormControl, Select, MenuItem, InputLabel} from "@mui/material"

import {
    Create,
    SimpleForm,
    TextInput,
    Edit,
    required,
    useTranslate,
    Toolbar,
    SaveButton,
} from "react-admin"

type TCommunicationTemplateCreate = {
    close?: () => void
}

export const CommunicationTemplateCreate: React.FC<TCommunicationTemplateCreate> = () => {
    return (
        <Create title={"CREATE"}>
            <SimpleForm>
                <PageHeaderStyles.Title>RECIBO EMAIL AND PDF</PageHeaderStyles.Title>

                <FormStyles.TextInput source="alias" label="Alias" validate={required()} />

                <FormControl fullWidth>
                    <InputLabel id="demo-simple-select-label">Age</InputLabel>
                    <Select
                        labelId="demo-simple-select-label"
                        id="demo-simple-select"
                        value={10}
                        label="Age"
                        onChange={() => null}
                    >
                        <MenuItem value={10}>Ten</MenuItem>
                        <MenuItem value={20}>Twenty</MenuItem>
                        <MenuItem value={30}>Thirty</MenuItem>
                    </Select>
                </FormControl>
            </SimpleForm>
        </Create>
    )
}
