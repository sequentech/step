import React from "react"

import { Typography } from "@mui/material"
import { SimpleForm, TextInput, Create } from "react-admin"

export const CommunicationTemplateCreate: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Communication Template</Typography>
                <Typography variant="body2">Create communication</Typography>
                <TextInput source="name" />
            </SimpleForm>
        </Create>
    )
}
