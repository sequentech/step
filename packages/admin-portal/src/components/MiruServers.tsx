// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {Box} from "@mui/material"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {IMiruCcsServer} from "@/types/miru"

interface MiruServersProps {
    servers: IMiruCcsServer[]
}

export const MiruServers: React.FC<MiruServersProps> = (props) => {
    const {servers} = props
    const {t} = useTranslation() //translations to be applied

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Name",
            flex: 1,
            editable: false,
            align: "left",
        },
        {
            field: "address",
            headerName: "Address",
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "public_key_pem",
            headerName: "Public Key",
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
            align: "right",
            headerAlign: "right",
        },
    ]

    return (
        <Box sx={{width: "100%"}}>
            {servers.length ? (
                <DataGrid
                    getRowId={(r) => r.address}
                    rows={servers}
                    columns={columns}
                    initialState={{
                        pagination: {
                            paginationModel: {
                                pageSize: 20,
                            },
                        },
                        sorting: {
                            sortModel: [{field: "name", sort: "asc"}],
                        },
                    }}
                    pageSizeOptions={[10, 20, 50, 100]}
                    disableRowSelectionOnClick
                />
            ) : (
                <NoItem />
            )}
        </Box>
    )
}
