// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {Checkbox, TextField} from "@mui/material"
import {SaveButton, SimpleForm, useNotify, useRefresh} from "react-admin"
import {useTranslation} from "react-i18next"
import {SubmitHandler} from "react-hook-form"
import {IPermission, IRole} from "@sequentech/ui-core"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useMutation} from "@apollo/client"
import {CREATE_ROLE} from "@/queries/CreateRole"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {IPermissions} from "@/types/keycloak"
import {getEnumValues} from "./EditRole"
//import {CreateRoleMutationVariables} from "@/gql/graphql"

interface CreateRoleProps {
    close?: () => void
    permissions?: Array<IPermission>
}

export const CreateRole: React.FC<CreateRoleProps> = ({close, permissions}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [rolePermissions, setRolePermissions] = useState<string[]>([])
    const [role, setRole] = useState<IRole>({
        access: {
            manage: true,
            manageMembers: true,
            manageMembership: true,
            view: true,
            viewMembers: true,
        },
        permissions: [],
    })
    const [createRole] = useMutation(CREATE_ROLE) //<CreateRoleMutationVariables>(CREATE_ROLE)
    const notify = useNotify()
    const refresh = useRefresh()
    let validPermissions = getEnumValues(IPermissions)

    const onSubmit: SubmitHandler<any> = async () => {
        console.log("creating role")
        try {
            let {errors, data} = await createRole({
                variables: {
                    tenantId,
                    role: {
                        ...role,
                        permissions: rolePermissions,
                    },
                },
            })
            console.log("data: ", data)

            if (errors) {
                notify(t("usersAndRolesScreen.roles.errors.createError"), {type: "error"})
                console.log(`Error creating user: ${errors}`)
            } else {
                notify(t("usersAndRolesScreen.roles.errors.createSuccess"), {type: "success"})
                refresh()
            }
            close?.()
        } catch (error) {
            notify(t("usersAndRolesScreen.voters.roles.createError"), {type: "error"})
            console.log(`Error creating role: ${error}`)
            close?.()
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setRole({...role, [name]: value})
    }

    let rows: Array<IPermission & {id: string; active: boolean; displayName?: string}> =
        useMemo(() => {
            return (permissions || [])
                .filter(
                    (permission) =>
                        permission.name && validPermissions.includes(permission.name as any)
                )
                .map((permission) => {
                    return {
                        ...permission,
                        id: permission.id || "",
                        name: permission.name,
                        active:
                            (!!permission.name && rolePermissions.includes(permission.name)) ||
                            false,
                        displayName:
                            permission.name &&
                            t(`usersAndRolesScreen.permissions.${permission.name}`),
                    }
                })
        }, [rolePermissions])

    const editRolePermission = (props: GridRenderCellParams<any, boolean>) => async () => {
        const exsitPermmistion = rolePermissions.find((p) => p === props.row.name)
        if (exsitPermmistion) {
            const filteredPermissions = rolePermissions.filter((p) => p !== props.row.name)
            setRolePermissions(filteredPermissions)
        } else {
            setRolePermissions((prev) => [...prev, props.row.name])
        }
    }

    const columns: GridColDef[] = [
        {
            field: "displayName",
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
            <SimpleForm
                toolbar={<SaveButton alwaysEnable />}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <PageHeaderStyles.Title>
                    {t("usersAndRolesScreen.roles.create.title")}
                </PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t("usersAndRolesScreen.roles.create.subtitle")}
                </PageHeaderStyles.SubTitle>

                <TextField
                    variant="outlined"
                    label={t("usersAndRolesScreen.roles.fields.name")}
                    value={role.name || ""}
                    name={"name"}
                    onChange={handleChange}
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
                />
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
