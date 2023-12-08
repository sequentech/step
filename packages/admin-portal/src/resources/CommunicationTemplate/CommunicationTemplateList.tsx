import React, {ReactElement} from "react"

import {Typography} from "@mui/material"
import {DatagridConfigurable, List, TextField, TextInput} from "react-admin"

import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../providers/TenantContextProvider"

const OMIT_FIELDS = ["id"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="ID" source="id" key={1} />,
]

export interface ListTrusteeProps {
    aside?: ReactElement
}

export const CommunicationTemplateList: React.FC<ListTrusteeProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)

    return (
        <>
            <Typography variant="h5">Communication Template</Typography>
            <List
                actions={
                    <ListActions open={openDrawer} setOpen={setOpenDrawer} withFilter={true} />
                }
                sx={{flexGrow: 2}}
                aside={aside}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
