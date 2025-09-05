// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Identifier, useListContext, useNotify, useRefresh} from "react-admin"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import ElectionHeader from "../../components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {IPermission, IRole} from "@sequentech/ui-core"
import {DataGrid, GridColDef, GridRenderCellParams, GridToolbar} from "@mui/x-data-grid"
import Checkbox from "@mui/material/Checkbox"
import {IPermissions} from "../../types/keycloak"
import {TextField} from "@mui/material"
import {useMutation} from "@apollo/client"
import {DELETE_ROLE_PERMISSION} from "@/queries/DeleteRolePermission"
import {DeleteRolePermissionMutation, SetRolePermissionMutation} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {SET_ROLE_PERMISSION} from "@/queries/SetRolePermission"

type EnumObject = {[key: string]: number | string}
type EnumObjectEnum<E extends EnumObject> = E extends {[key: string]: infer ET | string}
    ? ET
    : never

export function getEnumValues<E extends EnumObject>(enumObject: E): EnumObjectEnum<E>[] {
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
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const [deleteRolePermission] = useMutation<DeleteRolePermissionMutation>(DELETE_ROLE_PERMISSION)
    const [setRolePermission] = useMutation<SetRolePermissionMutation>(SET_ROLE_PERMISSION)
    const notify = useNotify()
    const refresh = useRefresh()

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

    const editRolePermission = (props: GridRenderCellParams<any, boolean>) => async () => {
        const permission = (permissions || []).find((el) => el.id === props.row.id)
        console.log(permission)

        if (!permission?.name || !role) {
            return
        }
        console.log(permission.name)

        // remove/add permission to role
        const {errors} = await (props.value ? deleteRolePermission : setRolePermission)({
            variables: {
                tenantId: tenantId,
                roleId: role.id,
                permissionName: permission.name,
            },
        })
        if (errors) {
            notify(t("usersAndRolesScreen.roles.notifications.permissionEditError"), {
                type: "error",
            })
            console.log(`Error editing permission: ${errors}`)
            return
        }
        notify(t("usersAndRolesScreen.roles.notifications.permissionEditSuccess"), {
            type: "success",
        })
        refresh()
    }

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
                <Checkbox checked={props.value} onClick={editRolePermission(props)} />
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
                disableColumnFilter
                disableColumnSelector
                disableDensitySelector
                slots={{toolbar: GridToolbar}}
                slotProps={{
                    toolbar: {
                        showQuickFilter: true,
                        printOptions: {disableToolbarButton: true},
                        csvOptions: {disableToolbarButton: true},
                    },
                }}
            />
        </PageHeaderStyles.Wrapper>
    )
}
//disableRowSelectionOnClick
