// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {NoItem} from "@/components/NoItem"
import {Box} from "@mui/material"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {IMiruSignature} from "@/types/miru"

interface MiruSignaturesProps {
    signatures: IMiruSignature[]
}

export const MiruSignatures: React.FC<MiruSignaturesProps> = (props) => {
    const {signatures} = props
    const {t} = useTranslation() //translations to be applied

    const columns: GridColDef[] = [
        {
            field: "trustee_name",
            headerName: "Name",
            flex: 1,
            editable: false,
            align: "left",
        },
        {
            field: "pub_key",
            headerName: "Address",
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "signature",
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
            {signatures.length ? (
                <DataGrid
                    getRowId={(r) => r.trustee_name}
                    rows={signatures}
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
