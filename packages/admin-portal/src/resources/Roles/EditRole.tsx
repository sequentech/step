// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Edit, Identifier, useListContext} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import ElectionHeader from "../../components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {IPermission, IRole} from "sequent-core"
import {DataGrid, GridColDef, GridRenderCellParams, GridValueGetterParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"

interface EditRoleProps {
    id?: Identifier | undefined
    close?: () => void
}

export const EditRole: React.FC<EditRoleProps> = ({id, close}) => {
    const {data, isLoading} = useListContext()
    const {t} = useTranslation()
    if (isLoading || !data) {
        return null
    }
    let role: IRole | undefined = data?.find((element) => element.id === id)

    let permissions: Array<string> = role?.permissions || []

    let rows: Array<IPermission & {active: boolean}> = permissions.map((permission) => ({
        id: permission,
        name: permission,
        attributes: {},
        active: true,
    }))

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Permission",
            width: 200,
            editable: false,
        },
        {
            field: "active",
            headerName: "Active",
            width: 70,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, boolean>) => (
                <Checkbox checked={props.value} />
            ),
        },
    ]

    return (
        <PageHeaderStyles.Wrapper>
            <ElectionHeader
                title={t("usersAndRolesScreen.roles.edit.title")}
                subtitle="usersAndRolesScreen.roles.edit.subtitle"
            />
            {role?.name}
            <DataGrid
                rows={rows}
                columns={columns}
                initialState={{
                    pagination: {
                        paginationModel: {
                            pageSize: 20,
                        },
                    },
                }}
                pageSizeOptions={[20, 50, 100]}
                disableRowSelectionOnClick
            />
        </PageHeaderStyles.Wrapper>
    )
}
