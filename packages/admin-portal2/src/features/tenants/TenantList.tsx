import React from 'react';
import {
    List,
    Datagrid,
    TextField,
    TextInput,
    FilterButton,
    TopToolbar,
    CreateButton,
    ExportButton,
    SelectColumnsButton,
    useRecordContext,
    BooleanInput
} from 'react-admin';
import { Chip } from '@mui/material';

const TenantListActions = () => (
    <TopToolbar>
        <FilterButton />
        <SelectColumnsButton />
        <CreateButton />
        <ExportButton />
    </TopToolbar>
);

const tenantFilters = [
    <TextInput label="Slug" source="slug" alwaysOn />,
    <TextInput label="ID" source="id" />,
    <BooleanInput label="Is Active" source="is_active" />,
];

const StatusField = ({ source }: { source: string }) => {
    const record = useRecordContext();
    if (!record) return null;
    return (
        <Chip
            label={record[source] ? "Active" : "Inactive"}
            color={record[source] ? "success" : "default"}
            size="small"
            variant="outlined"
        />
    );
};

export const TenantList = () => {
    return (
        <List
            actions={<TenantListActions />}
            filters={tenantFilters}
            sort={{ field: 'slug', order: 'ASC' }}
        >
            <Datagrid rowClick="edit">
                <TextField source="id" />
                <TextField source="slug" />
                <StatusField source="is_active" />
            </Datagrid>
        </List>
    );
};
