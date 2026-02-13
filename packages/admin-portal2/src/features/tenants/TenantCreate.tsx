import React from 'react';
import { Create, SimpleForm, TextInput } from 'react-admin';
import { Typography } from '@mui/material';
import { useTranslation } from 'react-i18next';

export const TenantCreate = () => {
    const { t } = useTranslation();
    
    return (
        <Create redirect="list">
            <SimpleForm>
                <Typography variant="h6" gutterBottom>
                    {t('tenantScreen.common.title')}
                </Typography>
                 <TextInput source="slug" fullWidth />
            </SimpleForm>
        </Create>
    );
};
