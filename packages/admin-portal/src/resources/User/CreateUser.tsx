// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { PageHeaderStyles } from "@/components/styles/PageHeaderStyles"
import React, { useState } from "react"
import { TextField } from "@mui/material"
import { SaveButton, SimpleForm, useNotify, useRefresh } from "react-admin"
import { useTranslation } from "react-i18next"
import { SubmitHandler } from "react-hook-form"
import { IUser } from "@sequentech/ui-core"
import { useTenantStore } from "@/providers/TenantContextProvider"
import { CREATE_USER } from "@/queries/CreateUser"
import { useMutation } from "@apollo/client"
import { CreateUserMutationVariables } from "@/gql/graphql"

interface CreateUserProps {
    electionEventId?: string
    close?: () => void
}

export const CreateUser: React.FC<CreateUserProps> = ({ electionEventId, close }) => {
    const { t } = useTranslation()
    const [tenantId] = useTenantStore()
    const [user, setUser] = useState<IUser>({ enabled: true })
    const [createUser] = useMutation<CreateUserMutationVariables>(CREATE_USER)
    const notify = useNotify()
    const refresh = useRefresh()

    const onSubmit: SubmitHandler<any> = async () => {
        console.log("creating user")
        try {
            let { errors } = await createUser({
                variables: {
                    tenantId,
                    electionEventId,
                    user,
                },
            })
            // console.log("data: ", data); /// data.id with new created user id.

            close?.()
            if (errors) {
                notify(t("usersAndRolesScreen.voters.errors.createError"), { type: "error" })
                console.log(`Error creating user: ${errors}`)
            } else {
                notify(t("usersAndRolesScreen.voters.errors.createSuccess"), { type: "success" })
                refresh()
            }
        } catch (error) {
            close?.()
            notify(t("usersAndRolesScreen.voters.errors.createError"), { type: "error" })
            console.log(`Error creating user: ${error}`)
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const { name, value } = e.target
        setUser({ ...user, [name]: value })
    }

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={<SaveButton alwaysEnable />}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <PageHeaderStyles.Title>
                    {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.create.title`)}
                </PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t(
                        `usersAndRolesScreen.${electionEventId ? "voters" : "users"
                        }.create.subtitle`
                    )}
                </PageHeaderStyles.SubTitle>

                <TextField
                    variant="outlined"
                    label={t("usersAndRolesScreen.users.fields.first_name")}
                    value={user.first_name || ""}
                    name={"first_name"}
                    onChange={handleChange}
                />
                <TextField
                    variant="outlined"
                    label={t("usersAndRolesScreen.users.fields.last_name")}
                    value={user.last_name || ""}
                    name={"last_name"}
                    onChange={handleChange}
                />
                <TextField
                    variant="outlined"
                    label={t("usersAndRolesScreen.users.fields.email")}
                    value={user.email || ""}
                    name={"email"}
                    onChange={handleChange}
                />
                <TextField
                    variant="outlined"
                    label={t("usersAndRolesScreen.users.fields.username")}
                    value={user.username || ""}
                    name={"username"}
                    onChange={handleChange}
                />
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}
