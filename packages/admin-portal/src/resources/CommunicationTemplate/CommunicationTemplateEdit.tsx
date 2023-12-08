import React from "react"

import { Typography } from "@mui/material"
import { SimpleForm, Edit, TextField, TextInput } from "react-admin"

import { CommunicationTemplateList } from './CommunicationTemplateList'

const CommunicationTemplateForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Communication Template</Typography>
            <Typography variant="body2">Edit communication</Typography>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
            <TextInput source="name" />
        </SimpleForm>
    )
}

export const CommunicationTemplateEdit: React.FC = () => {
    return (
        <CommunicationTemplateList
            aside={
                <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <CommunicationTemplateForm />
                </Edit>
            }
        />
    )
}
