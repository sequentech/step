// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"
import {TextField} from "@mui/material"
import {SaveButton, SimpleForm, useNotify, useRefresh} from "react-admin"
import {useTranslation} from "react-i18next"
import {SubmitHandler} from "react-hook-form"
import {IRole} from "@sequentech/ui-core"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useMutation} from "@apollo/client"
import {CREATE_ROLE} from "@/queries/CreateRole"
//import {CreateRoleMutationVariables} from "@/gql/graphql"

interface CreateRoleProps {
    close?: () => void
}

export const CreateRole: React.FC<CreateRoleProps> = ({close}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [role, setRole] = useState<IRole>({
        access: {
            manage: true,
            manageMembers: true,
            manageMembership: true,
            view: true,
            viewMembers: true,
        },
    })
    const [createRole] = useMutation(CREATE_ROLE) //<CreateRoleMutationVariables>(CREATE_ROLE)
    const notify = useNotify()
    const refresh = useRefresh()

    const onSubmit: SubmitHandler<any> = async () => {
        console.log("creating role")
        try {
            let {errors} = await createRole({
                variables: {
                    tenantId,
                    role,
                },
            })
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
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
