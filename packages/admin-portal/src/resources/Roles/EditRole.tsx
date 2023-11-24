// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Identifier, useListContext} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import ElectionHeader from "../../components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {IPermission, IRole} from "sequent-core"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {IPermissions} from "../../types/keycloak"
import { TextField } from "@mui/material"

type EnumObject = {[key: string]: number | string}
type EnumObjectEnum<E extends EnumObject> = E extends {[key: string]: infer ET | string}
    ? ET
    : never

function getEnumValues<E extends EnumObject>(enumObject: E): EnumObjectEnum<E>[] {
    return Object.keys(enumObject)
        .filter((key) => Number.isNaN(Number(key)))
        .map((key) => enumObject[key] as EnumObjectEnum<E>)
}

interface EditRoleProps {
    id?: Identifier | undefined
    close?: () => void
    permissions?: Array<IPermission>
}

export const EditRole: React.FC<EditRoleProps> = ({id, close, permissions}) => {
    const {data, isLoading} = useListContext()
    const {t} = useTranslation()
    if (isLoading || !data) {
        return null
    }
    let role: IRole | undefined = data?.find((element) => element.id === id)

    let rolePermissions: Array<string> = role?.permissions || []

    let validPermissions = getEnumValues(IPermissions)

    let rows: Array<IPermission & {id: string; active: boolean}> = (permissions || [])
        .filter(
            (permission) => permission.name && validPermissions.includes(permission.name as any)
        )
        .map((permission) => ({
            ...permission,
            id: permission.id || "",
            name: permission.name && t(`usersAndRolesScreen.permissions.${permission.name}`),
            active: (!!permission.name && rolePermissions.includes(permission.name)) || false,
        }))

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Permission",
            width: 350,
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

            <TextField
                label="Name"
                value={role?.name}
                InputProps={{
                    readOnly: true,
                }}
            />
            {role?.name}
            <DataGrid
                rows={rows}
                columns={columns}
                initialState={{
                    pagination: {
                        paginationModel: {
                            pageSize: 10,
                        },
                    },
                }}
                pageSizeOptions={[10, 20, 50, 100]}
                disableRowSelectionOnClick
            />
        </PageHeaderStyles.Wrapper>
    )
}
