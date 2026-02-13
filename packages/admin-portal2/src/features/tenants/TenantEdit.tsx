import React from 'react';
import { Edit, SimpleForm, TextInput, BooleanInput, TextField, useTranslate } from 'react-admin';
import { Typography, Grid } from '@mui/material';
import { JsonInput } from 'react-admin-json-view';

export const TenantEdit = () => {
    const t = useTranslate();
    
    return (
        <Edit>
            <SimpleForm>
                <Grid container spacing={2}>
                    <Grid size={12}>
                         <Typography variant="h6" gutterBottom>Configuration</Typography>
                    </Grid>
                    <Grid size={{ xs: 12, md: 6 }}>
                        <TextInput source="slug" fullWidth />
                    </Grid>
                    <Grid size={{ xs: 12, md: 6 }}>
                        <BooleanInput source="is_active" />
                    </Grid>
                    <Grid size={12}>
                        <TextField source="id" label="ID" />
                    </Grid>
                    <Grid size={12}>
                        <Typography variant="subtitle2">Labels</Typography>
                        <JsonInput
                            source="labels"
                            reactJsonOptions={{ collapsed: true, name: null }}
                        />
                    </Grid>
                    <Grid size={12}>
                        <Typography variant="subtitle2">Annotations</Typography>
                         <JsonInput
                            source="annotations"
                            reactJsonOptions={{ collapsed: true, name: null }}
                        />
                    </Grid>
                </Grid>
            </SimpleForm>
        </Edit>
    );
};
